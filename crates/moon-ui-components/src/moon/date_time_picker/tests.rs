//! Regression coverage for picking a day and a clock time in one control.

use chrono::{NaiveDate, NaiveTime, Timelike};
use gpui::{App, AppContext as _, Context, Empty, IntoElement, Render, TestAppContext, Window};

use super::{MoonDateTimePickerState, compose, format_value};
use crate::moon::time_wheel::MoonWheelMove;

/// Minimal window root: the picker state is driven directly, nothing has to be rendered.
struct DateTimePickerTestHarness;

impl Render for DateTimePickerTestHarness {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

/// Build a day without going through the calendar UI.
fn day(year: i32, month: u32, date: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, date).expect("valid test date")
}

/// Build a clock time without going through the drums.
fn at(hour: u32, minute: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(hour, minute, 0).expect("valid test time")
}

/// Catches defaulting the day in `date_time_picker.rs:compose`, which would report "today at
/// 00:00" as a real user selection before any day was picked.
#[test]
fn a_value_needs_a_day_not_only_a_time() {
    assert_eq!(compose(None, at(9, 30)), None);
    assert_eq!(
        compose(Some(day(2026, 8, 4)), at(9, 30)),
        Some(day(2026, 8, 4).and_time(at(9, 30)))
    );
}

/// Catches dropping the time half in `date_time_picker.rs:format_value`, which would render a
/// date-only field label and hide the very thing this control adds.
#[test]
fn the_field_label_shows_the_time_next_to_the_date() {
    assert_eq!(
        format_value(Some(day(2026, 8, 4)), at(9, 30), "%Y/%m/%d", "%H:%M").as_deref(),
        Some("2026/08/04 09:30")
    );
    assert_eq!(format_value(None, at(9, 30), "%Y/%m/%d", "%H:%M"), None);
}

/// Catches closing the popup on day selection in `date_time_picker.rs:select_day` (the behavior
/// the mirrored date-only picker has), which would dismiss the calendar before the user can reach
/// the time drums of the same control.
#[gpui::test]
fn picking_a_day_keeps_the_popup_open_for_the_time_drums(cx: &mut TestAppContext) {
    let window = cx.add_window(|_, _| DateTimePickerTestHarness);
    cx.update_window(window.into(), |_, window, cx: &mut App| {
        let state = cx.new(|cx| MoonDateTimePickerState::new(window, cx));

        state.update(cx, |this, cx| {
            this.set_open(true, cx);
            this.select_day(day(2026, 8, 4), window, cx);
        });

        let state = state.read(cx);
        assert!(
            state.is_open(),
            "the popup must stay open so the time drums remain reachable"
        );
        assert_eq!(state.date(), Some(day(2026, 8, 4)));
        assert_eq!(
            state.value(),
            Some(day(2026, 8, 4).and_time(NaiveTime::MIN))
        );
    })
    .expect("test window is open");
}

/// Catches resetting the day inside `date_time_picker.rs:set_hour` / `set_minute`, which would
/// throw away the calendar selection every time the user spins the clock.
#[gpui::test]
fn spinning_the_time_keeps_the_day(cx: &mut TestAppContext) {
    let window = cx.add_window(|_, _| DateTimePickerTestHarness);
    cx.update_window(window.into(), |_, window, cx: &mut App| {
        let state = cx.new(|cx| MoonDateTimePickerState::new(window, cx));

        state.update(cx, |this, cx| {
            this.select_day(day(2026, 8, 4), window, cx);
            this.set_hour(9, cx);
            this.set_minute(30, cx);
        });

        let state = state.read(cx);
        assert_eq!(state.date(), Some(day(2026, 8, 4)));
        assert_eq!(state.time().hour(), 9);
        assert_eq!(state.time().minute(), 30);
        assert_eq!(state.value(), Some(day(2026, 8, 4).and_time(at(9, 30))));
    })
    .expect("test window is open");
}

