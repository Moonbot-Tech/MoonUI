//! Layout and event regressions for contiguous sidebar menu hit rows.

use super::*;
use crate::sidebar::SidebarGroup;
use gpui::{Bounds, Context, Modifiers, Pixels, Render, TestAppContext, VisualTestContext, point};
use std::cell::RefCell;

/// Render the production menu next to the former gap-based layout as a geometry oracle.
struct MenuHarness {
    collapsed: bool,
    grouped: bool,
    submenu: bool,
    disabled: bool,
    clicks: Rc<RefCell<Vec<usize>>>,
}

impl Render for MenuHarness {
    /// Build real sidebar items and an independent snapshot of their previous plate layout.
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let items = (0..3)
            .map(|ix| {
                let clicks = self.clicks.clone();
                let item = SidebarMenuItem::new(format!("Item {ix}"))
                    .icon(IconName::Settings)
                    .active(ix == 0)
                    .disable(self.disabled && ix == 1)
                    .on_click(move |_, _, _| clicks.borrow_mut().push(ix));
                if self.submenu && ix == 0 {
                    item.default_open(true)
                        .click_to_toggle(true)
                        .children((0..2).map(|child| {
                            let clicks = self.clicks.clone();
                            SidebarMenuItem::new(format!("Child {child}"))
                                .on_click(move |_, _, _| clicks.borrow_mut().push(10 + child))
                        }))
                } else {
                    item
                }
            })
            .collect::<Vec<_>>();
        let actual = if self.grouped {
            SidebarGroup::new("Group")
                .children(items)
                .collapsed(self.collapsed)
                .render("menu", window, cx)
                .into_any_element()
        } else {
            SidebarMenu::new()
                .children(items)
                .collapsed(self.collapsed)
                .render("menu", window, cx)
                .into_any_element()
        };
        h_flex()
            .items_start()
            .gap(px(40.0))
            .p(px(20.0))
            .child(
                div()
                    .w(px(if self.collapsed { 32.0 } else { 200.0 }))
                    .child(actual),
            )
            .child(
                v_flex()
                    .w(px(if self.collapsed { 32.0 } else { 200.0 }))
                    .when(self.grouped && !self.collapsed, |this| {
                        this.child(div().h_8())
                    })
                    .child(v_flex().gap_2().children((0..3).map(|ix| {
                        div()
                            .child(
                                h_flex()
                                    .w_full()
                                    .p_2()
                                    .gap_x_2()
                                    .text_sm()
                                    .debug_selector(move || format!("legacy-{ix}"))
                                    .child(Icon::new(IconName::Settings))
                                    .when(!self.collapsed, |this| {
                                        this.h_7().child(format!("Item {ix}"))
                                    }),
                            )
                            .when(self.submenu && ix == 0, |this| {
                                this.child(
                                    v_flex()
                                        .debug_selector(|| "legacy-submenu".to_owned())
                                        .border_l_1()
                                        .gap_1()
                                        .ml_3p5()
                                        .pl_2p5()
                                        .py_0p5()
                                        .children((0..2).map(|child| {
                                            h_flex()
                                                .h_7()
                                                .p_2()
                                                .text_sm()
                                                .debug_selector(move || {
                                                    format!("legacy-child-{child}")
                                                })
                                                .child(format!("Child {child}"))
                                        })),
                                )
                            })
                    }))),
            )
    }
}

/// Return a rendered part's bounds, failing if production stopped emitting that part.
fn bounds(cx: &mut VisualTestContext, selector: &str) -> Bounds<Pixels> {
    let selectors = [
        "menu-0:plate",
        "menu-1:plate",
        "menu-2:plate",
        "menu-0:hit-row",
        "menu-1:hit-row",
        "menu-2:hit-row",
        "legacy-0",
        "legacy-1",
        "legacy-2",
        "menu-0-0:plate",
        "menu-0-1:plate",
        "menu-0-0:hit-row",
        "menu-0-1:hit-row",
        "legacy-child-0",
        "legacy-child-1",
        "menu-0:submenu-line",
        "legacy-submenu",
        "groups-0-0:plate",
        "groups-0-1:plate",
        "groups-1-0:plate",
        "groups-1-1:plate",
        "groups-0-0:hit-row",
        "groups-0-1:hit-row",
        "groups-1-0:hit-row",
        "groups-1-1:hit-row",
    ];
    let key = selectors
        .into_iter()
        .find(|key| *key == selector)
        .expect("known selector");
    cx.debug_bounds(key)
        .unwrap_or_else(|| panic!("missing {selector}"))
}

