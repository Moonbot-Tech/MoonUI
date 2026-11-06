use super::Popover;
use gpui::MouseButton;
use gpui::{Anchor, Bounds, Point};

#[test]
fn test_popover_builder_chaining() {
    let popover = Popover::new("test")
        .anchor(Anchor::BottomCenter)
        .mouse_button(MouseButton::Right)
        .default_open(true)
        .appearance(false)
        .overlay_closable(false);

    assert_eq!(popover.anchor, Anchor::BottomCenter);
    assert_eq!(popover.mouse_button, MouseButton::Right);
    assert!(popover.default_open);
    assert!(!popover.appearance);
    assert!(!popover.overlay_closable);
}

#[test]
fn test_resolved_corner_top_positions() {
    use gpui::px;

    let bounds = Bounds {
        origin: Point {
            x: px(100.),
            y: px(100.),
        },
        size: gpui::Size {
            width: px(200.),
            height: px(50.),
        },
    };

    let pos = Popover::resolved_corner(Anchor::TopLeft, bounds);
    assert_eq!(pos.x, px(100.));
    assert_eq!(pos.y, px(100.));

    let pos = Popover::resolved_corner(Anchor::TopCenter, bounds);
    assert_eq!(pos.x, px(200.));
    assert_eq!(pos.y, px(100.));

    let pos = Popover::resolved_corner(Anchor::TopRight, bounds);
    assert_eq!(pos.x, px(300.));
    assert_eq!(pos.y, px(100.));

    let pos = Popover::resolved_corner(Anchor::BottomLeft, bounds);
    assert_eq!(pos.x, px(100.));
    assert_eq!(pos.y, px(50.));

    let pos = Popover::resolved_corner(Anchor::BottomCenter, bounds);
    assert_eq!(pos.x, px(200.));
    assert_eq!(pos.y, px(50.));

    let pos = Popover::resolved_corner(Anchor::BottomRight, bounds);
    assert_eq!(pos.x, px(300.));
    assert_eq!(pos.y, px(50.));
}

use crate::{
    Root, WindowExt as _,
    moon::{MoonDropdown, MoonMenuItem, MoonTheme, ThemeMode},
};
use gpui::{
    AppContext as _, Context, FocusHandle, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, Styled as _, VisualTestContext, Window, div, px,
};
use std::{cell::Cell, rc::Rc};

/// A window-root trigger whose menu opens a real Root dialog with its own focus target.
struct DialogFocusHarness {
    open: Rc<Cell<bool>>,
    origin: FocusHandle,
    field: FocusHandle,
}

impl Render for DialogFocusHarness {
    /// Mount the real dropdown selection handler, not a duplicate selection policy.
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let field = self.field.clone();
        let open = self.open.clone();
        let owner = cx.entity_id();
        div().track_focus(&self.origin).child(
            MoonDropdown::new("dialog-opener")
                .label("Open")
                .open(self.open.get())
                .on_open_change(move |value, _, cx| {
                    open.set(value);
                    cx.notify(owner);
                })
                .item(
                    MoonMenuItem::with_key("modal", "Modal")
                        .closes_menu(true)
                        .on_click(move |_, window, cx| {
                            let body_focus = field.clone();
                            window.open_dialog(cx, move |dialog, _, _| {
                                let body_focus = body_focus.clone();
                                dialog.content(move |body, _, _| {
                                    body.child(
                                        div()
                                            .track_focus(&body_focus)
                                            .w(px(180.))
                                            .h(px(40.))
                                            .child(
                                                div()
                                                    .debug_selector(|| {
                                                        "dialog-inner-trigger".into()
                                                    })
                                                    .w(px(80.))
                                                    .h(px(30.))
                                                    .child(
                                                        MoonDropdown::new("dialog-inner")
                                                            .label("Nested")
                                                            .item(MoonMenuItem::new(
                                                                "Nested action",
                                                            )),
                                                    ),
                                            ),
                                    )
                                })
                            });
                            field.focus(window, cx);
                        }),
                ),
        )
    }
}