/// Catches clamping instead of wrapping in `date_time_picker.rs:step_hours` / `step_minutes`, and
/// catches letting the minute drum carry into the hour — a drum that silently changes the hour
/// while the user tunes minutes writes the wrong timestamp.
#[gpui::test]
fn the_drums_wrap_without_disturbing_the_day(cx: &mut TestAppContext) {
    let window = cx.add_window(|_, _| DateTimePickerTestHarness);
    cx.update_window(window.into(), |_, window, cx: &mut App| {
        let state = cx.new(|cx| MoonDateTimePickerState::new(window, cx));

        state.update(cx, |this, cx| {
            this.set_value(Some(day(2026, 8, 4).and_time(at(0, 59))), window, cx);
            this.step_hours(-1, cx);
            this.step_minutes(1, cx);
        });

        let state = state.read(cx);
        assert_eq!(state.time(), at(23, 0));
        assert_eq!(
            state.date(),
            Some(day(2026, 8, 4)),
            "wrapping the clock must not roll the calendar day"
        );
    })
    .expect("test window is open");
}

/// Catches clearing only the day in `date_time_picker.rs:clear`, which would leave a stale clock
/// time behind and re-apply it to the next day the user picks.
#[gpui::test]
fn clearing_resets_both_halves_of_the_value(cx: &mut TestAppContext) {
    let window = cx.add_window(|_, _| DateTimePickerTestHarness);
    cx.update_window(window.into(), |_, window, cx: &mut App| {
        let state = cx.new(|cx| MoonDateTimePickerState::new(window, cx));

        state.update(cx, |this, cx| {
            this.set_value(Some(day(2026, 8, 4).and_time(at(9, 30))), window, cx);
            this.clear(window, cx);
        });

        let state = state.read(cx);
        assert_eq!(state.date(), None);
        assert_eq!(state.time(), NaiveTime::MIN);
        assert_eq!(state.value(), None);
    })
    .expect("test window is open");
}

/// Catches mis-routing a wheel event in `date_time_picker.rs:wheel_hours` / `wheel_minutes`: a
/// notch must walk exactly one row and an empty notch none, and each drum must keep its own sub-row
/// trackpad carry until it adds up to whole rows. Otherwise one wheel click jumps several values, a
/// slow trackpad never moves the clock, or spinning the hours nudges the minutes.
///
/// Expected values by hand: 10:00 + one hour notch, minus one minute notch (wrapping within the
/// hour) = 11:59; hour carry 0.6 + 0.6 = 1.2 is one row → 12; minute carry 0.5 - 0.4 + 2.0 = 2.1 is
/// two rows → 59 + 2 wraps to 01.
#[gpui::test]
fn the_wheel_walks_each_drum_by_whole_rows(cx: &mut TestAppContext) {
    let window = cx.add_window(|_, _| DateTimePickerTestHarness);
    cx.update_window(window.into(), |_, window, cx: &mut App| {
        let state = cx.new(|cx| MoonDateTimePickerState::new(window, cx));

        state.update(cx, |this, cx| {
            this.set_value(Some(day(2026, 8, 4).and_time(at(10, 0))), window, cx);
            this.wheel_hours(MoonWheelMove::Notches(1), cx);
            this.wheel_minutes(MoonWheelMove::Notches(-1), cx);
            this.wheel_minutes(MoonWheelMove::Notches(0), cx);
        });
        assert_eq!(state.read(cx).time(), at(11, 59));

        state.update(cx, |this, cx| {
            this.wheel_hours(MoonWheelMove::Rows(0.6), cx);
            this.wheel_minutes(MoonWheelMove::Rows(0.5), cx);
        });
        assert_eq!(
            state.read(cx).time(),
            at(11, 59),
            "sub-row movement on either drum is not a step yet"
        );

        state.update(cx, |this, cx| {
            this.wheel_hours(MoonWheelMove::Rows(0.6), cx);
            this.wheel_minutes(MoonWheelMove::Rows(-0.4), cx);
            this.wheel_minutes(MoonWheelMove::Rows(2.0), cx);
        });
        assert_eq!(state.read(cx).time(), at(12, 1));
        assert_eq!(state.read(cx).date(), Some(day(2026, 8, 4)));
    })
    .expect("test window is open");
}
