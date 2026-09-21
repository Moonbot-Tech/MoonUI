//! Input and rendered-geometry regressions for cascading menus.

use super::{MoonMenuItem, MoonPopupMenu};
use crate::moon::foundation::MoonSize;
use crate::moon::{MoonScale, MoonTheme, ThemeMode};
use gpui::{ParentElement as _, Styled as _};
use std::{cell::Cell, rc::Rc};

/// Place two branches and a disabled branch near the requested viewport corner.
struct CascadeHarness {
    edge: bool,
    count: usize,
    parent_count: usize,
    calls: Rc<Cell<usize>>,
}

impl gpui::Render for CascadeHarness {
    /// Build real nested rows whose callbacks expose accidental hover activation.
    fn render(
        &mut self,
        window: &mut gpui::Window,
        _: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        let calls = self.calls.clone();
        let pos = if self.edge {
            gpui::point(
                window.viewport_size().width - gpui::px(205.0),
                window.viewport_size().height - gpui::px(145.0),
            )
        } else {
            gpui::point(gpui::px(30.0), gpui::px(30.0))
        };
        gpui::div().size_full().child(
            gpui::div().absolute().left(pos.x).top(pos.y).child(
                MoonPopupMenu::new("cascade")
                    // Pinned to the pre-density default's row geometry: an unset menu used to
                    // render at the old fixed `Normal` (24px) rows, but now follows the theme
                    // tier, which defaults to `Md` (32px). This is a rendered-geometry
                    // hover/flip regression whose viewport-corner placement and submenu-open
                    // assertions depend on exact row height, so it must keep testing against
                    // the geometry it was written for rather than silently drifting with the
                    // new default.
                    .size(MoonSize::Sm)
                    .width(180.0)
                    .items([
                        MoonMenuItem::new("First").submenu((0..self.count).map(|ix| {
                            let calls = calls.clone();
                            MoonMenuItem::new(format!("Action {ix}"))
                                .on_click(move |_, _, _| calls.set(calls.get() + 1))
                        })),
                        MoonMenuItem::new("Second")
                            .submenu([MoonMenuItem::new("Nested")
                                .submenu([MoonMenuItem::new("Deep action")])]),
                        MoonMenuItem::new("Disabled")
                            .disabled(true)
                            .submenu([MoonMenuItem::new("Never")]),
                        MoonMenuItem::new("Ordinary"),
                    ])
                    .items(
                        (4..self.parent_count).map(|ix| MoonMenuItem::new(format!("Parent {ix}"))),
                    ),
            ),
        )
    }
}

/// Catches restoring selected-only expansion or clearing a branch on row leave: hovering must open
/// without invoking actions, crossing into its child must preserve it, and siblings must replace it.
#[gpui::test]
fn submenu_hover_switches_without_clicking_and_preserves_child_access(
    cx: &mut gpui::TestAppContext,
) {
    cx.update(crate::init);
    for (theme, parent_count) in [
        (ThemeMode::Dark, 4),
        (ThemeMode::Light, 4),
        (ThemeMode::Dark, 100),
        (ThemeMode::Light, 100),
    ] {
        cx.update(|cx| MoonTheme::set_mode(theme, cx));
        let calls = Rc::new(Cell::new(0));
        let sink = calls.clone();
        let window = cx.add_window(move |_, _| CascadeHarness {
            edge: false,
            count: 3,
            parent_count,
            calls: sink,
        });
        let mut view = gpui::VisualTestContext::from_window(window.into(), cx);
        view.run_until_parked();
        assert!(view.debug_bounds("cascade:submenu:0").is_none());
        let first = view.debug_bounds("cascade:item:0").unwrap();
        view.simulate_mouse_move(first.center(), None, gpui::Modifiers::none());
        view.run_until_parked();
        let child = view
            .debug_bounds("cascade:submenu:0:item:0")
            .expect("hover must open the first submenu");
        assert_eq!(calls.get(), 0, "hover must not execute a leaf action");
        view.simulate_mouse_move(child.center(), None, gpui::Modifiers::none());
        view.run_until_parked();
        assert!(
            view.debug_bounds("cascade:submenu:0:item:0").is_some(),
            "moving into the child must retain the branch"
        );
        view.simulate_click(child.center(), gpui::Modifiers::none());
        assert_eq!(
            calls.get(),
            1,
            "the child outside the parent must remain clickable exactly once"
        );
        for ix in [1, 2, 0, 3] {
            let row = view
                .debug_bounds(
                    [
                        "cascade:item:0",
                        "cascade:item:1",
                        "cascade:item:2",
                        "cascade:item:3",
                    ][ix],
                )
                .unwrap();
            view.simulate_mouse_move(row.center(), None, gpui::Modifiers::none());
            view.run_until_parked();
            assert_eq!(
                view.debug_bounds("cascade:submenu:0").is_some(),
                ix == 0,
                "old branch must close on sibling hover"
            );
            assert_eq!(
                view.debug_bounds("cascade:submenu:1").is_some(),
                ix == 1,
                "enabled sibling must open automatically"
            );
            assert!(
                view.debug_bounds("cascade:submenu:2").is_none(),
                "disabled branches must stay closed"
            );
        }
    }
}

