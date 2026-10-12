//! Regression coverage for number-input parsing, sizing, seeding and keyboard stepping.

use std::{cell::RefCell, rc::Rc};

use gpui::{
    AppContext as _, Context, Entity, InteractiveElement as _, IntoElement, ParentElement as _,
    Render, Styled as _, VisualTestContext, Window, div,
};

// Do not use `super::*`: the glob would import the `gpui::test` macro, causing `#[test]` to
// recursively expand into itself.
use super::{
    MoonInputState, MoonNumberInput, MoonSize, Size, base_size_for, clamp_to_range,
    min_control_width, normalize_step, parse_number, widest_value_text,
};

/// Catches `number_input.rs:parse_number` returning 0 for a half-typed `-`, which would make a
/// consumer apply a zero amount while the user is still typing a negative one.
#[test]
fn partial_and_empty_entries_have_no_value() {
    assert_eq!(parse_number(""), None);
    assert_eq!(parse_number("-"), None);
    assert_eq!(parse_number("."), None);
    assert_eq!(parse_number("inf"), None);
    assert_eq!(parse_number(" 12.5 "), Some(12.5));
    assert_eq!(parse_number("-3"), Some(-3.0));
}

/// Catches `number_input.rs:normalize_step` passing a zero or negative step to the base input,
/// whose buttons would then never move the value or move it the wrong way.
#[test]
fn unusable_steps_fall_back_to_one() {
    assert_eq!(normalize_step(0.0), 1.0);
    assert_eq!(normalize_step(-2.0), 1.0);
    assert_eq!(normalize_step(f64::NAN), 1.0);
    assert_eq!(normalize_step(0.25), 0.25);
}

/// Catches `number_input.rs:clamp_to_range` letting an out-of-range default through, which would
/// show a value the control itself would refuse to step to.
#[test]
fn seed_values_clamp_into_the_range() {
    assert_eq!(clamp_to_range(15.0, Some(0.0), Some(10.0)), 10.0);
    assert_eq!(clamp_to_range(-5.0, Some(0.0), Some(10.0)), 0.0);
    assert_eq!(clamp_to_range(-5.0, None, None), -5.0);
    assert_eq!(clamp_to_range(5.0, Some(0.0), None), 5.0);
}

/// Catches `number_input.rs:base_size_for` rendering every tier at one height, which would break
/// the row alignment number inputs share with the other tier-sized inputs.
#[test]
fn tiers_map_onto_the_base_input_sizes() {
    assert_eq!(base_size_for(MoonSize::Xs), Size::XSmall);
    assert_eq!(base_size_for(MoonSize::Sm), Size::Small);
    assert_eq!(base_size_for(MoonSize::Md), Size::Medium);
    assert_eq!(base_size_for(MoonSize::Xxl), Size::Medium);
}

/// Root view hosting one number input bound to a caller-owned state.
struct NumberInputHarness {
    state: Entity<MoonInputState>,
}

impl Render for NumberInputHarness {
    /// Render a 0..=10 number input stepping by 4 with a unit.
    ///
    /// Args:
    ///     _window: Test window.
    ///     _cx: Test view context.
    ///
    /// Returns:
    ///     The rendered number input.
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        MoonNumberInput::new("number-input-harness")
            .state(&self.state)
            .range(0.0, 10.0)
            .step(4.0)
            .unit("ms")
    }
}

/// Catches dropping the builder's step or range on the way to the base input in
/// `number_input.rs:MoonNumberInput::render`: Up/Down would then step by 1 and run past the bounds.
#[gpui::test]
fn up_and_down_keys_step_and_clamp_to_the_range(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init);
    let slot: Rc<RefCell<Option<Entity<MoonInputState>>>> = Rc::new(RefCell::new(None));
    let window = cx.add_window({
        let slot = slot.clone();
        move |window, cx| {
            let state = cx.new(|cx| MoonInputState::new(window, cx).default_value("5"));
            *slot.borrow_mut() = Some(state.clone());
            let view = cx.new(|_| NumberInputHarness { state });
            crate::Root::new(view, window, cx).bordered(false)
        }
    });
    let state = slot.borrow().clone().expect("harness state is created");
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();
    cx.update(|window, cx| state.update(cx, |state, cx| state.focus(window, cx)));
    cx.run_until_parked();

    cx.simulate_keystrokes("up");
    assert_eq!(
        cx.update(|_, cx| MoonNumberInput::read_value(&state, cx)),
        Some(9.0)
    );
    cx.simulate_keystrokes("up");
    assert_eq!(
        cx.update(|_, cx| MoonNumberInput::read_value(&state, cx)),
        Some(10.0),
        "stepping past max must clamp to max"
    );
    cx.simulate_keystrokes("down down down");
    assert_eq!(
        cx.update(|_, cx| MoonNumberInput::read_value(&state, cx)),
        Some(0.0),
        "stepping past min must clamp to min"
    );
}

/// Catches measuring only the current text or only one range end: a field holding `5` in a
/// 0..=100000 range would then be sized for one digit and cut the larger values off.
#[test]
fn the_widest_value_is_the_longest_range_end_or_six_open_digits() {
    assert_eq!(widest_value_text(Some(0.0), Some(100000.0), "5"), "100000");
    assert_eq!(widest_value_text(Some(-250.5), Some(10.0), ""), "-250.5");
    assert_eq!(widest_value_text(None, Some(10.0), "3"), "000000");
    assert_eq!(
        widest_value_text(Some(1.0), Some(9.0), "12345678"),
        "12345678"
    );
}

/// Catches dropping a term from the width floor. Hand-computed: two 24px buttons, 8px padding on
/// both sides, a 40px value and a 12px unit after a 4px gap is 48 + 16 + 40 + 16 = 120.
#[test]
fn the_width_floor_adds_buttons_padding_value_and_unit() {
    assert_eq!(min_control_width(40.0, 12.0, 24.0, 8.0, 4.0), 120.0);
    assert_eq!(min_control_width(40.0, 0.0, 24.0, 8.0, 4.0), 104.0);
}

/// A number input inside a content-sized row, as Settings lays it out.
struct ContentSizedHarness {
    state: Entity<MoonInputState>,
}

impl Render for ContentSizedHarness {
    /// Render a 0..=100000 input with a unit inside a content-sized row.
    ///
    /// Args:
    ///     _window: Test window.
    ///     _cx: Test view context.
    ///
    /// Returns:
    ///     The row holding the input.
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().flex().child(
            div().debug_selector(|| "probe-number".into()).child(
                MoonNumberInput::new("number-input-content")
                    .state(&self.state)
                    .range(0.0, 100000.0)
                    .size(MoonSize::Sm)
                    .unit("px"),
            ),
        )
    }
}

/// Catches the control collapsing in a content-sized row (the live Settings bug: an empty box a
/// few pixels wide between - and +). The oracle is independent of the floor arithmetic: two Sm
/// buttons are 24px each and six digits at 14px are well over 40px, so anything under 88px
/// cannot show the value.
#[gpui::test]
fn a_content_sized_number_input_keeps_room_for_its_value(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init);
    let window = cx.add_window(|window, cx| {
        let state = cx.new(|cx| MoonInputState::new(window, cx).default_value("5"));
        let view = cx.new(|_| ContentSizedHarness { state });
        crate::Root::new(view, window, cx).bordered(false)
    });
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();
    let bounds = cx
        .debug_bounds("probe-number")
        .expect("number input is laid out");
    assert!(
        f32::from(bounds.size.width) >= 88.0,
        "number input is {} px wide",
        f32::from(bounds.size.width)
    );
}
