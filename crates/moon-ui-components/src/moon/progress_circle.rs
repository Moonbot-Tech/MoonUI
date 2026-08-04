use gpui::*;

use crate::{Sizable, Size, progress::ProgressCircle};

use super::{
    theme::MoonTheme,
    tokens::{MoonTone, rgba_from},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoonProgressCircleSize {
    Small,
    Normal,
    Large,
}

#[derive(IntoElement)]
pub struct MoonProgressCircle {
    inner: ProgressCircle,
    tone: MoonTone,
    color: Option<u32>,
    size: MoonProgressCircleSize,
}

impl MoonProgressCircle {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            inner: ProgressCircle::new(id),
            tone: MoonTone::Info,
            color: None,
            size: MoonProgressCircleSize::Normal,
        }
    }

    pub fn value(mut self, value: f32) -> Self {
        self.inner = self.inner.value(value);
        self
    }

    /// Show the indeterminate rotating arc instead of `value`.
    ///
    /// **Cost:** this is an endlessly repeating animation, and every animation frame re-renders
    /// the whole hosting window, not just the circle — see [`super::MoonProgress::loading`] for
    /// the measured numbers. Use it for a genuinely pending operation, not as decoration.
    pub fn loading(mut self, loading: bool) -> Self {
        self.inner = self.inner.loading(loading);
        self
    }

    pub fn tone(mut self, tone: MoonTone) -> Self {
        self.tone = tone;
        self
    }

    pub fn color(mut self, color: u32) -> Self {
        self.color = Some(color);
        self
    }

    pub fn size(mut self, size: MoonProgressCircleSize) -> Self {
        self.size = size;
        self
    }

    pub fn render(self) -> impl IntoElement {
        self
    }
}

impl RenderOnce for MoonProgressCircle {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = MoonTheme::active_tokens(cx).palette;
        let color = self.color.unwrap_or_else(|| self.tone.color(p));
        self.inner = self.inner.color(rgba_from(color, 1.0));
        self.inner = self.inner.with_size(match self.size {
            MoonProgressCircleSize::Small => Size::Small,
            MoonProgressCircleSize::Normal => Size::Large,
            MoonProgressCircleSize::Large => Size::Size(px(40.0)),
        });
        self.inner.render(window, cx)
    }
}
