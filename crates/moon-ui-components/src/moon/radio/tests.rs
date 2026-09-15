//! Regression coverage for MoonRadio behavior and reviewed geometry.

use std::{cell::RefCell, rc::Rc};

use gpui::{
    Bounds, Context, InteractiveElement, IntoElement, KeyUpEvent, Keystroke, Modifiers,
    ParentElement, Pixels, Render, Styled, TestAppContext, VisualTestContext, Window, div, point,
    px, size,
};

use super::super::{MoonCheckbox, MoonThemeConfig, foundation::MoonSize};
use super::{MoonRadio, MoonRadioSize, RadioMetrics, moon_radio_click_value};

/// Catches `radio.rs:RadioMetrics::resolve` drifting from the checkbox tiers it shares: every tier
/// must resolve to the circle, text and spacing of the checkbox of the same size (16px circle,
/// 14/20 text, 8px gap for `Sm`; 20px, 16/24, 12px for `Md`), snapping `Xs` to `Sm` and `Lg` and
/// above to `Md`, with a 6 or 8px dot.
#[test]
fn radio_tiers_match_checkbox_tiers() {
    let tokens = MoonThemeConfig::moon_terminal().dark;
    for (tier, box_px, font_px, line_px, gap_px, dot_px) in [
        (MoonSize::Xs, 16., 14., 20., 8., 6.),
        (MoonSize::Sm, 16., 14., 20., 8., 6.),
        (MoonSize::Md, 20., 16., 24., 12., 8.),
        (MoonSize::Lg, 20., 16., 24., 12., 8.),
        (MoonSize::Xl, 20., 16., 24., 12., 8.),
        (MoonSize::Xxl, 20., 16., 24., 12., 8.),
    ] {
        let metrics = RadioMetrics::resolve(tier.into(), &tokens);
        assert_eq!(
            metrics.choice,
            crate::checkbox::MoonCheckboxMetrics::resolve(
                super::super::checkbox::tier_size(tier),
                &tokens
            ),
            "{tier:?} radio must share the {tier:?} checkbox metrics"
        );
        assert_eq!(metrics.choice.box_size, px(box_px));
        assert_eq!(metrics.choice.font_size, px(font_px));
        assert_eq!(metrics.choice.line_height, px(line_px));
        assert_eq!(metrics.choice.gap, px(gap_px));
        assert_eq!(metrics.dot_size, px(dot_px));
    }
}

/// Catches the deprecated `Compact`/`Normal` aliases drifting from the tiers they replaced, which
/// would silently resize radios in apps that have not migrated yet.
#[test]
#[allow(deprecated)]
fn deprecated_radio_sizes_keep_their_tiers() {
    assert_eq!(MoonRadioSize::Compact, MoonSize::Sm.into());
    assert_eq!(MoonRadioSize::Normal, MoonSize::Md.into());
    assert_eq!(MoonRadio::new("default").size, MoonSize::Md.into());
}

/// Catches making `radio.rs:moon_radio_click_value` select disabled radios or ignore enabled
/// clicks, which would let unavailable choices change or leave available choices unselected.
#[test]
fn radio_click_value_respects_disabled_state() {
    assert_eq!(moon_radio_click_value(false), Some(true));
    assert_eq!(moon_radio_click_value(true), None);
}

const WRAPPING_LABEL: &str = "Close every open position when the stop price is reached";
const DESCRIPTION: &str = "Sent at market";

/// A checkbox over a radio with the same wrapping label and description in a narrow column.
struct CheckboxAndRadioHarness {
    size: MoonSize,
}

impl Render for CheckboxAndRadioHarness {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(180.))
            .flex()
            .flex_col()
            .child(
                MoonCheckbox::new("checkbox")
                    .size(self.size)
                    .label(WRAPPING_LABEL)
                    .description(DESCRIPTION)
                    .checked(true),
            )
            .child(
                MoonRadio::new("radio")
                    .size(self.size)
                    .label(WRAPPING_LABEL)
                    .description(DESCRIPTION)
                    .checked(true),
            )
    }
}

