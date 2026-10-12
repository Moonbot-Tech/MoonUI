//! Regression coverage for selectable-card chrome, clicks and keyboard selection.

use std::{cell::RefCell, rc::Rc};

use gpui::{
    AppContext as _, Context, IntoElement, KeyUpEvent, Keystroke, Modifiers, ParentElement as _,
    Render, VisualTestContext, Window, div,
};

// Do not use `super::*`: the glob would import the `gpui::test` macro, causing `#[test]` to
// recursively expand into itself.
use super::{MoonPalette, MoonSelectableCard, card_chrome};

/// Catches `selectable_card.rs:card_chrome` leaving the selected card on the neutral border, which
/// would make the chosen theme card look exactly like the others.
#[test]
fn only_the_selected_card_takes_the_accent_border() {
    let accent = 0x2f80ed;
    for p in [MoonPalette::TERMINAL, MoonPalette::LIGHT] {
        let selected = card_chrome(true, false, accent, p);
        let idle = card_chrome(false, false, accent, p);
        let focused = card_chrome(false, true, accent, p);
        assert_eq!(selected.border, accent);
        assert!(selected.ring.is_some());
        assert_eq!(idle.border, p.border);
        assert_eq!(idle.ring, None);
        assert_eq!(
            focused.border, p.border,
            "focus alone must not look selected"
        );
        assert!(
            focused.ring.is_some(),
            "a focused card must show a focus ring"
        );
    }
}

/// Root view with one enabled and one disabled card inside a clickable ancestor.
struct CardHarness {
    clicks: Rc<RefCell<Vec<&'static str>>>,
}

impl Render for CardHarness {
    /// Render the two cards with recording click handlers.
    ///
    /// Args:
    ///     _window: Test window.
    ///     _cx: Test view context.
    ///
    /// Returns:
    ///     The rendered cards.
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let enabled = self.clicks.clone();
        let disabled = self.clicks.clone();
        div()
            .child(
                MoonSelectableCard::new("dark")
                    .on_click(move |_, _, _| enabled.borrow_mut().push("dark"))
                    .child(div().child("Dark")),
            )
            .child(
                MoonSelectableCard::new("light")
                    .disabled(true)
                    .on_click(move |_, _, _| disabled.borrow_mut().push("light"))
                    .child(div().child("Light")),
            )
    }
}

/// Catches dropping `track_focus` or the click wiring in `selectable_card.rs`: theme cards would
/// then be unreachable from the keyboard, or a disabled card would still switch the theme.
#[gpui::test]
fn cards_select_by_click_and_keyboard_but_not_when_disabled(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init);
    let clicks = Rc::new(RefCell::new(Vec::new()));
    let window = cx.add_window({
        let clicks = clicks.clone();
        move |window, cx| {
            let view = cx.new(|_| CardHarness { clicks });
            crate::Root::new(view, window, cx).bordered(false)
        }
    });
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    let dark = cx
        .debug_bounds("moon-selectable-card:dark")
        .expect("enabled card must render");
    let light = cx
        .debug_bounds("moon-selectable-card:light")
        .expect("disabled card must render");
    cx.simulate_click(light.center(), Modifiers::none());
    cx.simulate_click(dark.center(), Modifiers::none());
    cx.run_until_parked();
    assert_eq!(&*clicks.borrow(), &["dark"]);
    clicks.borrow_mut().clear();

    // GPUI clicks a focused element when Space is released; `simulate_keystrokes` only presses.
    cx.simulate_keystrokes("space");
    cx.simulate_event(KeyUpEvent {
        keystroke: Keystroke::parse("space").expect("space must parse"),
    });
    cx.run_until_parked();
    assert_eq!(
        &*clicks.borrow(),
        &["dark"],
        "the clicked card keeps focus and Space selects it again"
    );
}
