//! Regression coverage for dock panel contracts and structural moves.

use std::rc::Rc;

use gpui::{AppContext as _, Bounds, IntoElement as _, div, point, px, size};

use super::{DOCK_TILE_MIN_H, DockArea, DockItem, DockRoot, MoonDockPanel, PanelView, TileMeta};
use crate::moon::MoonBackgroundPolicy;

/// Build a minimal panel for dock-structure tests.
fn panel(name: &'static str) -> Rc<dyn PanelView> {
    Rc::new(MoonDockPanel::new(name, name, |_, _| {
        div().into_any_element()
    }))
}

/// Empty render host that supplies a real GPUI window to panel-contract tests.
struct DockTestHarness;

impl gpui::Render for DockTestHarness {
    /// Render the empty host used only to access a test window.
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        div()
    }
}

/// Catches disconnecting any `dock.rs:MoonDockPanel` builder from its `PanelView` method, which
/// would make the dock ignore the caller's panel controls or omit its tab suffix.
#[gpui::test]
fn moon_dock_panel_builder_flags_are_observable(cx: &mut gpui::TestAppContext) {
    let window = cx.add_window(|_, _| DockTestHarness);
    cx.update_window(window.into(), |_, window, cx| {
        let bare = MoonDockPanel::new("orders", "Orders", |_, _| div().into_any_element());
        assert!(PanelView::title_suffix(&bare, window, cx).is_none());

        let panel = MoonDockPanel::new("orders", "Orders", |_, _| div().into_any_element())
            .background_policy(MoonBackgroundPolicy::NoFill)
            .closable(false)
            .zoomable(false)
            .detachable(true)
            .show_dock_header(true)
            .visible(false)
            .tab_suffix(|_, _| div().into_any_element());

        assert_eq!(
            PanelView::background_policy(&panel, cx),
            MoonBackgroundPolicy::NoFill
        );
        assert!(!PanelView::closable(&panel, cx));
        assert!(!PanelView::zoomable(&panel, cx));
        assert!(PanelView::detachable(&panel, cx));
        assert!(PanelView::show_dock_header(&panel, cx));
        assert!(!PanelView::visible(&panel, cx));
        assert!(PanelView::title_suffix(&panel, window, cx).is_some());
    })
    .unwrap();
}

/// Catches leaving `dock.rs:DockItem::with_panel_added` as a single panel, which would discard
/// either the existing or newly opened panel instead of exposing both as tabs.
#[test]
fn dock_item_add_panel_creates_tabs_and_activates_new_panel() {
    let first = panel("first");
    let second = panel("second");

    let item = DockItem::Panel(first.clone()).with_panel_added(second.clone());

    let DockItem::Tabs { items, active_ix } = item else {
        panic!("expected adding a panel to a panel to create a tab set");
    };
    assert_eq!(active_ix, 1);
    assert_eq!(items.len(), 2);
    assert!(Rc::ptr_eq(&items[0], &first));
    assert!(Rc::ptr_eq(&items[1], &second));
}

/// Catches removing an edge clamp in `dock.rs:DockArea::clamp_tile_meta`, which would leave
/// detached tiles partly off-screen or too short to interact with.
#[test]
fn dock_clamps_tile_meta_inside_root_bounds() {
    let bounds = Bounds::new(point(px(0.0), px(0.0)), size(px(300.0), px(200.0)));
    let clamped = DockArea::clamp_tile_meta(
        TileMeta {
            x: -50.0,
            y: 500.0,
            w: 900.0,
            h: 20.0,
            z_index: 7,
        },
        bounds,
    );

    assert_eq!(clamped.x, 0.0);
    assert_eq!(clamped.y, 104.0);
    assert_eq!(clamped.w, 300.0);
    assert_eq!(clamped.h, DOCK_TILE_MIN_H);
    assert_eq!(clamped.z_index, 7);
}

/// Catches resolving the target before removal in `dock.rs:DockArea::move_panel_to_tabs`, which
/// would insert a moved tab into a stale path after its source strip collapses.
#[gpui::test]
fn move_panel_to_tabs_resolves_target_after_take(cx: &mut gpui::TestAppContext) {
    cx.update(|cx| {
        let first = panel("first");
        let moved = panel("moved");
        let target_a = panel("target_a");
        let target_b = panel("target_b");
        let mut dock = DockArea::test_with_center(DockItem::Split {
            horizontal: true,
            sizes: Vec::new(),
            items: vec![
                DockItem::Tabs {
                    items: vec![first.clone(), moved.clone()],
                    active_ix: 1,
                },
                DockItem::Tabs {
                    items: vec![target_a.clone(), target_b.clone()],
                    active_ix: 0,
                },
            ],
        });

        assert!(dock.move_panel_to_tabs("moved", DockRoot::Center, &[1], 1, cx));

        let DockItem::Split { items, .. } = &dock.center else {
            panic!("expected root split to survive tab move");
        };
        assert_eq!(items.len(), 2);

        let DockItem::Panel(panel) = &items[0] else {
            panic!("source tab strip should collapse to its remaining panel");
        };
        assert_eq!(panel.panel_name(cx).as_ref(), "first");

        let DockItem::Tabs { items, active_ix } = &items[1] else {
            panic!("target tab strip should stay the target");
        };
        let names = items
            .iter()
            .map(|panel| panel.panel_name(cx).to_string())
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["target_a", "moved", "target_b"]);
        assert_eq!(*active_ix, 1);
    });
}

/// Catches taking the source before the self-drop guard in
/// `dock.rs:DockArea::move_panel_to_tabs`, which would remove a panel when it is dropped onto
/// itself.
#[gpui::test]
fn move_panel_to_tabs_ignores_self_drop_before_take(cx: &mut gpui::TestAppContext) {
    cx.update(|cx| {
        let only = panel("only");
        let mut dock = DockArea::test_with_center(DockItem::Panel(only.clone()));

        assert!(!dock.move_panel_to_tabs("only", DockRoot::Center, &[], 0, cx));

        let DockItem::Panel(panel) = &dock.center else {
            panic!("self-drop must leave the original panel in place");
        };
        assert_eq!(panel.panel_name(cx).as_ref(), "only");
    });
}
