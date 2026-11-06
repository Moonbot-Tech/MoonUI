//! Regression coverage for dock panel contracts and structural moves.

use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
};

use gpui::{
    AppContext as _, Bounds, Context, Entity, EventEmitter, IntoElement as _, Modifiers,
    ParentElement as _, SharedString, Styled as _, VisualTestContext, WeakEntity, Window, div,
    point, px, size,
};

use super::{
    DOCK_TILE_MIN_H, DockArea, DockEvent, DockItem, DockNamedLayout, DockPanelControlTooltips,
    DockRoot, DockSplitPlacement, DockTopologyByName, DockTopologyNode, DockTopologySide,
    MoonDockPanel, MoonTabPanelRuntimeState, Panel, PanelEvent, PanelView, TabPanel, TileMeta,
    close_control_shown, tab_interaction_policy,
    tab_panel::{tab_header_height, zoom_control},
};
use crate::moon::{MoonBackgroundPolicy, MoonThemeTokens};

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

/// Render host that exposes a live dock to pointer-driven activation tests.
struct DockEntityHarness {
    dock: Entity<DockArea>,
}

impl gpui::Render for DockEntityHarness {
    /// Render the configured dock across the full test window.
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        div().size_full().child(self.dock.clone())
    }
}

/// Observable activation and lifecycle state for one test panel.
#[derive(Default)]
struct PanelProbe {
    active: Cell<bool>,
    active_calls: Cell<usize>,
    zoomed: Cell<bool>,
    zoom_calls: Cell<usize>,
    added_calls: Cell<usize>,
    removed_calls: Cell<usize>,
    removed_while_zoomed: Cell<bool>,
}

/// Minimal entity-backed panel that records dock lifecycle transitions.
struct TrackingPanel {
    name: &'static str,
    probe: Rc<PanelProbe>,
}

impl EventEmitter<PanelEvent> for TrackingPanel {}

impl gpui::Render for TrackingPanel {
    /// Render an empty surface because these tests exercise dock state rather than pixels.
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl gpui::IntoElement {
        div()
    }
}

impl Panel for TrackingPanel {
    /// Return the stable registry name used by named dock APIs.
    fn panel_name(&self) -> &'static str {
        self.name
    }

    /// Record the active edge announced by the dock.
    fn set_active(&mut self, active: bool, _window: &mut Window, _cx: &mut Context<Self>) {
        self.probe.active.set(active);
        self.probe
            .active_calls
            .set(self.probe.active_calls.get() + 1);
    }

    /// Record the zoom edge announced by live layout replacement.
    fn set_zoomed(&mut self, zoomed: bool, _window: &mut Window, _cx: &mut Context<Self>) {
        self.probe.zoomed.set(zoomed);
        self.probe.zoom_calls.set(self.probe.zoom_calls.get() + 1);
    }

    /// Record the initial dock ownership callback.
    fn on_added_to(
        &mut self,
        _dock_area: WeakEntity<DockArea>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        self.probe.added_calls.set(self.probe.added_calls.get() + 1);
    }

    /// Record destructive removal callbacks that live layout transforms must avoid.
    fn on_removed(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        self.probe.removed_while_zoomed.set(self.probe.zoomed.get());
        self.probe
            .removed_calls
            .set(self.probe.removed_calls.get() + 1);
    }
}

/// Create an entity-backed panel view with externally readable lifecycle state.
fn tracking_panel(
    name: &'static str,
    probe: Rc<PanelProbe>,
    cx: &mut gpui::App,
) -> Rc<dyn PanelView> {
    Rc::new(cx.new(|_| TrackingPanel { name, probe }))
}

/// Root view that reconstructs one standalone `TabPanel` on every render.
struct StandaloneTabHarness {
    first: Rc<dyn PanelView>,
    second: Rc<dyn PanelView>,
}

impl gpui::Render for StandaloneTabHarness {
    /// Render the standalone tab group without a dock-owned topology.
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl gpui::IntoElement {
        TabPanel::new(
            "standalone-tabs",
            vec![self.first.clone(), self.second.clone()],
        )
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

/// Catches applying `dock.rs:TabPanel::active_index` on every standalone re-render, which would
/// jump a user-clicked standalone tab back to the builder's default first item.
#[gpui::test]
fn standalone_tab_panel_keeps_user_selection_across_renders(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init);
    let first_probe = Rc::new(PanelProbe::default());
    let second_probe = Rc::new(PanelProbe::default());
    let (first, second) = cx.update(|cx| {
        (
            tracking_panel("first", first_probe.clone(), cx),
            tracking_panel("second", second_probe.clone(), cx),
        )
    });
    let window = cx.add_window(move |_, _| StandaloneTabHarness { first, second });
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();

    let second_tab = cx
        .debug_bounds("standalone-tabs:tab-host:1")
        .expect("the second standalone tab must expose rendered bounds");
    assert!(first_probe.active.get());
    assert!(!second_probe.active.get());

    cx.simulate_click(second_tab.center(), Modifiers::default());
    cx.run_until_parked();
    assert!(!first_probe.active.get());
    assert!(second_probe.active.get());
    let calls_after_click = (
        first_probe.active_calls.get(),
        second_probe.active_calls.get(),
    );

    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
    assert!(!first_probe.active.get());
    assert!(second_probe.active.get());
    assert_eq!(
        (
            first_probe.active_calls.get(),
            second_probe.active_calls.get(),
        ),
        calls_after_click,
        "an unrelated render must not reset or reannounce the standalone selection"
    );
}

/// Catches removing `DockEvent::PanelActivated` from `dock.rs:TabPanel`'s tab click, which would
/// leave the host's persisted Auto tab unchanged after the operator selects another surface.
#[gpui::test]
fn dock_tab_click_emits_the_exact_activated_panel_name(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init);
    let activations = Rc::new(RefCell::new(Vec::new()));
    let dock_slot = Rc::new(RefCell::new(None));
    let window = cx.add_window({
        let activations = activations.clone();
        let dock_slot = dock_slot.clone();
        move |window, cx| {
            let dock = cx.new(|cx| DockArea::new("click-dock", None, window, cx));
            dock.update(cx, |dock, cx| {
                dock.set_center(
                    DockItem::Tabs {
                        items: vec![panel("report"), panel("chart")],
                        active_ix: 0,
                    },
                    window,
                    cx,
                );
            });
            cx.subscribe(&dock, move |_, _, event: &DockEvent, _| {
                if let DockEvent::PanelActivated { panel_name } = event {
                    activations.borrow_mut().push(panel_name.to_string());
                }
            })
            .detach();
            *dock_slot.borrow_mut() = Some(dock.clone());
            DockEntityHarness { dock }
        }
    });
    let dock = dock_slot
        .borrow()
        .clone()
        .expect("the test window must construct its dock");
    let mut visual = VisualTestContext::from_window(window.into(), cx);
    visual.update(|window, _| window.refresh());
    visual.run_until_parked();

    let chart_tab = visual
        .debug_bounds("click-dock:center:tab-host:1")
        .expect("the second dock tab must expose rendered bounds");
    visual.simulate_click(chart_tab.center(), Modifiers::default());
    visual.run_until_parked();

