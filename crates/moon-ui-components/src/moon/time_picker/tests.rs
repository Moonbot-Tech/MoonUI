//! Regression coverage for the standalone time picker.

use chrono::{NaiveTime, Timelike};
use gpui::{App, AppContext as _, Context, Empty, IntoElement, Render, TestAppContext, Window};

use super::{MoonTimePickerState, format_hh_mm, normalized_minute_step};
use crate::moon::time_wheel::MoonWheelMove;

/// Minimal window root: the picker state is driven directly, nothing has to be rendered.
struct TimePickerTestHarness;

impl Render for TimePickerTestHarness {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

/// Build a time without going through the drums.
fn at(hour: u32, minute: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(hour, minute, 0).expect("valid test time")
}

/// Catches dropping the zero padding or the 24-hour format in `time_picker.rs:format_hh_mm`, which
/// is the exact output shape this control promises its callers.
#[test]
fn the_value_reads_back_as_zero_padded_hh_mm() {
    assert_eq!(format_hh_mm(at(9, 5)), "09:05");
    assert_eq!(format_hh_mm(at(0, 0)), "00:00");
    assert_eq!(format_hh_mm(at(23, 59)), "23:59");
}

/// Catches passing a caller's step through unchecked in
/// `time_picker.rs:normalized_minute_step`, which would leave a drum that renders but never moves.
#[test]
fn a_zero_minute_step_still_moves_the_drum() {
    assert_eq!(normalized_minute_step(0), 1);
    assert_eq!(normalized_minute_step(15), 15);
    assert_eq!(normalized_minute_step(600), 60);
}

/// Catches clamping instead of wrapping in `time_picker.rs:step_hours` / `step_minutes`. A drum
/// that sticks at `00` or `23` is the difference between a phone-style picker and a broken one.
#[gpui::test]
fn spinning_past_an_end_wraps_around(cx: &mut TestAppContext) {
    let window = cx.add_window(|_, _| TimePickerTestHarness);
    cx.update_window(window.into(), |_, _, cx: &mut App| {
        let state = cx.new(|cx| MoonTimePickerState::new(cx));

        state.update(cx, |this, cx| {
            this.set_value(at(0, 0), cx);
            this.step_hours(-1, cx);
        });
        assert_eq!(state.read(cx).value().hour(), 23);

        state.update(cx, |this, cx| {
            this.set_value(at(23, 59), cx);
            this.step_minutes(1, cx);
        });
        assert_eq!(state.read(cx).value().minute(), 0);
        assert_eq!(
            state.read(cx).value().hour(),
            23,
            "the minute drum must not carry into the hour drum"
        );
    })
    .expect("test window is open");
}

/// Catches ignoring `minute_step` in `time_picker.rs:step_minutes`, which would make a
/// five-minute picker walk one minute at a time and disagree with the rows it draws.
#[gpui::test]
fn the_minute_drum_walks_whole_steps(cx: &mut TestAppContext) {
    let window = cx.add_window(|_, _| TimePickerTestHarness);
    cx.update_window(window.into(), |_, _, cx: &mut App| {
        let state = cx.new(|cx| MoonTimePickerState::new(cx).minute_step(5));

        state.update(cx, |this, cx| {
            this.set_value(at(10, 0), cx);
            this.step_minutes(2, cx);
        });

        assert_eq!(state.read(cx).text(), "10:10");
    })
    .expect("test window is open");
}

/// Catches dropping the wheel carry in `time_picker.rs:wheel_minutes`: a trackpad sends sub-row
/// deltas, and without accumulation the drum would never move for those users.
#[gpui::test]
fn sub_row_wheel_movement_eventually_moves_the_drum(cx: &mut TestAppContext) {
    let window = cx.add_window(|_, _| TimePickerTestHarness);
    cx.update_window(window.into(), |_, _, cx: &mut App| {
        let state = cx.new(|cx| MoonTimePickerState::new(cx));

        state.update(cx, |this, cx| {
            this.set_value(at(10, 0), cx);
            this.wheel_minutes(MoonWheelMove::Rows(0.5), cx);
        });
        assert_eq!(
            state.read(cx).text(),
            "10:00",
            "half a row is not a step yet"
        );

        state.update(cx, |this, cx| {
            this.wheel_minutes(MoonWheelMove::Rows(0.6), cx)
        });
        assert_eq!(state.read(cx).text(), "10:01");
    })
    .expect("test window is open");
}

/// Catches a notch being routed through the sub-row accumulator in
/// `time_picker.rs:wheel_minutes`, which would make one wheel click move a fraction of a minute —
/// or several minutes — instead of exactly one.
#[gpui::test]
fn one_wheel_notch_changes_exactly_one_minute(cx: &mut TestAppContext) {
    let window = cx.add_window(|_, _| TimePickerTestHarness);
    cx.update_window(window.into(), |_, _, cx: &mut App| {
        let state = cx.new(|cx| MoonTimePickerState::new(cx));

        state.update(cx, |this, cx| {
            this.set_value(at(10, 0), cx);
            this.wheel_minutes(MoonWheelMove::Notches(1), cx);
        });
        assert_eq!(state.read(cx).text(), "10:01");

        state.update(cx, |this, cx| {
            this.wheel_minutes(MoonWheelMove::Notches(-1), cx);
            this.wheel_hours(MoonWheelMove::Notches(1), cx);
        });
        assert_eq!(state.read(cx).text(), "11:00");
    })
    .expect("test window is open");
}

/// Catches keeping a stale carry when the popup closes in `time_picker.rs:set_open`, which would
/// make the next opened drum jump on the first small scroll.
#[gpui::test]
fn closing_the_popup_forgets_partial_wheel_movement(cx: &mut TestAppContext) {
    let window = cx.add_window(|_, _| TimePickerTestHarness);
    cx.update_window(window.into(), |_, _, cx: &mut App| {
        let state = cx.new(|cx| MoonTimePickerState::new(cx));

        state.update(cx, |this, cx| {
            this.set_open(true, cx);
            this.wheel_hours(MoonWheelMove::Rows(0.9), cx);
            this.set_open(false, cx);
            this.set_open(true, cx);
            this.wheel_hours(MoonWheelMove::Rows(0.2), cx);
        });

        assert_eq!(
            state.read(cx).value().hour(),
            0,
            "carry from before the close must not complete a step after reopening"
        );
    })
    .expect("test window is open");
}