/// Returns the bounds of a control's box, label and description, found by `selectors` in that
/// order, each placed relative to the box's origin.
fn parts_from_box(cx: &mut VisualTestContext, selectors: [&'static str; 3]) -> [Bounds<Pixels>; 3] {
    let parts = selectors.map(|selector| {
        cx.debug_bounds(selector)
            .unwrap_or_else(|| panic!("{selector} must render"))
    });
    let box_origin = parts[0].origin;
    parts.map(|part| Bounds::new(part.origin - box_origin, part.size))
}

/// Catches a radio laying out its text differently from the checkbox of the same size: the circle
/// must match the box in size and the label and description must sit at the same offsets from it,
/// so a wrapping label keeps the circle on its first line and the description stacks the same way.
#[gpui::test]
fn radio_text_lays_out_like_the_checkbox_of_its_size(cx: &mut TestAppContext) {
    cx.update(crate::init);
    for (tier, line_px) in [(MoonSize::Sm, 20.), (MoonSize::Md, 24.)] {
        let window = cx.add_window(move |_, _| CheckboxAndRadioHarness { size: tier });
        let mut cx = VisualTestContext::from_window(window.into(), cx);
        cx.run_until_parked();

        let checkbox = parts_from_box(
            &mut cx,
            ["checkbox:box", "checkbox:label", "checkbox:description"],
        );
        let radio = parts_from_box(&mut cx, ["radio:box", "radio:label", "radio:description"]);
        assert!(
            radio[1].size.height >= px(line_px * 2.),
            "{tier:?} label must wrap for this case to mean anything"
        );
        assert_eq!(
            radio, checkbox,
            "{tier:?} radio must lay out like the checkbox"
        );
    }
}

/// Catches the radio's focus ring changing layout or losing its reviewed geometry: on keyboard focus
/// the radio must draw a ring whose outer edge sits 4px outside the circle on every side, as the
/// checkbox does, while the circle and label keep exactly their unfocused bounds.
#[gpui::test]
fn radio_focus_ring_surrounds_the_circle_without_moving_layout(cx: &mut TestAppContext) {
    cx.update(crate::init);
    for tier in [MoonSize::Sm, MoonSize::Md] {
        let window = cx.add_window(move |_, _| CheckboxAndRadioHarness { size: tier });
        let mut cx = VisualTestContext::from_window(window.into(), cx);
        cx.run_until_parked();

        let circle_before = cx.debug_bounds("radio:box").expect("circle must render");
        let label_before = cx.debug_bounds("radio:label").expect("label must render");
        assert!(cx.debug_bounds("radio:focus-ring").is_none());

        // The checkbox comes first in the harness, so the second stop is the radio.
        cx.update(|window, cx| {
            window.focus_next(cx);
            window.focus_next(cx);
        });
        cx.run_until_parked();

        assert!(cx.debug_bounds("checkbox:focus-ring").is_none());
        let ring = cx
            .debug_bounds("radio:focus-ring")
            .expect("focused radio must draw its ring");
        assert_eq!(cx.debug_bounds("radio:box"), Some(circle_before));
        assert_eq!(cx.debug_bounds("radio:label"), Some(label_before));
        assert_eq!(ring.origin, circle_before.origin - point(px(4.), px(4.)));
        assert_eq!(ring.size, circle_before.size + size(px(8.), px(8.)));
    }
}

/// A group of three labelled radios whose middle one is disabled, recording every change.
struct RadioGroupHarness {
    changes: Rc<RefCell<Vec<(&'static str, bool)>>>,
}

impl Render for RadioGroupHarness {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().children([("first", false), ("second", true), ("third", false)].map(
            |(id, disabled)| {
                let changes = self.changes.clone();
                MoonRadio::new(id)
                    .label(id)
                    .disabled(disabled)
                    .on_change(move |value, _, _| changes.borrow_mut().push((id, *value)))
            },
        ))
    }
}

/// Catches a radio that swallows focus on press or can't be used from the keyboard: pressing an
/// enabled radio must focus it (its listener stops the press before GPUI's own focus-on-press),
/// Tab must move on past a disabled radio to the next enabled one, and Space on the focused radio
/// must select it.
#[gpui::test]
fn pressing_a_radio_focuses_it_and_the_keyboard_selects(cx: &mut TestAppContext) {
    use gpui::AppContext as _;

    cx.update(crate::init);
    let changes = Rc::new(RefCell::new(Vec::new()));
    let window = cx.add_window({
        let changes = changes.clone();
        move |window, cx| {
            let view = cx.new(|_| RadioGroupHarness { changes });
            crate::Root::new(view, window, cx).bordered(false)
        }
    });
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();
    assert!(cx.debug_bounds("first:focus-ring").is_none());

    let label = cx.debug_bounds("first:label").expect("label must render");
    cx.simulate_click(label.center(), Modifiers::none());
    cx.run_until_parked();
    assert!(cx.debug_bounds("first:focus-ring").is_some());
    assert_eq!(&*changes.borrow(), &[("first", true)]);

    cx.simulate_keystrokes("tab");
    cx.run_until_parked();
    assert!(cx.debug_bounds("first:focus-ring").is_none());
    assert!(cx.debug_bounds("second:focus-ring").is_none());
    assert!(cx.debug_bounds("third:focus-ring").is_some());

    // GPUI clicks a focused element when Space is released; `simulate_keystrokes` only presses.
    cx.simulate_keystrokes("space");
    cx.simulate_event(KeyUpEvent {
        keystroke: Keystroke::parse("space").expect("space must parse"),
    });
    cx.run_until_parked();
    assert_eq!(&*changes.borrow(), &[("first", true), ("third", true)]);
}

/// A radio with an optional label in a flex row that shrinks the probe to the radio.
struct ProbedRadioHarness {
    size: MoonSize,
    label: Option<&'static str>,
}

impl Render for ProbedRadioHarness {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let radio = MoonRadio::new("probed").size(self.size).checked(true);
        crate::h_flex().child(div().debug_selector(|| "probed-control".into()).child(
            match self.label {
                Some(label) => radio.label(label),
                None => radio,
            },
        ))
    }
}

/// Catches a textless radio reserving text space (the text column and its 8 or 12px gap) for a
/// missing or empty label, or a checked dot drifting from its reviewed size or off the circle's
/// centre: the radio must be exactly its 16 or 20px circle, with a centred 6 or 8px dot.
#[gpui::test]
fn radio_without_text_is_its_circle_with_a_centred_dot(cx: &mut TestAppContext) {
    cx.update(crate::init);
    for (tier, box_px, dot_px) in [(MoonSize::Sm, 16., 6.), (MoonSize::Md, 20., 8.)] {
        for label in [None, Some("")] {
            let window = cx.add_window(move |_, _| ProbedRadioHarness { size: tier, label });
            let mut cx = VisualTestContext::from_window(window.into(), cx);
            cx.run_until_parked();

            let control = cx
                .debug_bounds("probed-control")
                .expect("probe must render");
            let circle = cx.debug_bounds("probed:box").expect("circle must render");
            let dot = cx
                .debug_bounds("probed:dot")
                .expect("checked radio must draw its dot");
            assert_eq!(
                control.size,
                size(px(box_px), px(box_px)),
                "{tier:?} radio with label {label:?} must be exactly its circle"
            );
            assert_eq!(dot.size, size(px(dot_px), px(dot_px)));
            assert_eq!(dot.center(), circle.center());
        }
    }
}
