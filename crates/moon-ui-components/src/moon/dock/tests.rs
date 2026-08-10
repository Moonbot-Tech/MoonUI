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
    DOCK_TILE_MIN_H, DockArea, DockEvent, DockItem, DockNamedLayout, DockRoot, DockSplitPlacement,
    DockTopologyByName, DockTopologyNode, DockTopologySide, MoonDockPanel,
    MoonTabPanelRuntimeState, Panel, PanelEvent, PanelView, TabPanel, TileMeta,
    tab_interaction_policy,
};
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
