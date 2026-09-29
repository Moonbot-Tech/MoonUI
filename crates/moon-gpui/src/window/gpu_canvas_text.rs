//! Per-canvas retention of gpu canvas text frames.
//!
//! A GPU-only frame prepares text only for the canvases that asked to present;
//! every other canvas replays the text frame it produced last time, as long as
//! nothing its text depends on changed. A cached frame holding polychrome
//! (image/emoji) sprites is never replayed: `Window::drop_image` can free those
//! atlas tiles without dirtying the window, so such a canvas prepares again.

use std::hash::{Hash, Hasher};

use collections::{FxHashMap, FxHashSet};

use crate::{
    Bounds, ContentMask, DrawOrder, GpuCanvasLayer, GpuCanvasTextFrame, Pixels, ScaledPixels,
    TextRenderingMode, WindowBackgroundAppearance,
};

/// One paint of a canvas's text in one text layer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct GpuCanvasTextKey {
    /// Stable identity of the canvas driver for as long as it lives.
    pub canvas: usize,
    pub layer: GpuCanvasLayer,
    /// How many times this canvas and layer were already seen in the frame, so a
    /// handle painted twice keeps separate text.
    pub occurrence: usize,
}

impl Hash for GpuCanvasTextKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.canvas.hash(state);
        matches!(self.layer, GpuCanvasLayer::OverScene).hash(state);
        self.occurrence.hash(state);
    }
}

/// Everything outside the canvas that its prepared text depends on.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GpuCanvasTextEnv {
    pub bounds: Bounds<Pixels>,
    pub content_mask: ContentMask<ScaledPixels>,
    pub scale_factor: f32,
    pub content_zoom: f32,
    pub text_rendering_mode: TextRenderingMode,
    pub subpixel_rendering_supported: bool,
    pub background_appearance: WindowBackgroundAppearance,
    pub order: DrawOrder,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TextAction {
    Prepare,
    Reuse,
}

/// Prepare when the frame is forced, the canvas itself presents, nothing is
/// cached, or the environment the cached text was prepared in changed.
pub(crate) fn text_action(
    force_present: bool,
    wants_present: bool,
    cached_env: Option<&GpuCanvasTextEnv>,
    env: &GpuCanvasTextEnv,
) -> TextAction {
    match cached_env {
        Some(cached) if !force_present && !wants_present && cached == env => TextAction::Reuse,
        _ => TextAction::Prepare,
    }
}

#[derive(Default)]
pub(crate) struct GpuCanvasTextRetention {
    entries: FxHashMap<GpuCanvasTextKey, (GpuCanvasTextEnv, GpuCanvasTextFrame)>,
    prepares: u64,
    reuses: u64,
}

impl GpuCanvasTextRetention {
    pub(crate) fn cached_env(&self, key: &GpuCanvasTextKey) -> Option<&GpuCanvasTextEnv> {
        self.entries.get(key).map(|(env, _)| env)
    }

    /// The cached environment, unless the cached frame must not be replayed
    /// (it holds polychrome sprites whose atlas tiles may have been freed).
    pub(crate) fn reusable_env(&self, key: &GpuCanvasTextKey) -> Option<&GpuCanvasTextEnv> {
        self.entries
            .get(key)
            .filter(|(_, frame)| frame.polychrome_sprites.is_empty())
            .map(|(env, _)| env)
    }

    pub(crate) fn cached_frame(&self, key: &GpuCanvasTextKey) -> Option<GpuCanvasTextFrame> {
        self.entries.get(key).map(|(_, frame)| frame.clone())
    }

    pub(crate) fn store(
        &mut self,
        key: GpuCanvasTextKey,
        env: GpuCanvasTextEnv,
        frame: GpuCanvasTextFrame,
    ) {
        self.prepares += 1;
        self.entries.insert(key, (env, frame));
    }

    /// Drops one canvas's cached text.
    pub(crate) fn evict(&mut self, key: &GpuCanvasTextKey) {
        self.entries.remove(key);
    }

    pub(crate) fn note_reuse(&mut self) {
        self.reuses += 1;
    }

    /// Drops the text of canvases that were not part of this frame.
    pub(crate) fn retain_seen(&mut self, seen: &FxHashSet<GpuCanvasTextKey>) {
        self.entries.retain(|key, _| seen.contains(key));
    }

    /// Forgets every cached frame (device recovery: atlas tiles are stale).
    pub(crate) fn clear(&mut self) {
        self.entries.clear();
    }

    pub(crate) fn prepares(&self) -> u64 {
        self.prepares
    }

    pub(crate) fn reuses(&self) -> u64 {
        self.reuses
    }
}
