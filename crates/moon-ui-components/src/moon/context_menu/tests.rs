//! Regression coverage for MoonContextMenu viewport clamping.

use super::{
    MoonContextMenuOverlay, MoonContextMenuWindowExt as _, context_menu_clamped_origin,
    context_menu_max_height,
};
use crate::moon::dropdown::{MoonMenuItem, take_menu_item_clone_probe_count};
use crate::moon::{MoonPalette, MoonScale, MoonTheme};
use crate::{Root, WindowExt as _};
use gpui::{
    AppContext as _, Context, IntoElement, Modifiers, Render, Styled as _, Window, point, px,
};
use std::{cell::Cell, rc::Rc};

/// Empty application view hosted by [`Root`] for fitted WindowExt geometry and dismissal probes.
struct ContextMenuHarness;

impl Render for ContextMenuHarness {
    /// Render an inert surface underneath the Root-owned overlay.
    ///
    /// Args:
    ///     _window: Test window hosting the shared Root.
    ///     _cx: View context unused by the inert surface.
    ///
    /// Returns:
    ///     A full-size background element that can receive outside clicks.
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        gpui::div().size_full()
    }
}

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
        context_menu_clamped_origin(320.0, 240.0, -40.0, -20.0, 140.0, 86.0),
        (6.0, 6.0)
    );

    assert_eq!(
        context_menu_clamped_origin(320.0, 240.0, 500.0, 500.0, 140.0, 162.0),
        (174.0, 72.0)
    );
}

/// Catches ignoring the requested limit in `context_menu.rs:context_menu_max_height`, which would
/// let height-limited menus open past the viewport's bottom edge.
#[test]
fn context_menu_requested_max_height_limits_vertical_clamp() {
    assert_eq!(context_menu_max_height(240.0, Some(80.0)), 80.0);
    assert_eq!(
        context_menu_clamped_origin(320.0, 240.0, 200.0, 500.0, 140.0, 80.0),
        (174.0, 154.0)
    );
}

/// Open a fitted menu through the public WindowExt route and return its painted bounds.
fn open_fitted_and_measure(
    cx: &mut gpui::TestAppContext,
    id: &'static str,
    labels: Vec<String>,
) -> (gpui::Bounds<gpui::Pixels>, gpui::Size<gpui::Pixels>) {
    let (_root, visual) = cx.add_window_view(|window, cx| {
        let view = cx.new(|_| ContextMenuHarness);
        Root::new(view, window, cx).bordered(false)
    });
    visual.update(move |window, cx| {
        let viewport = window.viewport_size();
        window.open_fitted_moon_context_menu(
            cx,
            id,
            point(viewport.width - px(2.0), viewport.height - px(2.0)),
            labels.into_iter().map(MoonMenuItem::new).collect(),
            120.0,
            560.0,
        );
    });
    let selector = Box::leak(format!("{id}:menu:popup").into_boxed_str());
    let bounds = (0..8)
        .find_map(|_| {
            visual.update(|window, _| window.refresh());
            visual.run_until_parked();
            visual.debug_bounds(selector)
        })
        .expect("fitted Root-owned context menu must expose its popup bounds");
    let viewport = visual.update(|window, _| window.viewport_size());
    (bounds, viewport)
}

/// Catches collapsing the fitted context-menu route back to its minimum width or leaving its
/// height uncapped. Either edit recreates the supplied screenshots: long localized rows clip, or
/// the settings-style row stack extends beyond the bottom of a scaled viewport.
#[gpui::test]
fn fitted_root_context_menu_grows_and_stays_inside_scaled_viewports(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init);
    for (palette, scale) in [
        (
            MoonPalette::TERMINAL,
            MoonScale {
                ui: 0.9,
                font: 1.35,
                font_delta: 2.0,
            },
        ),
        (
            MoonPalette::LIGHT,
            MoonScale {
                ui: 1.35,
                font: 0.9,
                font_delta: 2.0,
            },
        ),
    ] {
        cx.update(|cx| {
            let theme = MoonTheme::global_mut(cx);
            theme.palette = palette;
            theme.scale = scale;
        });
        let (short, _) = open_fitted_and_measure(cx, "fit-short", vec!["Open".to_string()]);
        let (long, viewport) = open_fitted_and_measure(
            cx,
            "fit-long",
            (0..48)
                .map(|index| {
                    format!(
                        "Add to selected cores blacklist ({index}) with a deliberately long strategy name"
                    )
                })
                .collect(),
        );

        assert!(
            long.size.width > short.size.width,
            "fitted menu did not grow for long content in {palette:?} at {scale:?}"
        );
        assert!(
            long.origin.x >= px(6.0) && long.right() <= viewport.width - px(6.0),
            "fitted menu escaped the horizontal viewport in {palette:?} at {scale:?}: {long:?}"
        );
        assert!(
            long.origin.y >= px(6.0) && long.bottom() <= viewport.height - px(6.0),
            "fitted menu escaped the vertical viewport in {palette:?} at {scale:?}: {long:?}"
        );
    }
}

/// Catches bypassing the fitted custom-dismiss route or invoking its callback twice. Escape and
/// an outside click must each dismiss exactly one Root-owned overlay, or menus remain visible or
/// duplicate caller cleanup after the width API change.
#[gpui::test]
fn fitted_root_context_menu_dismisses_once_per_escape_or_outside_click(
    cx: &mut gpui::TestAppContext,
) {
    cx.update(crate::init);
    for escape in [true, false] {
        let dismissals = Rc::new(Cell::new(0));
        let sink = dismissals.clone();
        let (_root, visual) = cx.add_window_view(|window, cx| {
            let view = cx.new(|_| ContextMenuHarness);
            Root::new(view, window, cx).bordered(false)
        });
        visual.update(move |window, cx| {
            window.open_fitted_moon_context_menu_with_dismiss(
                cx,
                "fit-dismiss",
                point(px(120.0), px(120.0)),
                vec![MoonMenuItem::new("Open")],
                120.0..=320.0,
                move |window, cx| {
                    sink.set(sink.get() + 1);
                    window.close_context_menu(cx);
                },
            );
        });
        for _ in 0..4 {
            visual.update(|window, _| window.refresh());
            visual.run_until_parked();
        }
        if escape {
            visual.simulate_keystrokes("escape");
        } else {
            visual.simulate_click(point(px(2.0), px(2.0)), Modifiers::default());
        }
        visual.run_until_parked();
        assert_eq!(
            dismissals.get(),
            1,
            "fitted context menu dismissal must dispatch exactly once"
        );
    }
}
