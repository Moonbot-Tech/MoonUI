//! Regression coverage for MoonContextMenu viewport clamping.

use super::{MoonContextMenuOverlay, context_menu_clamped_origin, context_menu_max_height};
use crate::moon::dropdown::{MoonMenuItem, take_menu_item_clone_probe_count};

/// Catches changing `MoonContextMenuOverlay::items` back to an owned row vector. That edit makes
/// the root overlay closure deep-clone every dynamic context-menu row on each rebuild.
#[test]
fn retained_context_menu_level_clones_without_cloning_rows() {
    let overlay = MoonContextMenuOverlay::new("shared-context-menu").items(
        (0..1_000).map(|ix| MoonMenuItem::new(format!("moon-menu-clone-probe-context-{ix:04}"))),
    );
    _ = take_menu_item_clone_probe_count();

    let _retained_repaint_level = overlay.items.clone();

    assert_eq!(
        take_menu_item_clone_probe_count(),
        0,
        "cloning retained context-menu storage must not clone its 1,000 rows"
    );
}

/// Catches removing the edge clamps in `context_menu.rs:context_menu_clamped_origin`, which would
/// let menus opened near a viewport edge render partially off-screen.
#[test]
fn context_menu_origin_clamps_to_viewport_edges() {
    assert_eq!(
        context_menu_clamped_origin(320.0, 240.0, -40.0, -20.0, 140.0, 200.0, 3),
        (6.0, 6.0)
    );

    assert_eq!(
        context_menu_clamped_origin(320.0, 240.0, 500.0, 500.0, 140.0, 200.0, 6),
        (174.0, 72.0)
    );
}

/// Catches ignoring the requested limit in `context_menu.rs:context_menu_max_height`, which would
/// let height-limited menus open past the viewport's bottom edge.
#[test]
fn context_menu_requested_max_height_limits_vertical_clamp() {
    assert_eq!(context_menu_max_height(240.0, Some(80.0)), 80.0);
    assert_eq!(
        context_menu_clamped_origin(320.0, 240.0, 200.0, 500.0, 140.0, 80.0, 20),
        (174.0, 154.0)
    );
}