/// Reintroducing container gaps or moving padding into the plate must fail: old gap clicks
/// select exactly the nearer item, plates keep their original positions and height, including
/// the icon-only layout and groups holding direct menu items.
#[gpui::test]
fn contiguous_hit_rows_preserve_plates_and_click_the_nearer_item(cx: &mut TestAppContext) {
    cx.update(crate::init);
    for collapsed in [false, true] {
        for grouped in [false, true] {
            let clicks = Rc::new(RefCell::new(Vec::new()));
            let sink = clicks.clone();
            let window = cx.add_window(move |_, _| MenuHarness {
                collapsed,
                grouped,
                submenu: false,
                disabled: false,
                clicks: sink,
            });
            let mut visual = VisualTestContext::from_window(window.into(), cx);
            visual.run_until_parked();
            let plates = (0..3)
                .map(|ix| bounds(&mut visual, &format!("menu-{ix}:plate")))
                .collect::<Vec<_>>();
            let rows = (0..3)
                .map(|ix| bounds(&mut visual, &format!("menu-{ix}:hit-row")))
                .collect::<Vec<_>>();
            for ix in 0..3 {
                let legacy = bounds(&mut visual, &format!("legacy-{ix}"));
                assert_eq!(plates[ix].origin.y, legacy.origin.y);
                assert_eq!(plates[ix].size, legacy.size);
                assert_eq!(
                    plates[ix].size.height,
                    px(if collapsed { 30.0 } else { 28.0 })
                );
            }
            assert_eq!(rows[0].top(), plates[0].top());
            assert_eq!(rows[2].bottom(), plates[2].bottom());
            for ix in 0..2 {
                assert_eq!(rows[ix].bottom(), rows[ix + 1].top());
                assert_eq!(plates[ix + 1].top() - plates[ix].bottom(), px(8.0));
                visual.simulate_click(
                    point(rows[ix].center().x, plates[ix].bottom() + px(1.0)),
                    Modifiers::none(),
                );
                visual.simulate_click(
                    point(rows[ix].center().x, plates[ix + 1].top() - px(1.0)),
                    Modifiers::none(),
                );
            }
            assert_eq!(&*clicks.borrow(), &[0, 1, 1, 2]);
        }
    }
}

/// Restoring submenu gaps must fail; bubbling child clicks into the padded parent must also
/// fail because that would close the submenu and select its parent instead of only its child.
#[gpui::test]
fn submenu_gap_clicks_keep_pitch_and_do_not_toggle_the_parent(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let clicks = Rc::new(RefCell::new(Vec::new()));
    let sink = clicks.clone();
    let window = cx.add_window(move |_, _| MenuHarness {
        collapsed: false,
        grouped: false,
        submenu: true,
        disabled: false,
        clicks: sink,
    });
    let mut visual = VisualTestContext::from_window(window.into(), cx);
    visual.run_until_parked();
    let first = bounds(&mut visual, "menu-0-0:plate");
    let second = bounds(&mut visual, "menu-0-1:plate");
    for ix in 0..2 {
        let plate = bounds(&mut visual, &format!("menu-0-{ix}:plate"));
        let legacy = bounds(&mut visual, &format!("legacy-child-{ix}"));
        assert_eq!(plate.top(), legacy.top());
        assert_eq!(plate.size, legacy.size);
    }
    assert_eq!(second.top() - first.top(), px(32.0));
    let line = bounds(&mut visual, "menu-0:submenu-line");
    let legacy_line = bounds(&mut visual, "legacy-submenu");
    assert_eq!(line.top(), legacy_line.top());
    assert_eq!(line.size.height, legacy_line.size.height);
    assert_eq!(line.size.width, px(1.0));
    assert_eq!(
        bounds(&mut visual, "menu-0:hit-row").bottom(),
        bounds(&mut visual, "menu-0-0:hit-row").top()
    );
    assert_eq!(
        bounds(&mut visual, "menu-0-1:hit-row").bottom(),
        bounds(&mut visual, "menu-1:hit-row").top()
    );
    assert_eq!(
        bounds(&mut visual, "menu-0-0:hit-row").bottom(),
        bounds(&mut visual, "menu-0-1:hit-row").top()
    );
    visual.simulate_click(
        point(first.center().x, first.bottom() + px(1.0)),
        Modifiers::none(),
    );
    visual.simulate_click(
        point(second.center().x, second.top() - px(1.0)),
        Modifiers::none(),
    );
    visual.run_until_parked();
    assert_eq!(&*clicks.borrow(), &[10, 11]);
    assert!(visual.debug_bounds("menu-0-1:plate").is_some());
    let next = bounds(&mut visual, "menu-1:plate");
    assert_eq!(next.top(), bounds(&mut visual, "legacy-1").top());
    let parent = bounds(&mut visual, "menu-0:plate");
    visual.simulate_click(parent.center(), Modifiers::none());
    visual.run_until_parked();
    assert!(visual.debug_bounds("menu-0-0:plate").is_none());
    assert_eq!(&*clicks.borrow(), &[10, 11, 0]);
}

