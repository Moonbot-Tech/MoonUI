use super::*;
use crate::{AtlasTextureId, AtlasTextureKind, AtlasTile, Corners, PolychromeSprite, TileId, px};

fn env() -> GpuCanvasTextEnv {
    GpuCanvasTextEnv {
        bounds: Bounds::default(),
        content_mask: ContentMask::default(),
        scale_factor: 1.0,
        content_zoom: 1.0,
        text_rendering_mode: TextRenderingMode::default(),
        subpixel_rendering_supported: false,
        background_appearance: WindowBackgroundAppearance::default(),
        order: 1,
    }
}

fn key() -> GpuCanvasTextKey {
    GpuCanvasTextKey {
        canvas: 1,
        layer: GpuCanvasLayer::UnderScene,
        occurrence: 0,
    }
}

/// Catches `gpu_canvas_text.rs:text_action` ignoring the environment: after a resize or a
/// zoom a static canvas would replay labels laid out for the old geometry.
#[test]
fn env_change_reprepares() {
    let cached = env();
    let mut moved = env();
    moved.bounds.origin.x = px(10.);
    assert_eq!(
        text_action(false, false, Some(&cached), &cached),
        TextAction::Reuse
    );
    assert_eq!(
        text_action(false, false, Some(&cached), &moved),
        TextAction::Prepare
    );
}

/// Catches `text_action` ignoring `force_present`: a UI redraw would keep stale text of
/// canvases whose own state changed with the view.
#[test]
fn force_present_reprepares_all() {
    let cached = env();
    assert_eq!(
        text_action(true, false, Some(&cached), &cached),
        TextAction::Prepare
    );
}

/// Catches `gpu_canvas_text.rs:reusable_env` replaying a frame with polychrome sprites:
/// `Window::drop_image` may have freed those atlas tiles, so an emoji would draw garbage.
#[test]
fn polychrome_frame_never_reused() {
    let mut retention = GpuCanvasTextRetention::default();
    let mut frame = GpuCanvasTextFrame::default();
    frame.polychrome_sprites.push(PolychromeSprite {
        order: 1,
        pad: 0,
        grayscale: false,
        opacity: 1.0,
        bounds: Bounds::default(),
        content_mask: ContentMask::default(),
        corner_radii: Corners::default(),
        tile: AtlasTile {
            texture_id: AtlasTextureId {
                index: 0,
                kind: AtlasTextureKind::Polychrome,
            },
            tile_id: TileId(0),
            padding: 0,
            bounds: Bounds::default(),
        },
    });
    retention.store(key(), env(), frame);
    assert!(retention.cached_env(&key()).is_some());
    assert_eq!(
        text_action(false, false, retention.reusable_env(&key()), &env()),
        TextAction::Prepare
    );
}