/// Closing a menu after its callback opened a dialog must neither steal the field's focus nor
/// leave the dialog's return focus on a hidden menu. Reordering close after the callback reddens
/// the final origin assertion; unconditional deferred restoration reddens the field assertion.
#[gpui::test]
fn dropdown_select_plan_respects_close_and_controlled_state(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init);
    for theme in [ThemeMode::Dark, ThemeMode::Light] {
        cx.update(|cx| MoonTheme::set_mode(theme, cx));
        let open = Rc::new(Cell::new(false));
        let handles = Rc::new(std::cell::RefCell::new(None));
        let window = cx.add_window({
            let open = open.clone();
            let handles = handles.clone();
            move |window, cx| {
                let origin = cx.focus_handle();
                let field = cx.focus_handle();
                *handles.borrow_mut() = Some((origin.clone(), field.clone()));
                let view = cx.new(|_| DialogFocusHarness {
                    open,
                    origin,
                    field,
                });
                Root::new(view, window, cx)
            }
        });
        let (origin, field) = handles.borrow().clone().unwrap();
        let mut visual = VisualTestContext::from_window(window.into(), cx);
        visual.run_until_parked();
        visual.update(|window, cx| {
            origin.focus(window, cx);
            open.set(true);
            window.refresh();
        });
        visual.run_until_parked();
        let row = visual
            .debug_bounds("dialog-opener:menu:item:0")
            .expect("open menu row");
        visual.simulate_click(row.center(), gpui::Modifiers::default());
        visual.run_until_parked();
        visual.update(|window, cx| {
            assert_eq!(
                window.focused(cx),
                Some(field.clone()),
                "menu close stole modal focus"
            );
            assert!(window.has_active_dialog(cx));
        });
        let nested_trigger = visual
            .debug_bounds("dialog-inner-trigger")
            .expect("dialog menu trigger");
        visual.simulate_click(nested_trigger.center(), gpui::Modifiers::default());
        visual.run_until_parked();
        let nested_row = visual
            .debug_bounds("dialog-inner:menu:item:0")
            .expect("menu inside dialog must open");
        visual.simulate_click(nested_row.center(), gpui::Modifiers::default());
        visual.run_until_parked();
        visual.update(|window, cx| {
            assert_eq!(
                window.focused(cx),
                Some(field.clone()),
                "nested menu must return to modal field"
            );
            window.close_dialog(cx);
            assert_eq!(
                window.focused(cx),
                Some(origin.clone()),
                "dialog returned to hidden menu"
            );
        });
    }
}

/// A controlled popover with either a descendant or explicitly tracked content focus.
struct ClosingFocusHarness {
    open: Rc<Cell<bool>>,
    origin: FocusHandle,
    child: FocusHandle,
    other: FocusHandle,
    tracked: bool,
}

impl Render for ClosingFocusHarness {
    /// Render the same ownership tree used by a retained input inside a controlled popover.
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let child = self.child.clone();
        let mut popup = Popover::new("closing-owner")
            .open(self.open.get())
            .trigger_any(div().w(px(20.)).h(px(20.)))
            .content(move |_, _, _| div().track_focus(&child).w(px(100.)).h(px(40.)));
        if self.tracked {
            popup = popup.track_focus(&self.child);
        }
        div()
            .track_focus(&self.origin)
            .child(popup)
            .child(div().track_focus(&self.other))
    }
}

/// Restoring only the popover's exact handle strands descendant/tracked input focus after close;
/// restoring no-prior focus to the removed handle also strands application hotkeys.
#[gpui::test]
fn controlled_close_releases_descendant_and_tracked_focus(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init);
    for theme in [ThemeMode::Dark, ThemeMode::Light] {
        cx.update(|cx| MoonTheme::set_mode(theme, cx));
        for tracked in [false, true] {
            for (previous, transfer) in [(false, false), (true, false), (true, true)] {
                let open = Rc::new(Cell::new(false));
                let window = cx.add_window({
                    let open = open.clone();
                    move |_, cx| ClosingFocusHarness {
                        open,
                        origin: cx.focus_handle(),
                        child: cx.focus_handle(),
                        other: cx.focus_handle(),
                        tracked,
                    }
                });
                let (origin, child, other) = window
                    .read_with(cx, |view, _| {
                        (view.origin.clone(), view.child.clone(), view.other.clone())
                    })
                    .unwrap();
                let mut visual = VisualTestContext::from_window(window.into(), cx);
                visual.run_until_parked();
                visual.update(|window, cx| {
                    if previous {
                        origin.focus(window, cx);
                    } else {
                        window.blur();
                    }
                    open.set(true);
                    window.refresh();
                });
                visual.run_until_parked();
                visual.update(|window, cx| {
                    child.focus(window, cx);
                    if transfer {
                        other.focus(window, cx);
                    }
                    open.set(false);
                    window.refresh();
                });
                visual.run_until_parked();
                visual.update(|window, cx| {
                    assert_eq!(
                        window.focused(cx),
                        if transfer {
                            Some(other.clone())
                        } else {
                            previous.then(|| origin.clone())
                        },
                        "close must release removed content, tracked={tracked}, previous={previous}"
                    );
                });
            }
        }
    }
}