/// Growing hit rows must not make disabled items actionable in either half of the old gap.
#[gpui::test]
fn disabled_padding_never_fires_a_handler(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let clicks = Rc::new(RefCell::new(Vec::new()));
    let sink = clicks.clone();
    let window = cx.add_window(move |_, _| MenuHarness {
        collapsed: false,
        grouped: false,
        submenu: false,
        disabled: true,
        clicks: sink,
    });
    let mut visual = VisualTestContext::from_window(window.into(), cx);
    visual.run_until_parked();
    let row = bounds(&mut visual, "menu-1:hit-row");
    for y in [row.top() + px(1.0), row.center().y, row.bottom() - px(1.0)] {
        visual.simulate_click(point(row.center().x, y), Modifiers::none());
    }
    assert!(clicks.borrow().is_empty());
}

/// Two menus in one group exercise the boundary between separately rendered item lists.
struct GroupedMenusHarness {
    rem_size: f32,
    clicks: Rc<RefCell<Vec<usize>>>,
}

impl Render for GroupedMenusHarness {
    /// Render grouped menus at the chosen rem size to catch hardcoded pixel spacing.
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        window.set_rem_size(px(self.rem_size));
        let menus = (0..2).map(|menu| {
            SidebarMenu::new().children((0..2).map(|item| {
                let clicks = self.clicks.clone();
                let index = menu * 2 + item;
                SidebarMenuItem::new(format!("Item {index}"))
                    .on_click(move |_, _, _| clicks.borrow_mut().push(index))
            }))
        });
        div().w(px(200.0)).child(
            SidebarGroup::new("Group")
                .children(menus)
                .render("groups", window, cx),
        )
    }
}

/// Dropping group-child spacing or leaving it outside hit rows breaks this test: the last row
/// of one menu and first row of the next must touch while their plates keep the old 8 px gap.
#[gpui::test]
fn grouped_menus_share_gap_hits_at_multiple_rem_sizes(cx: &mut TestAppContext) {
    cx.update(crate::init);
    for rem_size in [16.0, 24.0] {
        let clicks = Rc::new(RefCell::new(Vec::new()));
        let sink = clicks.clone();
        let window = cx.add_window(move |_, _| GroupedMenusHarness {
            rem_size,
            clicks: sink,
        });
        let mut visual = VisualTestContext::from_window(window.into(), cx);
        visual.run_until_parked();
        let keys = ["groups-0-0", "groups-0-1", "groups-1-0", "groups-1-1"];
        let plates = keys.map(|key| bounds(&mut visual, &format!("{key}:plate")));
        let rows = keys.map(|key| bounds(&mut visual, &format!("{key}:hit-row")));
        for ix in 0..3 {
            assert_eq!(rows[ix].bottom(), rows[ix + 1].top());
            assert_eq!(
                plates[ix + 1].top() - plates[ix].bottom(),
                px(rem_size * 0.5)
            );
            assert_eq!(plates[ix].size.height, px(rem_size * 1.75));
        }
        visual.simulate_click(
            point(rows[1].center().x, plates[1].bottom() + px(1.0)),
            Modifiers::none(),
        );
        visual.simulate_click(
            point(rows[2].center().x, plates[2].top() - px(1.0)),
            Modifiers::none(),
        );
        assert_eq!(&*clicks.borrow(), &[1, 2]);
    }
}