/// Catches removing the side flip, viewport height cap, or prepaint translation: eager and virtual
/// submenus must fit near both edges at enlarged UI scale, including a third menu level.
#[gpui::test]
fn submenu_flips_left_and_caps_height_at_viewport_edges(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init);
    for theme in [ThemeMode::Dark, ThemeMode::Light] {
        for count in [12, 100] {
            cx.update(|cx| {
                MoonTheme::set_mode(theme, cx);
                MoonTheme::global_mut(cx).scale = MoonScale {
                    ui: 1.25,
                    font: 1.0,
                    font_delta: 2.0,
                    tier: Default::default(),
                    zoom: 1.0,
                };
            });
            let window = cx.add_window(move |_, _| CascadeHarness {
                edge: true,
                count,
                parent_count: 4,
                calls: Rc::new(Cell::new(0)),
            });
            let mut view = gpui::VisualTestContext::from_window(window.into(), cx);
            view.run_until_parked();
            let viewport = view.update(|window, _| window.viewport_size());
            let first = view.debug_bounds("cascade:item:0").unwrap();
            view.simulate_mouse_move(first.center(), None, gpui::Modifiers::none());
            view.run_until_parked();
            let child = view
                .debug_bounds("cascade:submenu:0")
                .expect("edge submenu must open on hover");
            assert!(
                child.right() <= first.left(),
                "submenu must flip to the parent's left at the right edge: row={first:?} child={child:?} viewport={viewport:?}"
            );
            assert!(
                child.left() >= gpui::px(6.0) && child.right() <= viewport.width - gpui::px(6.0),
                "submenu must fit horizontally"
            );
            assert!(
                child.top() >= gpui::px(6.0) && child.bottom() <= viewport.height - gpui::px(6.0),
                "submenu must fit vertically"
            );
            let second = view.debug_bounds("cascade:item:1").unwrap();
            view.simulate_mouse_move(second.center(), None, gpui::Modifiers::none());
            view.run_until_parked();
            let nested = view.debug_bounds("cascade:submenu:1:item:0").unwrap();
            view.simulate_mouse_move(nested.center(), None, gpui::Modifiers::none());
            view.run_until_parked();
            let deep = view
                .debug_bounds("cascade:submenu:1:submenu:0")
                .expect("third level must also open by hover");
            assert!(deep.left() >= gpui::px(6.0) && deep.right() <= viewport.width - gpui::px(6.0));
            assert!(
                deep.top() >= gpui::px(6.0) && deep.bottom() <= viewport.height - gpui::px(6.0)
            );
        }
    }
}
