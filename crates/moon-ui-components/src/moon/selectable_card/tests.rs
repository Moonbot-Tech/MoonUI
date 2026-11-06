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

// ---- Theme skin: the card surface and how the selected card is told apart ----

use super::card_facet;
use crate::moon::{MoonFacetKind, MoonSkin, MoonThemeTokens, facet, rgba_from};
use gpui::{BoxShadow, point, px};

/// Theme tokens for `palette` drawn with `skin`, with every length doubled.
fn doubled_skin_tokens(palette: MoonPalette, skin: MoonSkin) -> MoonThemeTokens {
    let mut tokens = MoonThemeTokens {
        palette,
        skin,
        ..MoonThemeTokens::default()
    };
    tokens.scale.ui = 2.0;
    tokens
}

/// Catches `selectable_card.rs:card_facet` ignoring the skin: every flat theme's cards would
/// trade the `panel_high` fill and the hard selected ring `card_chrome` gives them for a
/// gradient card with a glow.
#[test]
fn a_flat_card_is_left_to_its_chrome() {
    for palette in [MoonPalette::TERMINAL, MoonPalette::LIGHT] {
        let tokens = doubled_skin_tokens(palette, MoonSkin::Flat);
        for (selected, focused) in [(false, false), (true, false), (false, true), (true, true)] {
            assert_eq!(card_facet(selected, focused, palette.accent, &tokens), None);
        }
    }
}

/// Catches `selectable_card.rs:card_facet` keeping the Flat skin's hard one-pixel ring on the
/// selected card, dropping its glow, or leaving its edge on the neutral colour: the chosen
/// theme card would either look ringed like a flat one or exactly like the cards beside it.
#[test]
fn a_faceted_card_marks_selection_with_an_accent_edge_and_a_glow() {
    let p = MoonPalette::OBSIDIAN;
    let tokens = doubled_skin_tokens(p, MoonSkin::Facet);
    let accent = p.blue;
    let card = facet(MoonFacetKind::Card, &tokens);

    let idle = card_facet(false, false, accent, &tokens).expect("the Facet skin resolves a card");
    assert_eq!(idle.fill, card.fill);
    assert_eq!(idle.edge, card.edge);
    assert_eq!(idle.shadows, card.shadows);
    assert_eq!(idle.hover_edge, rgba_from(p.overlay, 0.22));

    // The glow written out: centred on the card, no spread, 22 design pixels of blur doubled
    // with the UI scale and halved into the renderer's blur radius.
    let glow = BoxShadow {
        color: rgba_from(accent, 0.2),
        offset: point(px(0.0), px(0.0)),
        blur_radius: px(22.0),
        spread_radius: px(0.0),
        inset: false,
    };
    let mut card_and_glow = card.shadows.clone();
    card_and_glow.push(glow);
    for focused in [false, true] {
        let selected =
            card_facet(true, focused, accent, &tokens).expect("the Facet skin resolves a card");
        assert_eq!(selected.fill, card.fill);
        assert_eq!(selected.edge, rgba_from(accent, 0.75));
        assert_eq!(selected.shadows, card_and_glow, "focused: {focused}");
        assert!(
            selected
                .shadows
                .iter()
                .all(|shadow| shadow.spread_radius == px(0.0)),
            "no hard spread ring on a selected faceted card"
        );
    }

    // The focus ring written out: two design pixels of spread, doubled, and no blur.
    let ring = BoxShadow {
        color: rgba_from(accent, 0.55),
        offset: point(px(0.0), px(0.0)),
        blur_radius: px(0.0),
        spread_radius: px(4.0),
        inset: false,
    };
    let mut card_and_ring = card.shadows.clone();
    card_and_ring.push(ring);
    let focused = card_facet(false, true, accent, &tokens).expect("the Facet skin resolves a card");
    assert_eq!(
        focused.edge, card.edge,
        "focus alone must not look selected"
    );
    assert_eq!(
        focused.shadows, card_and_ring,
        "a focused card keeps its focus ring"
    );
}
