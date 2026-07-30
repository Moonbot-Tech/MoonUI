//! Rendered interaction coverage for Moon status-bar items.

use std::{cell::RefCell, rc::Rc};

use gpui::{Context, IntoElement, Modifiers, Render, VisualTestContext, Window};

use super::{MoonStatusBar, MoonStatusItem};
use crate::moon::{MoonPalette, MoonTheme};

/// View harness with one actionable label and three inert status-item kinds.
struct StatusItemInteractionHarness {
    clicks: Rc<RefCell<usize>>,
}

impl Render for StatusItemInteractionHarness {
    /// Render status items whose actual bounds receive the simulated clicks.
    ///
    /// Args:
    ///     _window: Test window receiving native pointer events.
    ///     _cx: Test view context.
    ///
    /// Returns:
    ///     A status bar containing actionable and inert items.
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let action_clicks = self.clicks.clone();
        let separator_clicks = self.clicks.clone();
        let group_separator_clicks = self.clicks.clone();
        MoonStatusBar::new("status-item-interaction").items([
            MoonStatusItem::new("action")
                .id("status-action")
                .tooltip("Action tooltip")
                .on_click(move |_, _, _| {
                    *action_clicks.borrow_mut() += 1;
                }),
            MoonStatusItem::new("static").id("status-static"),
            MoonStatusItem::separator()
                .id("status-separator")
                .on_click(move |_, _, _| {
                    *separator_clicks.borrow_mut() += 10;
                }),
            MoonStatusItem::group_separator()
                .id("status-group-separator")
                .on_click(move |_, _, _| {
                    *group_separator_clicks.borrow_mut() += 100;
                }),
        ])
    }
}

/// Render and click every item kind under one palette.
///
/// Args:
///     cx: GPUI test application context.
///     palette: Palette used to render the status bar.
///
/// Returns:
///     Nothing; the independent callback counter is asserted after each click.
fn assert_status_item_interactions(cx: &mut gpui::TestAppContext, palette: MoonPalette) {
    cx.update(|cx| {
        MoonTheme::global_mut(cx).palette = palette;
    });
    let clicks = Rc::new(RefCell::new(0));
    let window = cx.add_window({
        let clicks = clicks.clone();
        move |_, _| StatusItemInteractionHarness { clicks }
    });
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();

    let bounds = [
        "status-action",
        "status-static",
        "status-separator",
        "status-group-separator",
    ]
    .map(|selector| {
        cx.debug_bounds(selector)
            .expect("every status item must register its rendered bounds")
    });

    cx.simulate_click(bounds[0].center(), Modifiers::default());
    assert_eq!(
        *clicks.borrow(),
        1,
        "the actionable text item must dispatch exactly one callback"
    );

    for inert in &bounds[1..] {
        cx.simulate_click(inert.center(), Modifiers::default());
    }
    assert_eq!(
        *clicks.borrow(),
        1,
        "static text and both separator kinds must remain inert"
    );
}

/// `status_bar.rs:MoonStatusBar::render_items` removing the text item's native `on_click` branch
/// must redden the first callback-count assertion; otherwise MoonTerminal's visible status actions
/// would stop responding after their external overlay hitboxes are removed.
#[gpui::test]
fn interactive_text_item_dispatches_click_without_external_hitbox(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init);
    for palette in [MoonPalette::TERMINAL, MoonPalette::LIGHT] {
        assert_status_item_interactions(cx, palette);
    }
}