    assert_eq!(activations.borrow().as_slice(), ["chart"]);
    visual.update(|_, cx| {
        let DockItem::Tabs { active_ix, .. } = &dock.read(cx).center else {
            panic!("the click test must retain its tab group");
        };
        assert_eq!(*active_ix, 1);
    });
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

/// Catches rebuilding `dock.rs:DockArea::apply_named_layout` through panel factories, which would
/// reset local filters or duplicate lifecycle hooks when restoring Classic.
#[gpui::test]
fn named_layout_reuses_instances_and_drops_auto_only_panels(cx: &mut gpui::TestAppContext) {
    let window = cx.add_window(|_, _| DockTestHarness);
    let layout_events = Rc::new(Cell::new(0));
    let (dock, snapshot, report, orders, auto_only, report_probe, orders_probe, auto_probe) = cx
        .update_window(window.into(), {
            let layout_events = layout_events.clone();
            move |_, window, cx| {
                let report_probe = Rc::new(PanelProbe::default());
                let orders_probe = Rc::new(PanelProbe::default());
                let auto_probe = Rc::new(PanelProbe::default());
                let report = tracking_panel("report", report_probe.clone(), cx);
                let orders = tracking_panel("orders", orders_probe.clone(), cx);
                let auto_only = tracking_panel("auto-only", auto_probe.clone(), cx);
                let dock = cx.new(|cx| DockArea::new("snapshot-dock", None, window, cx));

                dock.update(cx, |dock, cx| {
                    dock.set_center(
                        DockItem::Tabs {
                            items: vec![orders.clone(), report.clone()],
                            active_ix: 0,
                        },
                        window,
                        cx,
                    );
                    dock.toggle_zoom_panel("report", window, cx);
                });
                let snapshot = dock.read(cx).named_layout(cx);
                cx.subscribe(&dock, move |_, event: &DockEvent, _| {
                    if matches!(event, DockEvent::LayoutChanged) {
                        layout_events.set(layout_events.get() + 1);
                    }
                })
                .detach();
                (
                    dock,
                    snapshot,
                    report,
                    orders,
                    auto_only,
                    report_probe,
                    orders_probe,
                    auto_probe,
                )
            }
        })
        .unwrap();
    cx.run_until_parked();
    layout_events.set(0);

    cx.update_window(window.into(), |_, window, cx| {
        dock.update(cx, |dock, cx| {
            let topology = DockTopologyByName::tab_preset(["report", "orders"]);
            assert!(dock.apply_topology_by_name(&topology, vec![auto_only.clone()], window, cx,));
        });
    })
    .unwrap();
    cx.run_until_parked();
    assert_eq!(layout_events.get(), 1);
    assert!(report_probe.zoomed.get());
    assert_eq!(auto_probe.added_calls.get(), 1);

    cx.update_window(window.into(), |_, window, cx| {
        dock.update(cx, |dock, cx| {
            assert!(dock.apply_named_layout(&snapshot, Vec::new(), window, cx));
        });
    })
    .unwrap();
    cx.run_until_parked();
    assert_eq!(layout_events.get(), 2);

    cx.update_window(window.into(), |_, _window, cx| {
        let restored_report = dock.read(cx).find_panel_named("report", cx).unwrap();
        let restored_orders = dock.read(cx).find_panel_named("orders", cx).unwrap();
        assert!(Rc::ptr_eq(&restored_report, &report));
        assert!(Rc::ptr_eq(&restored_orders, &orders));
        assert_eq!(report_probe.added_calls.get(), 1);
        assert_eq!(orders_probe.added_calls.get(), 1);
        assert_eq!(report_probe.removed_calls.get(), 0);
        assert_eq!(orders_probe.removed_calls.get(), 0);
        assert_eq!(auto_probe.removed_calls.get(), 1);
        assert!(dock.read(cx).find_panel_named("auto-only", cx).is_none());
        assert_eq!(dock.read(cx).zoomed_panel.as_deref(), Some("report"));
        assert!(report_probe.zoomed.get());
        assert_eq!(report_probe.zoom_calls.get(), 1);
        assert!(!orders_probe.zoomed.get());
        assert_eq!(orders_probe.zoom_calls.get(), 0);
    })
    .unwrap();
}

/// Catches changing `dock.rs:DockArea::take_panel_by_name` to stop at one root, return a rebuilt
/// panel, or repeat lifecycle callbacks for duplicate occurrences; any of those edits would leak
/// a Classic-only panel into Auto or discard its retained local state.
#[gpui::test]
fn take_panel_removes_all_roots_and_restores_the_canonical_rc_once(cx: &mut gpui::TestAppContext) {
    let window = cx.add_window(|_, _| DockTestHarness);
    let layout_events = Rc::new(Cell::new(0));
    let (dock, snapshot, canonical, canonical_probe, duplicate_probe, bottom_probe) = cx
        .update_window(window.into(), {
            let layout_events = layout_events.clone();
            move |_, window, cx| {
                let canonical_probe = Rc::new(PanelProbe::default());
                let duplicate_probe = Rc::new(PanelProbe::default());
                let bottom_probe = Rc::new(PanelProbe::default());
                let canonical = tracking_panel("news", canonical_probe.clone(), cx);
                let duplicate = tracking_panel("news", duplicate_probe.clone(), cx);
                let bottom_duplicate = tracking_panel("news", bottom_probe.clone(), cx);
                canonical.set_active(true, window, cx);
                duplicate.set_active(true, window, cx);
                bottom_duplicate.set_active(true, window, cx);
                canonical.set_zoomed(true, window, cx);

                let dock = cx.new(|_| {
                    let mut dock = DockArea::test_with_center(DockItem::Tabs {
                        items: vec![canonical.clone(), canonical.clone(), panel("center-other")],
                        active_ix: 0,
                    });
                    dock.left = Some((DockItem::Panel(duplicate.clone()), 160.0, true));
                    dock.right = Some((
                        DockItem::Tabs {
                            items: vec![duplicate, panel("right-other")],
                            active_ix: 0,
                        },
                        170.0,
                        true,
                    ));
                    dock.bottom = Some((DockItem::Panel(bottom_duplicate), 180.0, true));
                    dock.zoomed_panel = Some("news".into());
                    dock
                });
                let snapshot = dock.read(cx).named_layout(cx);
                cx.subscribe(&dock, move |_, event: &DockEvent, _| {
                    if matches!(event, DockEvent::LayoutChanged) {
                        layout_events.set(layout_events.get() + 1);
                    }
                })
                .detach();
                (
                    dock,
                    snapshot,
                    canonical,
                    canonical_probe,
                    duplicate_probe,
                    bottom_probe,
                )
            }
        })
        .unwrap();

    let taken = cx
        .update_window(window.into(), |_, window, cx| {
            dock.update(cx, |dock, cx| {
                dock.take_panel_by_name("news", window, cx)
                    .expect("the canonical News identity must be returned")
            })
        })
        .unwrap();
    cx.run_until_parked();

    assert!(Rc::ptr_eq(&taken, &canonical));
    assert_eq!(layout_events.get(), 1);
    for probe in [&canonical_probe, &duplicate_probe, &bottom_probe] {
        assert!(!probe.active.get());
        assert_eq!(probe.active_calls.get(), 2);
        assert_eq!(probe.removed_calls.get(), 1);
    }
    assert!(!canonical_probe.zoomed.get());
    assert_eq!(canonical_probe.zoom_calls.get(), 2);
    assert!(!canonical_probe.removed_while_zoomed.get());
    cx.update(|cx| {
        assert!(dock.read(cx).find_panel_named("news", cx).is_none());
        assert!(
            !dock
                .read(cx)
                .topology_by_name(cx)
                .panel_names()
                .iter()
                .any(|name| name == "news")
        );
    });

    cx.update_window(window.into(), |_, window, cx| {
        dock.update(cx, |dock, cx| {
            assert!(dock.apply_named_layout(&snapshot, vec![taken.clone()], window, cx));
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update(|cx| {
        let restored = dock
            .read(cx)
            .find_panel_named("news", cx)
            .expect("the saved layout must restore News");
        assert!(Rc::ptr_eq(&restored, &canonical));
        assert_eq!(canonical_probe.added_calls.get(), 1);
        assert_eq!(canonical_probe.removed_calls.get(), 1);
        assert_eq!(duplicate_probe.added_calls.get(), 0);
        assert_eq!(bottom_probe.added_calls.get(), 0);
    });
}

/// Catches deduplicating lifecycle callbacks or stopping at one root in
/// `dock.rs:DockArea::remove_panel_by_name`, which would leave matching dock occurrences or their
/// existing per-occurrence host cleanup behind after a destructive removal.
#[gpui::test]
fn remove_panel_clears_all_roots_and_preserves_occurrence_lifecycle(cx: &mut gpui::TestAppContext) {
    let window = cx.add_window(|_, _| DockTestHarness);
    let layout_events = Rc::new(Cell::new(0));
    let (dock, canonical_probe, duplicate_probe, bottom_probe) = cx
        .update_window(window.into(), {
            let layout_events = layout_events.clone();
            move |_, window, cx| {
                let canonical_probe = Rc::new(PanelProbe::default());
                let duplicate_probe = Rc::new(PanelProbe::default());
                let bottom_probe = Rc::new(PanelProbe::default());
                let canonical = tracking_panel("news", canonical_probe.clone(), cx);
                let duplicate = tracking_panel("news", duplicate_probe.clone(), cx);
                let bottom_duplicate = tracking_panel("news", bottom_probe.clone(), cx);
                canonical.set_active(true, window, cx);
                duplicate.set_active(true, window, cx);
                bottom_duplicate.set_active(true, window, cx);
                canonical.set_zoomed(true, window, cx);

                let dock = cx.new(|_| {
                    let mut dock = DockArea::test_with_center(DockItem::Tabs {
                        items: vec![canonical.clone(), canonical, panel("center-other")],
                        active_ix: 0,
                    });
                    dock.left = Some((DockItem::Panel(duplicate.clone()), 160.0, true));
                    dock.right = Some((
                        DockItem::Tabs {
                            items: vec![duplicate, panel("right-other")],
                            active_ix: 0,
                        },
                        170.0,
                        true,
                    ));
                    dock.bottom = Some((DockItem::Panel(bottom_duplicate), 180.0, true));
                    dock.zoomed_panel = Some("news".into());
                    dock
                });
                cx.subscribe(&dock, move |_, event: &DockEvent, _| {
                    if matches!(event, DockEvent::LayoutChanged) {
                        layout_events.set(layout_events.get() + 1);
                    }
                })
                .detach();
                (dock, canonical_probe, duplicate_probe, bottom_probe)
            }
        })
        .unwrap();

    cx.update_window(window.into(), |_, window, cx| {
        dock.update(cx, |dock, cx| {
            assert!(dock.remove_panel_by_name("news", window, cx));
        });
    })
    .unwrap();
    cx.run_until_parked();

    assert_eq!(layout_events.get(), 1);
    assert_eq!(canonical_probe.removed_calls.get(), 2);
    assert_eq!(duplicate_probe.removed_calls.get(), 2);
    assert_eq!(bottom_probe.removed_calls.get(), 1);
    for probe in [&canonical_probe, &duplicate_probe, &bottom_probe] {
        assert!(probe.active.get());
        assert_eq!(probe.active_calls.get(), 1);
    }
    assert!(canonical_probe.zoomed.get());
    assert_eq!(canonical_probe.zoom_calls.get(), 1);
    assert!(canonical_probe.removed_while_zoomed.get());
    cx.update(|cx| {
        assert!(dock.read(cx).zoomed_panel.is_none());
        assert!(dock.read(cx).find_panel_named("news", cx).is_none());
        assert!(
            !dock
                .read(cx)
                .topology_by_name(cx)
                .panel_names()
                .iter()
                .any(|name| name == "news")
        );
    });
}

/// Catches moving `dock.rs:install_resolved_layout`'s zoom reset below `Panel::on_removed`, which
/// would let a panel tear down while it still believes it owns the full-dock zoom surface.
#[gpui::test]
fn layout_replacement_clears_zoom_before_removing_the_panel(cx: &mut gpui::TestAppContext) {
    let window = cx.add_window(|_, _| DockTestHarness);
    cx.update_window(window.into(), |_, window, cx| {
        let chart_probe = Rc::new(PanelProbe::default());
        let chart = tracking_panel("chart", chart_probe.clone(), cx);
        let report = panel("report");
        let dock = cx.new(|cx| DockArea::new("zoom-removal-dock", None, window, cx));
        dock.update(cx, |dock, cx| {
            dock.set_center(DockItem::Panel(chart), window, cx);
            dock.toggle_zoom_panel("chart", window, cx);
            assert!(chart_probe.zoomed.get());

            let layout = DockNamedLayout {
                topology: DockTopologyByName::tab_preset(["report"]),
                active_tabs: HashMap::new(),
                zoomed_panel: None,
            };
            assert!(dock.apply_named_layout(&layout, vec![report], window, cx));
        });

        assert_eq!(chart_probe.removed_calls.get(), 1);
        assert!(
            !chart_probe.removed_while_zoomed.get(),
            "the outgoing zoom edge must close before the destructive removal callback"
        );
    })
    .unwrap();
}

/// Closing a side root must stop its selected panel without waiting for another hidden render.
#[gpui::test]
fn side_root_open_state_drives_activity_and_preserves_zoom(cx: &mut gpui::TestAppContext) {
    let window = cx.add_window(|_, _| DockTestHarness);
    cx.update_window(window.into(), |_, window, cx| {
        let assets = Rc::new(PanelProbe::default());
        let log = Rc::new(PanelProbe::default());
        let chart = Rc::new(PanelProbe::default());
        let items = vec![
            tracking_panel("assets", assets.clone(), cx),
            tracking_panel("log", log.clone(), cx),
        ];
        let runtime = cx.new(|_| MoonTabPanelRuntimeState::default());
        let center = tracking_panel("chart", chart.clone(), cx);
        let dock = cx.new(|_| DockArea::test_with_center(DockItem::Panel(center)));
        dock.update(cx, |dock, cx| {
            dock.right = Some((
                DockItem::Tabs {
                    items,
                    active_ix: 1,
                },
                240.,
                true,
            ));
            dock.tab_runtime_states
                .insert("test-dock:right".into(), runtime.downgrade());
            dock.sync_layout_active(window, cx);
            assert!(!assets.active.get());
            assert!(log.active.get());

            assert!(dock.set_dock_open(super::DockPlacement::Right, false, window, cx));
            assert!(
                !log.active.get(),
                "a closed side root cannot keep its panel active"
            );
            assert_eq!(runtime.read(cx).notified_active, Some((None, 2)));
            let calls = log.active_calls.get();
            assert!(!dock.set_dock_open(super::DockPlacement::Right, false, window, cx));
            assert_eq!(
                log.active_calls.get(),
                calls,
                "unchanged state has no lifecycle edge"
            );

            assert!(dock.toggle_dock(super::DockPlacement::Right, window, cx));
            assert!(log.active.get());
            assert!(!assets.active.get());
            assert_eq!(runtime.read(cx).active_ix, 1);

            dock.toggle_zoom_panel("chart", window, cx);
            assert!(!log.active.get());
            let chart_calls = chart.active_calls.get();
            assert!(dock.set_dock_open(super::DockPlacement::Right, false, window, cx));
            assert!(dock.toggle_dock(super::DockPlacement::Right, window, cx));
            assert_eq!(
                chart.active_calls.get(),
                chart_calls,
                "unchanged zoom has no activity edges"
            );
            assert!(
                !log.active.get(),
                "reopening must not activate behind another panel's zoom"
            );
            dock.clear_zoom(window, cx);
            assert!(log.active.get());
            assert!(!assets.active.get());
        });
    })
    .unwrap();
}

/// Catches making `dock.rs:DockArea::sync_layout_active` ignore `zoomed_panel`, which would keep
/// the hidden normal tab active while the visible zoom surface reports no active panel.
#[gpui::test]
fn zoomed_layout_replacement_activates_only_the_visible_zoom_surface(
    cx: &mut gpui::TestAppContext,
) {
    let window = cx.add_window(|_, _| DockTestHarness);
    cx.update_window(window.into(), |_, window, cx| {
        let orders_probe = Rc::new(PanelProbe::default());
        let chart_probe = Rc::new(PanelProbe::default());
        let orders = tracking_panel("orders", orders_probe.clone(), cx);
        let chart = tracking_panel("chart", chart_probe.clone(), cx);
        let center_state = cx.new(|_| MoonTabPanelRuntimeState {
            active_ix: 1,
            notified_active: Some((Some(SharedString::from("stale")), 9)),
        });
        let zoom_state = cx.new(|_| MoonTabPanelRuntimeState {
            active_ix: 0,
            notified_active: Some((None, 0)),
        });
        let dock = cx.new(|_| {
            DockArea::test_with_center(DockItem::Tabs {
                items: vec![orders.clone(), chart.clone()],
                active_ix: 0,
            })
        });
        orders.set_active(true, window, cx);
        chart.set_active(true, window, cx);

        dock.update(cx, |dock, cx| {
            dock.tab_runtime_states
                .insert("test-dock:center".to_string(), center_state.downgrade());
            dock.tab_runtime_states
                .insert("test-dock:zoom".to_string(), zoom_state.downgrade());
            dock.toggle_zoom_panel("chart", window, cx);
            let layout = dock.named_layout(cx);
            dock.clear_zoom(window, cx);
            assert!(dock.apply_named_layout(&layout, Vec::new(), window, cx));
        });

        assert!(!orders_probe.active.get());
        assert!(chart_probe.active.get());
        assert!(chart_probe.zoomed.get());
        assert_eq!(center_state.read(cx).active_ix, 0);
        assert_eq!(
            center_state.read(cx).notified_active,
            Some((None, 2)),
            "the hidden normal tab runtime must retain its index but announce no visible panel"
        );
        assert_eq!(zoom_state.read(cx).active_ix, 0);
        assert_eq!(
            zoom_state.read(cx).notified_active,
            Some((Some(SharedString::from("chart")), 1))
        );
    })
    .unwrap();
}

/// Catches leaving `dock.rs:DockArea::activate_panel_by_name` underneath another panel's zoom,
/// which would update a hidden tab while the operator still sees the old full-dock surface.
#[gpui::test]
fn named_activation_clears_foreign_zoom_with_one_layout_event(cx: &mut gpui::TestAppContext) {
    let window = cx.add_window(|_, _| DockTestHarness);
    let layout_events = Rc::new(Cell::new(0));
    let (dock, report_probe, chart_probe, center_state, zoom_state) = cx
        .update_window(window.into(), {
            let layout_events = layout_events.clone();
            move |_, window, cx| {
                let report_probe = Rc::new(PanelProbe::default());
                let chart_probe = Rc::new(PanelProbe::default());
                let report = tracking_panel("report", report_probe.clone(), cx);
                let chart = tracking_panel("chart", chart_probe.clone(), cx);
                let center_state = cx.new(|_| MoonTabPanelRuntimeState::default());
                let zoom_state = cx.new(|_| MoonTabPanelRuntimeState::default());
                let dock = cx.new(|_| {
                    DockArea::test_with_center(DockItem::Tabs {
                        items: vec![report, chart],
                        active_ix: 0,
                    })
                });
                dock.update(cx, |dock, cx| {
                    dock.tab_runtime_states
                        .insert("test-dock:center".to_string(), center_state.downgrade());
                    dock.tab_runtime_states
                        .insert("test-dock:zoom".to_string(), zoom_state.downgrade());
                    dock.toggle_zoom_panel("report", window, cx);
                });
                cx.subscribe(&dock, move |_, event: &DockEvent, _| {
                    if matches!(event, DockEvent::LayoutChanged) {
                        layout_events.set(layout_events.get() + 1);
                    }
                })
                .detach();
                (dock, report_probe, chart_probe, center_state, zoom_state)
            }
        })
        .unwrap();
    cx.run_until_parked();
    layout_events.set(0);

    cx.update_window(window.into(), |_, window, cx| {
        dock.update(cx, |dock, cx| {
            assert!(dock.activate_panel_by_name("chart", window, cx));
        });
    })
    .unwrap();
    cx.run_until_parked();

    assert_eq!(layout_events.get(), 1);
    cx.update(|cx| {
        assert!(dock.read(cx).zoomed_panel.is_none());
        assert!(!report_probe.zoomed.get());
        assert!(!report_probe.active.get());
        assert!(chart_probe.active.get());
        assert_eq!(center_state.read(cx).active_ix, 1);
        assert_eq!(
            center_state.read(cx).notified_active,
            Some((Some(SharedString::from("chart")), 2))
        );
        assert_eq!(zoom_state.read(cx).notified_active, Some((None, 1)));
    });
}

/// Catches adding active, zoom, or panel payload fields to `dock.rs:DockTopologyByName`, which
/// would leak one group's runtime state into every Auto workspace window after persistence.
#[test]
fn topology_serialization_is_normalized_and_runtime_state_free() {
    let noisy = DockTopologyByName {
        center: DockTopologyNode::Tabs {
            names: vec!["chart".into(), "chart".into(), "report".into()],
        },
        left: Some(DockTopologySide {
            item: DockTopologyNode::Empty,
            size: f32::NAN,
            open: true,
        }),
        right: None,
        bottom: None,
    };
    let expected = DockTopologyByName::tab_preset(["chart", "report"]);

    assert_eq!(noisy, expected);
    assert_eq!(noisy.panel_names(), vec!["chart", "report"]);
    let json = serde_json::to_string(&noisy.normalized()).unwrap();
    assert!(!json.contains("active"));
    assert!(!json.contains("zoom"));
    assert!(!json.contains("payload"));
    assert!(!json.contains("group"));
}

/// Catches dropping unknown, omitted, or duplicate live identities in
/// `dock.rs:DockArea::apply_topology_by_name`, which would make future or detached panels vanish
/// when applying the shared Auto workspace.
#[gpui::test]
fn named_tab_preset_orders_known_panels_and_appends_unknown_once(cx: &mut gpui::TestAppContext) {
    let window = cx.add_window(|_, _| DockTestHarness);
    cx.update_window(window.into(), |_, window, cx| {
        let chart = panel("chart");
        let future = panel("future");
        let report = panel("report");
        let detached = panel("detached");
        let dock = cx.new(|_| {
            DockArea::test_with_center(DockItem::Split {
                horizontal: false,
                items: vec![
                    DockItem::Panel(chart.clone()),
                    DockItem::Tabs {
                        items: vec![future.clone(), report.clone(), chart.clone()],
                        active_ix: 0,
                    },
                ],
                sizes: vec![None, Some(180.0)],
            })
        });

        dock.update(cx, |dock, cx| {
            let topology = DockTopologyByName {
                center: DockTopologyNode::Tabs {
                    names: vec![
                        "report".into(),
                        "missing".into(),
                        "report".into(),
                        "chart".into(),
                    ],
                },
                left: None,
                right: None,
                bottom: None,
            };
            assert!(dock.apply_topology_by_name(&topology, vec![detached.clone()], window, cx,));
        });

        let DockItem::Tabs { items, active_ix } = &dock.read(cx).center else {
            panic!("repaired panels should become one tab group");
        };
        let names = items
            .iter()
            .map(|panel| panel.panel_name(cx).to_string())
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["report", "chart", "future", "detached"]);
        assert_eq!(
            *active_ix, 2,
            "the previously active future panel must remain active"
        );
        assert_eq!(items.len(), 4);
        assert_eq!(
            items
                .iter()
                .filter(|panel| Rc::ptr_eq(panel, &chart))
                .count(),
            1
        );
        assert!(Rc::ptr_eq(&items[0], &report));
        assert!(Rc::ptr_eq(&items[1], &chart));
        assert!(Rc::ptr_eq(&items[2], &future));
        assert!(Rc::ptr_eq(&items[3], &detached));
    })
    .unwrap();
}

/// Replacing stable-name deduplication in `dock.rs:NamedPanelResolver::new` with `Rc::ptr_eq`
/// must fail: two separately created instances of one logical panel would survive topology repair
/// and keep hidden work alive under duplicate tabs.
#[gpui::test]
fn topology_repair_removes_a_second_identity_with_the_same_stable_name(
    cx: &mut gpui::TestAppContext,
) {
    let window = cx.add_window(|_, _| DockTestHarness);
    let first_probe = Rc::new(PanelProbe::default());
    let duplicate_probe = Rc::new(PanelProbe::default());
    cx.update_window(window.into(), |_, window, cx| {
        let first = tracking_panel("report", first_probe.clone(), cx);
        let duplicate = tracking_panel("report", duplicate_probe.clone(), cx);
        let dock = cx.new(|_| {
            DockArea::test_with_center(DockItem::Tabs {
                items: vec![first.clone(), duplicate],
                active_ix: 0,
            })
        });

        dock.update(cx, |dock, cx| {
            assert!(dock.apply_topology_by_name(
                &DockTopologyByName::tab_preset(["report"]),
                Vec::new(),
                window,
                cx,
            ));
        });

        let DockItem::Panel(remaining) = &dock.read(cx).center else {
            panic!("one stable report identity must remain after repair");
        };
        assert!(Rc::ptr_eq(remaining, &first));
        assert_eq!(duplicate_probe.removed_calls.get(), 1);
        assert_eq!(first_probe.removed_calls.get(), 0);
    })
    .unwrap();
}

/// Catches removing the normalized equality guard in `dock.rs:apply_topology_by_name`, which would
/// rebroadcast a remote Auto revision forever even though the visible topology is already equal.
#[gpui::test]
fn equal_topology_does_not_mutate_or_emit_layout_event(cx: &mut gpui::TestAppContext) {
    let window = cx.add_window(|_, _| DockTestHarness);
    let layout_events = Rc::new(Cell::new(0));
    let (dock, topology) = cx
        .update_window(window.into(), {
            let layout_events = layout_events.clone();
            move |_, _window, cx| {
                let dock = cx.new(|_| {
                    DockArea::test_with_center(DockItem::Tabs {
                        items: vec![panel("chart"), panel("report")],
                        active_ix: 1,
                    })
                });
                let topology = dock.read(cx).topology_by_name(cx);
                cx.subscribe(&dock, move |_, event: &DockEvent, _| {
                    if matches!(event, DockEvent::LayoutChanged) {
                        layout_events.set(layout_events.get() + 1);
                    }
                })
                .detach();
                (dock, topology)
            }
        })
        .unwrap();

    cx.update_window(window.into(), |_, window, cx| {
        dock.update(cx, |dock, cx| {
            assert!(!dock.apply_topology_by_name(&topology, Vec::new(), window, cx));
        });
    })
    .unwrap();
    cx.run_until_parked();
    assert_eq!(layout_events.get(), 0);
}

/// Catches removing tree, keyed runtime, side-dock, or `Panel::set_active` synchronization from
/// `dock.rs:DockArea::activate_panel_by_name`, which would leave an opened chart tab hidden or
/// stale after a trade navigation request.
#[gpui::test]
fn activating_named_panel_updates_tree_runtime_and_panel_state(cx: &mut gpui::TestAppContext) {
    let window = cx.add_window(|_, _| DockTestHarness);
    cx.update_window(window.into(), |_, window, cx| {
        let report_probe = Rc::new(PanelProbe::default());
        let chart_probe = Rc::new(PanelProbe::default());
        let sibling_probe = Rc::new(PanelProbe::default());
        let report = tracking_panel("report", report_probe.clone(), cx);
        let chart = tracking_panel("chart", chart_probe.clone(), cx);
        let sibling = tracking_panel("sibling", sibling_probe.clone(), cx);
        let dock = cx.new(|_| {
            let mut dock = DockArea::test_with_center(DockItem::Empty);
            dock.bottom = Some((
                DockItem::Split {
                    horizontal: true,
                    items: vec![
                        DockItem::Tabs {
                            items: vec![report, chart],
                            active_ix: 0,
                        },
                        DockItem::Panel(sibling),
                    ],
                    sizes: vec![None, Some(180.0)],
                },
                180.0,
                false,
            ));
            dock
        });
        let state = cx.new(|_| MoonTabPanelRuntimeState::default());
        dock.update(cx, |dock, _| {
            dock.tab_runtime_states
                .insert("test-dock:bottom:split:0".to_string(), state.downgrade());
        });

        dock.update(cx, |dock, cx| {
            assert!(dock.activate_panel_by_name("chart", window, cx));
        });

        {
            let dock_ref = dock.read(cx);
            let Some((DockItem::Split { items, .. }, _, open)) = &dock_ref.bottom else {
                panic!("expected the bottom split to survive activation");
            };
            assert!(*open);
            let DockItem::Tabs { active_ix, .. } = &items[0] else {
                panic!("expected the target tab group inside the bottom split");
            };
            assert_eq!(*active_ix, 1);
        }

        assert_eq!(state.read(cx).active_ix, 1);
        assert_eq!(
            state.read(cx).notified_active,
            Some((Some(SharedString::from("chart")), 2))
        );
        assert!(!report_probe.active.get());
        assert!(chart_probe.active.get());
        assert_eq!(report_probe.active_calls.get(), 1);
        assert_eq!(chart_probe.active_calls.get(), 1);
        assert!(sibling_probe.active.get());
        assert_eq!(sibling_probe.active_calls.get(), 1);
    })
    .unwrap();
}

/// Catches omitting or reordering `DockEvent::PanelActivated` in named activation or user move
/// wrappers, which would persist the wrong Auto tab before the accompanying layout update after a
/// programmatic reveal, reorder, cross-group drop, or split transfer.
#[gpui::test]
fn activation_paths_emit_the_exact_stable_panel_name(cx: &mut gpui::TestAppContext) {
    let window = cx.add_window(|_, _| DockTestHarness);
    let event_order = Rc::new(RefCell::new(Vec::new()));
    let dock = cx
        .update_window(window.into(), {
            let event_order = event_order.clone();
            move |_, _window, cx| {
                let dock = cx.new(|_| {
                    DockArea::test_with_center(DockItem::Split {
                        horizontal: true,
                        items: vec![
                            DockItem::Tabs {
                                items: vec![
                                    panel("programmatic"),
                                    panel("reordered"),
                                    panel("transferred"),
                                    panel("split"),
                                ],
                                active_ix: 0,
                            },
                            DockItem::Tabs {
                                items: vec![panel("target-a"), panel("target-b")],
                                active_ix: 0,
                            },
                        ],
                        sizes: Vec::new(),
                    })
                });
                cx.subscribe(&dock, move |_, event: &DockEvent, _| match event {
                    DockEvent::PanelActivated { panel_name } => {
                        event_order
                            .borrow_mut()
                            .push(format!("activated:{panel_name}"));
                    }
                    DockEvent::LayoutChanged => event_order.borrow_mut().push("layout".to_string()),
                    DockEvent::DetachRequested { .. }
                    | DockEvent::PanelCloseRequested { .. }
                    | DockEvent::TabContextMenu { .. } => {}
                })
                .detach();
                dock
            }
        })
        .unwrap();

    cx.update_window(window.into(), |_, window, cx| {
        dock.update(cx, |dock, cx| {
            assert!(dock.activate_panel_by_name("reordered", window, cx));
            assert!(dock.move_tab_before_from_user(DockRoot::Center, &[0], "programmatic", 2, cx,));
            assert!(dock.move_panel_to_tabs_from_user(
                "transferred",
                DockRoot::Center,
                &[1],
                1,
                cx,
            ));
            assert!(dock.move_panel_to_split_from_user(
                "split",
                DockRoot::Center,
                &[1],
                DockSplitPlacement::Right,
                cx,
            ));
        });
    })
    .unwrap();
    cx.run_until_parked();

    assert_eq!(
        event_order.borrow().as_slice(),
        [
            "activated:reordered",
            "layout",
            "activated:programmatic",
            "layout",
            "activated:transferred",
            "layout",
            "activated:split",
            "layout",
        ]
    );
}

/// Catches coupling `dock.rs:DockArea::set_detach_allowed` back to structural editing, which would
/// either allow Auto panels to escape into windows or disable reorder and close at the same time.
#[gpui::test]
fn disabled_detach_keeps_tab_reorder_close_and_activation_enabled(cx: &mut gpui::TestAppContext) {
    let window = cx.add_window(|_, _| DockTestHarness);
    let detach_events = Rc::new(Cell::new(0));
    let close_events = Rc::new(Cell::new(0));
    let dock = cx
        .update_window(window.into(), {
            let detach_events = detach_events.clone();
            let close_events = close_events.clone();
            move |_, _window, cx| {
                let report = panel("report");
                let chart = panel("chart");
                let dock = cx.new(|_| {
                    DockArea::test_with_center(DockItem::Tabs {
                        items: vec![report, chart],
                        active_ix: 0,
                    })
                });
                cx.subscribe(&dock, move |_, event: &DockEvent, _| match event {
                    DockEvent::DetachRequested { .. } => detach_events.set(detach_events.get() + 1),
                    DockEvent::PanelCloseRequested { .. } => {
                        close_events.set(close_events.get() + 1)
                    }
                    DockEvent::LayoutChanged
                    | DockEvent::PanelActivated { .. }
                    | DockEvent::TabContextMenu { .. } => {}
                })
                .detach();
                dock
            }
        })
        .unwrap();
    cx.run_until_parked();

    cx.update_window(window.into(), |_, window, cx| {
        dock.update(cx, |dock, cx| {
            dock.set_detach_allowed(false, cx);
            assert!(dock.move_tab_before_from_user(DockRoot::Center, &[], "chart", 0, cx,));
            dock.request_detach_from_user("report".into(), cx);
            dock.request_close_from_user("report".into(), cx);
            assert!(dock.activate_panel_by_name("chart", window, cx));
        });

        let DockItem::Tabs { items, active_ix } = &dock.read(cx).center else {
            panic!("detach policy must not remove the editable tab strip");
        };
        let names = items
            .iter()
            .map(|panel| panel.panel_name(cx).to_string())
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["chart", "report"]);
        assert_eq!(*active_ix, 0);
    })
    .unwrap();
    cx.run_until_parked();
    assert_eq!(detach_events.get(), 0);
    assert_eq!(close_events.get(), 1);
}

/// Catches coupling `dock.rs:DockArea::set_close_allowed` to structural editing or omitting its
/// request guard, which would either lock Auto's modular dock or expose a close action it refuses.
#[gpui::test]
fn disabled_close_keeps_reorder_split_and_activation_without_emitting_close(
    cx: &mut gpui::TestAppContext,
) {
    let window = cx.add_window(|_, _| DockTestHarness);
    let close_events = Rc::new(Cell::new(0));
    let dock = cx
        .update_window(window.into(), {
            let close_events = close_events.clone();
            move |_, _window, cx| {
                let report = panel("report");
                let chart = panel("chart");
                let dock = cx.new(|_| {
                    DockArea::test_with_center(DockItem::Tabs {
                        items: vec![report, chart],
                        active_ix: 0,
                    })
                });
                cx.subscribe(&dock, move |_, event: &DockEvent, _| {
                    if matches!(event, DockEvent::PanelCloseRequested { .. }) {
                        close_events.set(close_events.get() + 1);
                    }
                })
                .detach();
                dock
            }
        })
        .unwrap();
    cx.run_until_parked();

    cx.update_window(window.into(), |_, window, cx| {
        dock.update(cx, |dock, cx| {
            dock.set_close_allowed(false, cx);
            assert!(dock.move_tab_before_from_user(DockRoot::Center, &[], "chart", 0, cx));
            dock.request_close_from_user("report".into(), cx);
            assert!(dock.activate_panel_by_name("chart", window, cx));
        });
    })
    .unwrap();
    cx.run_until_parked();
    assert_eq!(close_events.get(), 0);

    dock.update(cx, |dock, cx| {
        dock.set_close_allowed(true, cx);
        dock.request_close_from_user("report".into(), cx);
    });
    cx.run_until_parked();
    assert_eq!(close_events.get(), 1);
}

/// Catches removing the pinned guard or insertion clamp from `dock.rs:DockArea` tab moves, which
/// would let ChartTabs drift away from the emphasized leading edge in Auto mode.
#[gpui::test]
fn pinned_leading_panel_stays_first_and_blocks_user_drag(cx: &mut gpui::TestAppContext) {
    let window = cx.add_window(|_, _| DockTestHarness);
    cx.update_window(window.into(), |_, _window, cx| {
        let dock = cx.new(|_| {
            DockArea::test_with_center(DockItem::Tabs {
                items: vec![panel("report"), panel("chart"), panel("orders")],
                active_ix: 0,
            })
        });
        dock.update(cx, |dock, cx| {
            assert!(dock.set_pinned_leading_panels(vec!["chart".into()], cx));
            assert!(!dock.move_tab_before_from_user(DockRoot::Center, &[], "chart", 2, cx,));
            assert!(dock.move_tab_before_from_user(DockRoot::Center, &[], "orders", 0, cx,));
        });

        let DockItem::Tabs { items, active_ix } = &dock.read(cx).center else {
            panic!("pinned role must preserve the tab group");
        };
        let names = items
            .iter()
            .map(|panel| panel.panel_name(cx).to_string())
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["chart", "orders", "report"]);
        assert_eq!(
            *active_ix, 1,
            "the successfully dragged orders tab must become active"
        );
    })
    .unwrap();
}

/// Catches removing `DockArea::move_panel_to_split_from_user`'s pinned-target guard, which would
/// let an operational panel split to the left of Charts and break the strict leading workspace
/// route while the persisted topology still appears valid.
#[gpui::test]
fn pinned_subtree_rejects_leading_split_but_accepts_trailing_split(cx: &mut gpui::TestAppContext) {
    let window = cx.add_window(|_, _| DockTestHarness);
    cx.update_window(window.into(), |_, _window, cx| {
        let chart = panel("chart");
        let report = panel("report");
        let dock = cx.new(|_| {
            let mut dock = DockArea::test_with_center(DockItem::Tabs {
                items: vec![chart, report],
                active_ix: 0,
            });
            dock.pinned_leading_panels = vec!["chart".into()];
            dock
        });
        dock.update(cx, |dock, cx| {
            assert!(!dock.move_panel_to_split_from_user(
                "report",
                DockRoot::Center,
                &[],
                DockSplitPlacement::Left,
                cx,
            ));
            let DockItem::Tabs { items, .. } = &dock.center else {
                panic!("rejected leading split must preserve the original tab group");
            };
            assert_eq!(
                items
                    .iter()
                    .map(|panel| panel.panel_name(cx).to_string())
                    .collect::<Vec<_>>(),
                vec!["chart", "report"]
            );

            assert!(dock.move_panel_to_split_from_user(
                "report",
                DockRoot::Center,
                &[],
                DockSplitPlacement::Right,
                cx,
            ));
            let DockItem::Split {
                horizontal, items, ..
            } = &dock.center
            else {
                panic!("trailing split must remain available");
            };
            assert!(*horizontal);
            assert!(items[0].find_panel_named("chart", cx).is_some());
            assert!(items[1].find_panel_named("report", cx).is_some());
        });
    })
    .unwrap();
}

/// Catches limiting `dock.rs:DockItem::enforce_pinned_leading` to tab sorting; an older persisted
/// topology could then restore an operational split to the left of the pinned Charts subtree.
#[gpui::test]
fn persisted_horizontal_split_is_repaired_before_a_pinned_subtree(cx: &mut gpui::TestAppContext) {
    let window = cx.add_window(|_, _| DockTestHarness);
    cx.update_window(window.into(), |_, window, cx| {
        let dock = cx.new(|_| {
            let mut dock = DockArea::test_with_center(DockItem::Tabs {
                items: vec![panel("chart"), panel("report")],
                active_ix: 0,
            });
            dock.pinned_leading_panels = vec!["chart".into()];
            dock
        });
        let stale = DockTopologyByName {
            center: DockTopologyNode::Split {
                horizontal: true,
                items: vec![
                    DockTopologyNode::Panel {
                        name: "report".into(),
                    },
                    DockTopologyNode::Panel {
                        name: "chart".into(),
                    },
                ],
                sizes: vec![Some(180.0), None],
            },
            left: None,
            right: None,
            bottom: None,
        };

        dock.update(cx, |dock, cx| {
            assert!(dock.apply_topology_by_name(&stale, Vec::new(), window, cx));
        });
        let DockItem::Split { items, sizes, .. } = &dock.read(cx).center else {
            panic!("repaired persisted topology must remain a split");
        };
        assert!(items[0].find_panel_named("chart", cx).is_some());
        assert!(items[1].find_panel_named("report", cx).is_some());
        assert_eq!(sizes, &vec![None, Some(180.0)]);
    })
    .unwrap();
}

/// Catches making `dock.rs:tab_interaction_policy` exclude pinned hosts from drop targets, which
/// would make a lone Charts tab unable to accept operational tabs back into its strip.
#[test]
fn pinned_leading_tab_is_fixed_but_remains_a_drop_target() {
    let policy = tab_interaction_policy(true, true, true);
    assert!(!policy.draggable, "the pinned Charts tab must stay fixed");
    assert!(
        policy.accepts_drop,
        "the pinned Charts host must accept tabs inserted after the pinned prefix"
    );
    assert!(
        !policy.detachable,
        "the pinned Charts tab must stay attached"
    );
}

/// Catches omitting the pinned guard from the header close control, which would leave a
/// visibly dead × on the pinned leading tab — close is already a no-op for that panel.
#[test]
fn pinned_active_panel_hides_the_close_control() {
    assert!(
        close_control_shown(true, true, true, false),
        "an unpinned closable panel must still show the close control"
    );
    assert!(
        !close_control_shown(true, true, true, true),
        "a pinned closable panel must not show a dead close control"
    );
}

/// Catches splitting the zoom glyph and tooltip onto separate conditions, which would label
/// a maximize button "restore" after the panel has already filled the dock.
#[test]
fn zoom_control_pairs_the_glyph_with_its_tooltip() {
    let tooltips = DockPanelControlTooltips {
        zoom_in: Some("Zoom the panel to fill the dock".into()),
        zoom_out: Some("Restore the normal layout".into()),
        ..Default::default()
    };
    let (glyph, tip) = zoom_control(true, &tooltips);
    assert_eq!(
        glyph, "\u{25A1}",
        "zoomed panel must show the restore square"
    );
    assert_eq!(
        tip.as_deref(),
        Some("Restore the normal layout"),
        "zoomed panel must use zoom_out, not zoom_in"
    );
    let (glyph, tip) = zoom_control(false, &tooltips);
    assert_eq!(
        glyph, "\u{25A3}",
        "unzoomed panel must show the maximize square"
    );
    assert_eq!(
        tip.as_deref(),
        Some("Zoom the panel to fill the dock"),
        "unzoomed panel must use zoom_in, not zoom_out"
    );
}

/// Catches leaving split drop zones at the top of a headed slot, which would steal tab drops
/// from the header so a dragged panel cannot land back on the strip.
#[test]
fn split_drop_header_inset_clears_the_tab_header() {
    let tokens = MoonThemeTokens::default();
    assert_eq!(
        tab_header_height(&tokens),
        tokens.fit_height(29.0, 13.0, 8.0)
    );
    assert_eq!(
        DockArea::split_drop_header_inset(true, &tokens),
        tab_header_height(&tokens)
    );
    assert_eq!(DockArea::split_drop_header_inset(false, &tokens), 0.0);
}

// ---- Theme skin: what a panel, a tile and the gap between panels paint ----

use super::{
    TileFacet,
    tab_panel::{PanelFacet, PanelPlacement},
};
use crate::{
    ElementExt as _, Root,
    moon::{MoonFacetKind, MoonPalette, MoonSkin, MoonTheme, MoonThemeConfig, facet, rgba_from},
};
use gpui::{
    AnyWindowHandle, AtlasKey, AtlasTile, Background, BoxShadow, DevicePixels, HeadlessAppContext,
    NoopTextSystem, Pixels, PlatformAtlas, PlatformHeadlessRenderer, Point, Quad, ScaledPixels,
    Scene, Shadow, Size, linear_color_stop, linear_gradient, transparent_black,
};
use std::{borrow::Cow, sync::Arc};

const ALL_POLICIES: [MoonBackgroundPolicy; 3] = [
    MoonBackgroundPolicy::Opaque,
    MoonBackgroundPolicy::Transparent,
    MoonBackgroundPolicy::NoFill,
];

const ALL_PLACEMENTS: [PanelPlacement; 3] = [
    PanelPlacement::Standalone,
    PanelPlacement::Docked,
    PanelPlacement::Floating,
];

/// The highlight a slab carries along its top edge on a dark palette, written out rather than
/// taken from `skin.rs`: one pixel down, no blur, inside the slab.
fn slab_top_highlight(p: MoonPalette) -> BoxShadow {
    BoxShadow {
        color: rgba_from(p.overlay, 0.12),
        offset: point(px(0.0), px(1.0)),
        blur_radius: px(0.0),
        spread_radius: px(0.0),
        inset: true,
    }
}

/// Theme tokens for `palette` drawn with `skin`, at the default scale.
fn skin_tokens(palette: MoonPalette, skin: MoonSkin) -> MoonThemeTokens {
    MoonThemeTokens {
        palette,
        skin,
        ..MoonThemeTokens::default()
    }
}

/// The sheen a faceted header lays over its surface, written out rather than taken from
/// `tab_panel.rs`.
fn header_sheen(p: MoonPalette) -> Background {
    linear_gradient(
        180.0,
        linear_color_stop(rgba_from(p.overlay, 0.05), 0.0),
        linear_color_stop(rgba_from(p.overlay, 0.0), 1.0),
    )
}

/// Catches `tab_panel.rs:PanelFacet::resolve` or `dock.rs:TileFacet::resolve` ignoring the skin:
/// every flat theme would get gradient panels, rounded corners and shadows it never asked for.
#[test]
fn the_flat_skin_resolves_no_panel_or_tile_facet() {
    for palette in [
        MoonPalette::TERMINAL,
        MoonPalette::LIGHT,
        MoonPalette::GRAPHITE,
        MoonPalette::OBSIDIAN,
    ] {
        let tokens = skin_tokens(palette, MoonSkin::Flat);
        for root in ALL_POLICIES {
            for content in ALL_POLICIES {
                for placement in ALL_PLACEMENTS {
                    assert_eq!(
                        PanelFacet::resolve(
                            &tokens,
                            placement,
                            root,
                            MoonBackgroundPolicy::Opaque,
                            content
                        ),
                        None
                    );
                }
            }
            assert_eq!(TileFacet::resolve(&tokens, root), None);
        }
    }
}

/// Catches `tab_panel.rs:PanelFacet::resolve` keying the slab on the root policy alone. A host
/// that leaves the dock unfilled so a chart can show — every panel root is `NoFill` there —
/// would get no slab on its ordinary, opaque panels, which is the whole look of the skin.
///
/// Also catches a docked slab keeping the shadows it casts: the slot beside it may hold a
/// see-through panel, and a shadow cast over that slot would darken the chart it shows.
#[test]
fn a_panel_with_an_opaque_body_is_one_slab_and_casts_a_shadow_only_on_its_own() {
    let p = MoonPalette::OBSIDIAN;
    let tokens = skin_tokens(p, MoonSkin::Facet);
    let slab = facet(MoonFacetKind::Slab, &tokens);

    for (root, content) in [
        (MoonBackgroundPolicy::NoFill, MoonBackgroundPolicy::Opaque),
        (MoonBackgroundPolicy::Opaque, MoonBackgroundPolicy::Opaque),
        (MoonBackgroundPolicy::Opaque, MoonBackgroundPolicy::NoFill),
    ] {
        let alone = PanelFacet::resolve(
            &tokens,
            PanelPlacement::Standalone,
            root,
            MoonBackgroundPolicy::Opaque,
            content,
        )
        .expect("the Facet skin resolves a panel facet");
        assert_eq!(alone.root_fill, Some(slab.fill), "{root:?}/{content:?}");
        assert_eq!(
            alone.root_shadows, slab.shadows,
            "a panel on its own keeps the whole slab"
        );

        let panel = PanelFacet::resolve(
            &tokens,
            PanelPlacement::Docked,
            root,
            MoonBackgroundPolicy::Opaque,
            content,
        )
        .expect("the Facet skin resolves a panel facet");

        assert_eq!(panel.root_fill, Some(slab.fill), "{root:?}/{content:?}");
        assert_eq!(panel.edge, slab.edge);
        assert_eq!(
            panel.root_shadows,
            vec![slab_top_highlight(p)],
            "a docked slab keeps its top highlight and casts nothing outside itself"
        );
        assert_eq!(panel.radius, 8.0, "the container radius of the theme");
        assert_eq!(
            panel.header_fill,
            Some(header_sheen(p)),
            "the header is a sheen over the slab, not the flat `panel` fill"
        );
        assert_eq!(panel.header_overlay, None);
        assert_eq!(
            panel.header_radius, 7.0,
            "the header starts inside the one-pixel edge"
        );
        assert_eq!(panel.header_rule, rgba_from(p.overlay, 0.11));
        assert!(
            !panel.body_keeps_policy_fill,
            "the body paints nothing, so the slab shows through it"
        );
    }
}

/// Catches `tab_panel.rs:PanelFacet::resolve` giving a see-through panel the slab fill or the
/// slab's outer shadows. GPUI paints a shadow under the whole of its element, so either one
/// would cover the GPU chart canvas that such a panel exists to show.
#[test]
fn a_see_through_panel_takes_the_slab_on_its_header_only() {
    let p = MoonPalette::OBSIDIAN;
    let tokens = skin_tokens(p, MoonSkin::Facet);
    let slab = facet(MoonFacetKind::Slab, &tokens);

    for policy in [
        MoonBackgroundPolicy::NoFill,
        MoonBackgroundPolicy::Transparent,
    ] {
        for placement in [PanelPlacement::Docked, PanelPlacement::Standalone] {
            let panel = PanelFacet::resolve(
                &tokens,
                placement,
                policy,
                MoonBackgroundPolicy::Opaque,
                policy,
            )
            .expect("the Facet skin resolves a panel facet");

            assert_eq!(
                panel.root_fill, None,
                "{placement:?} {policy:?} body must stay unpainted"
            );
            assert!(
                panel.body_keeps_policy_fill,
                "the body is left to its policy, which paints nothing"
            );
            assert_eq!(
                panel.root_shadows,
                vec![slab_top_highlight(p)],
                "only the inset highlight: no shadow that falls outside the panel"
            );
            assert_eq!(panel.edge, slab.edge);
            assert_eq!(
                panel.header_fill,
                Some(slab.fill),
                "the header alone carries the slab"
            );
            assert_eq!(panel.header_overlay, Some(header_sheen(p)));
        }

        let bare_header = PanelFacet::resolve(
            &tokens,
            PanelPlacement::Docked,
            policy,
            MoonBackgroundPolicy::NoFill,
            policy,
        )
        .expect("the Facet skin resolves a panel facet");
        assert_eq!(
            (bare_header.header_fill, bare_header.header_overlay),
            (None, None),
            "a header that asked for no fill gets none"
        );
    }
}

/// Catches `dock.rs:TileFacet::resolve` and `tab_panel.rs:PanelFacet::resolve` both painting a
/// surface for a floating tile — a slab inside a popup — or neither, and catches a see-through
/// tile taking the popup's fill or its outer shadows over the canvas behind it.
#[test]
fn a_floating_tile_is_a_popup_and_the_panel_inside_paints_no_surface() {
    let p = MoonPalette::OBSIDIAN;
    let tokens = skin_tokens(p, MoonSkin::Facet);
    let popup = facet(MoonFacetKind::Popup, &tokens);

    let tile = TileFacet::resolve(&tokens, MoonBackgroundPolicy::Opaque)
        .expect("the Facet skin resolves a tile facet");
    assert_eq!(tile.fill, Some(popup.fill));
    assert_eq!(tile.edge, popup.edge);
    assert_eq!(tile.shadows, popup.shadows);
    assert_eq!(tile.radius, 8.0);

    let inside = PanelFacet::resolve(
        &tokens,
        PanelPlacement::Floating,
        MoonBackgroundPolicy::NoFill,
        MoonBackgroundPolicy::Opaque,
        MoonBackgroundPolicy::Opaque,
    )
    .expect("the Facet skin resolves a panel facet");
    assert_eq!(inside.root_fill, None, "the tile carries the fill");
    assert_eq!(
        inside.edge,
        transparent_black(),
        "the tile carries the edge"
    );
    assert_eq!(inside.root_shadows, Vec::new());
    assert_eq!(inside.header_fill, Some(header_sheen(p)));
    assert_eq!(
        inside.header_radius, 6.0,
        "inside the tile's edge and the panel's own unpainted one"
    );
    assert!(!inside.body_keeps_policy_fill);

    for policy in [
        MoonBackgroundPolicy::NoFill,
        MoonBackgroundPolicy::Transparent,
    ] {
        let tile = TileFacet::resolve(&tokens, policy).expect("the Facet skin resolves a tile");
        assert_eq!(tile.fill, None, "{policy:?} tile must stay unpainted");
        assert_eq!(tile.edge, popup.edge);
        assert!(
            !tile.shadows.is_empty() && tile.shadows.iter().all(|shadow| shadow.inset),
            "only the inset highlight: {:?}",
            tile.shadows
        );

        let inside = PanelFacet::resolve(
            &tokens,
            PanelPlacement::Floating,
            MoonBackgroundPolicy::NoFill,
            MoonBackgroundPolicy::Opaque,
            policy,
        )
        .expect("the Facet skin resolves a panel facet");
        assert_eq!(inside.root_fill, None);
        assert_eq!(
            inside.header_fill,
            Some(popup.fill),
            "the header of a see-through tile carries the popup surface"
        );
        assert!(inside.body_keeps_policy_fill);
    }
}

/// The quads and shadows of the last rendered scene.
#[derive(Default)]
struct RecordedScene {
    quads: Vec<Quad>,
    shadows: Vec<Shadow>,
}

impl RecordedScene {
    /// Return the fills painted at `probe`, bottom to top: every quad that covers the point
    /// with a background that is not fully transparent.
    fn fills_at(&self, probe: Point<ScaledPixels>) -> Vec<Background> {
        self.quads
            .iter()
            .filter(|quad| {
                !quad.background.is_transparent()
                    && quad.bounds.contains(&probe)
                    && quad.content_mask.bounds.contains(&probe)
            })
            .map(|quad| quad.background)
            .collect()
    }

    /// Return the shadows that can reach `probe`.
    ///
    /// A shadow outside its element reaches three blur radii past its own rectangle and is
    /// painted under the whole of it. A shadow inside its element covers the element except the
    /// hole, which is that rectangle shrunk by the same margin.
    fn shadows_at(&self, probe: Point<ScaledPixels>) -> Vec<&Shadow> {
        self.shadows
            .iter()
            .filter(|shadow| {
                let reach = 3.0 * shadow.blur_radius.0;
                shadow.content_mask.bounds.contains(&probe)
                    && if shadow.inset == 0 {
                        shadow.bounds.dilate(ScaledPixels(reach)).contains(&probe)
                    } else {
                        shadow.element_bounds.contains(&probe)
                            && !shadow.bounds.dilate(ScaledPixels(-reach)).contains(&probe)
                    }
            })
            .collect()
    }
}

/// Records the primitives a headless window submits, without rasterizing them.
struct SkinSceneRecorder(Rc<RefCell<RecordedScene>>);

impl PlatformHeadlessRenderer for SkinSceneRecorder {
    /// Record the scene; image output is intentionally unsupported.
    fn render_scene_to_image(
        &mut self,
        scene: &Scene,
        size: Size<DevicePixels>,
    ) -> anyhow::Result<image::RgbaImage> {
        self.render_scene(scene, size)?;
        anyhow::bail!("scene recorder does not rasterize images")
    }

    /// Replace the preceding frame, so assertions inspect the final submission only.
    fn render_scene(&mut self, scene: &Scene, _size: Size<DevicePixels>) -> anyhow::Result<()> {
        self.0.replace(RecordedScene {
            quads: scene.quads.clone(),
            shadows: scene.shadows.clone(),
        });
        Ok(())
    }

    /// Fills and shadows do not depend on glyph or icon textures.
    fn sprite_atlas(&self) -> Arc<dyn PlatformAtlas> {
        Arc::new(NoSpriteAtlas)
    }
}

/// Omits sprites from the fill-and-shadow scene probe.
struct NoSpriteAtlas;

impl PlatformAtlas for NoSpriteAtlas {
    /// Decline every sprite: the probe reads quads and shadows only.
    fn get_or_insert_with<'a>(
        &self,
        _key: &AtlasKey,
        _build: &mut dyn FnMut() -> anyhow::Result<Option<(Size<DevicePixels>, Cow<'a, [u8]>)>>,
    ) -> anyhow::Result<Option<AtlasTile>> {
        Ok(None)
    }

    /// Nothing was stored, so there is nothing to remove.
    fn remove(&self, _key: &AtlasKey) {}
}

/// Build a headed panel that reports where its body was painted.
fn probed_panel(
    name: &'static str,
    policy: MoonBackgroundPolicy,
    body: Rc<Cell<Bounds<Pixels>>>,
) -> Rc<dyn PanelView> {
    Rc::new(
        MoonDockPanel::new(name, name, move |_, _| {
            let body = body.clone();
            div()
                .size_full()
                .on_prepaint(move |bounds, _, _| body.set(bounds))
                .into_any_element()
        })
        .background_policy(policy)
        .show_dock_header(true),
    )
}

/// Draw `frames` frames, so prepaint captures settle before the scene is read.
fn pump_frames(cx: &mut HeadlessAppContext, window: AnyWindowHandle, frames: usize) {
    for _ in 0..frames {
        cx.update_window(window, |_, window, cx| {
            window.refresh();
            window.draw(cx).clear();
        })
        .expect("headless frame must draw");
        cx.run_until_parked();
    }
}

/// Catches `tab_panel.rs:TabPanel::render` or `dock.rs:DockArea::render_item` painting over the
/// body of a see-through panel under the Facet skin — a slab fill, a body fill or a shadow —
/// which would hide the GPU chart canvas a chart host shows through it. Also catches the Flat
/// skin losing a fill it has always painted: the panel body, the header or the strip between
/// two panels.
///
/// The dock is set up as a chart host sets it up: the window root, the dock and every panel
/// root are `NoFill`, and only a panel's own policy decides whether its body is painted.
#[test]
fn a_see_through_panel_body_stays_unpainted_and_the_gap_opens_only_under_the_facet_skin() {
    for (name, config) in [
        ("flat", MoonThemeConfig::moon_terminal()),
        ("facet", MoonThemeConfig::moon_obsidian()),
    ] {
        let scene = Rc::new(RefCell::new(RecordedScene::default()));
        let recorded = scene.clone();
        let mut cx = HeadlessAppContext::with_platform(
            Arc::new(NoopTextSystem::new()),
            Arc::new(()),
            move || Some(Box::new(SkinSceneRecorder(recorded.clone()))),
        );
        cx.update(crate::init);
        cx.update(|cx| MoonTheme::install_config(config.clone(), cx));
        let tokens = cx.update(|cx| MoonTheme::active_tokens(cx));
        let p = tokens.palette;
        let faceted = tokens.skin.is_facet();
        assert_eq!(faceted, name == "facet", "{name}: the theme under test");

        let opaque_body = Rc::new(Cell::new(Bounds::default()));
        let chart_body = Rc::new(Cell::new(Bounds::default()));
        let window = cx
            .open_window(size(px(800.0), px(600.0)), {
                let opaque_body = opaque_body.clone();
                let chart_body = chart_body.clone();
                move |window, cx| {
                    let dock = cx.new(|cx| {
                        DockArea::new("skin-dock", None, window, cx)
                            .background_policy(MoonBackgroundPolicy::NoFill)
                            .tab_background_policy(MoonBackgroundPolicy::NoFill)
                    });
                    dock.update(cx, |dock, cx| {
                        dock.set_center(
                            DockItem::Split {
                                horizontal: true,
                                items: vec![
                                    DockItem::Panel(probed_panel(
                                        "orders",
                                        MoonBackgroundPolicy::Opaque,
                                        opaque_body,
                                    )),
                                    DockItem::Panel(probed_panel(
                                        "chart",
                                        MoonBackgroundPolicy::NoFill,
                                        chart_body,
                                    )),
                                ],
                                sizes: vec![Some(300.0), None],
                            },
                            window,
                            cx,
                        );
                    });
                    let view = cx.new(|_| DockEntityHarness { dock });
                    cx.new(|cx| {
                        Root::new(view, window, cx)
                            .bordered(false)
                            .background_policy(MoonBackgroundPolicy::NoFill)
                    })
                }
            })
            .expect("headless window must open");

        pump_frames(&mut cx, window.into(), 4);
        let scale = cx
            .update_window(window.into(), |_, window, _| window.scale_factor())
            .expect("headless window must report its scale");
        // The recorder only fires on a render pass; a plain draw never reaches it.
        let _ = cx.capture_screenshot(window.into());
        let scene = scene.borrow();

        let opaque = opaque_body.get();
        let chart = chart_body.get();
        assert!(
            opaque.size.width > px(100.0) && chart.size.width > px(100.0),
            "{name}: both panel bodies must have been painted: {opaque:?} {chart:?}"
        );
        assert!(
            chart.left() > opaque.right(),
            "{name}: the chart sits right of the gap"
        );
        let header_height = px(tab_header_height(&tokens));
        let chart_center = chart.center().scale(scale);
        let chart_header = point(chart.center().x, chart.top() - header_height / 2.0).scale(scale);
        let opaque_center = opaque.center().scale(scale);
        let gap = point((opaque.right() + chart.left()) / 2.0, opaque.center().y).scale(scale);

        assert_eq!(
            scene.fills_at(chart_center),
            Vec::new(),
            "{name}: nothing may be filled over the body of a see-through panel"
        );
        let over_chart = scene.shadows_at(chart_center);
        assert!(
            over_chart.is_empty(),
            "{name}: no shadow may fall over the body of a see-through panel: {over_chart:?}"
        );

        let slab = facet(MoonFacetKind::Slab, &tokens);
        let flat = |color: u32, alpha: f32| Background::from(rgba_from(color, alpha));
        // Shadows cast by something the size of a panel, as (inside, outside) counts. The glow
        // under an active tab is a shadow too, under either skin, and is not one of these.
        let panel_side = 100.0 * scale;
        let panel_shadows = scene.shadows.iter().filter(|shadow| {
            shadow.element_bounds.size.width.0 > panel_side
                && shadow.element_bounds.size.height.0 > panel_side
        });
        let (inside, outside): (Vec<&Shadow>, Vec<&Shadow>) =
            panel_shadows.partition(|shadow| shadow.inset != 0);
        if faceted {
            assert_eq!(
                (inside.len(), outside.len()),
                (2, 0),
                "{name}: each panel is lit along its top, and a docked panel casts no shadow"
            );
            assert_eq!(
                scene.fills_at(opaque_center),
                vec![slab.fill],
                "{name}: an opaque panel is one slab, and its body adds no fill over it"
            );
            assert_eq!(
                scene.fills_at(chart_header).first(),
                Some(&slab.fill),
                "{name}: the header of a see-through panel carries the slab"
            );
            assert_eq!(
                scene.fills_at(gap),
                Vec::new(),
                "{name}: the gap between two slabs shows what lies behind the dock"
            );
        } else {
            let over_opaque = scene.fills_at(opaque_center);
            assert_eq!(
                over_opaque.last(),
                Some(&flat(p.shell, 1.0)),
                "{name}: the body paints `shell`: {over_opaque:?}"
            );
            assert_eq!(
                scene.fills_at(chart_header),
                vec![flat(p.panel, 1.0)],
                "{name}: the header paints `panel`"
            );
            assert_eq!(
                scene.fills_at(gap).first(),
                Some(&flat(p.shell, 1.0)),
                "{name}: the strip between two panels paints `shell`"
            );
            assert_eq!(
                (inside.len(), outside.len()),
                (0, 0),
                "{name}: a flat panel casts no shadow"
            );
        }
    }
}
