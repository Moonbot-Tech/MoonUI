//! Moon date **and** time picking inside a single control.
//!
//! [`MoonDatePicker`](super::MoonDatePicker) is a Mirror of the Longbridge date picker: it owns
//! date-only selection and must keep zero donor drift. This module adds the Moon-owned variant
//! that keeps **one** field and **one** popup while letting the user pick the clock time right
//! under the calendar, so an application never has to place a second, standalone time control
//! beside the date field.
//!
//! The field is drawn by the shared [`moon_picker_field`], the same chain the donor date picker
//! uses, so the two controls are visually one family. The time half is the drum pair from
//! [`super::time_wheel`], shared with [`MoonTimePicker`](super::MoonTimePicker).
//!
//! The popup deliberately stays open after a day is clicked — the value is not complete until the
//! time is set too — and closes on an outside click, on the field, or through
//! [`MoonDateTimePickerState::set_open`].

use chrono::{NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use gpui::*;

use crate::{IconName, Size};

use crate::time::calendar::{Calendar, CalendarEvent, CalendarState, Date};

use super::{
    foundation::v_flex,
    picker_field::{MoonPickerFieldTrailing, moon_picker_field},
    popover::{MoonPopover, MoonPopoverChrome, MoonPopoverPlacement},
    theme::MoonTheme,
    time_picker::normalized_minute_step,
    time_wheel::{
        HOURS_PER_DAY, MINUTES_PER_HOUR, MoonWheelMove, moon_time_wheel_pair, wheel_steps,
        wrap_value,
    },
    tokens::rgba_from,
};

/// Rendered format of the date half of the field label.
const DEFAULT_DATE_FORMAT: &str = "%Y/%m/%d";
/// Rendered format of the time half of the field label.
const DEFAULT_TIME_FORMAT: &str = "%H:%M";

/// Join the picked day and the picked clock time into one value.
///
/// Args:
///     date: Selected day, or `None` while the calendar has no selection.
///     time: Selected clock time, which always has a value.
///
/// Returns:
///     The combined timestamp, or `None` while no day is selected.
pub(crate) fn compose(date: Option<NaiveDate>, time: NaiveTime) -> Option<NaiveDateTime> {
    date.map(|date| date.and_time(time))
}

/// Render the field label for a picked value.
///
/// Time is shown next to the date so the control reads as one value rather than as a date with a
/// hidden extra field.
///
/// Args:
///     date: Selected day, or `None` while the calendar has no selection.
///     time: Selected clock time.
///     date_format: `chrono` format for the date half.
///     time_format: `chrono` format for the time half.
///
/// Returns:
///     The formatted label, or `None` while no day is selected.
pub(crate) fn format_value(
    date: Option<NaiveDate>,
    time: NaiveTime,
    date_format: &str,
    time_format: &str,
) -> Option<SharedString> {
    date.map(|date| {
        SharedString::from(format!(
            "{} {}",
            date.format(date_format),
            time.format(time_format)
        ))
    })
}

/// Events emitted by [`MoonDateTimePickerState`].
pub enum MoonDateTimePickerEvent {
    /// The picked value changed.
    ///
    /// Carries `None` while the calendar has no day selected — a time-only change still emits, so
    /// a consumer that mirrors the state never drifts out of sync with the popup.
    Change(Option<NaiveDateTime>),
}

/// State of a [`MoonDateTimePicker`].
///
/// Owns the day, the clock time and the open flag, and drives the mirrored calendar state so the
/// Longbridge calendar keeps rendering the selection it already knows how to render.
pub struct MoonDateTimePickerState {
    focus_handle: FocusHandle,
    calendar: Entity<CalendarState>,
    date: Option<NaiveDate>,
    time: NaiveTime,
    open: bool,
    date_format: SharedString,
    time_format: SharedString,
    minute_step: u32,
    hour_carry: f32,
    minute_carry: f32,
    _subscriptions: Vec<Subscription>,
}

impl Focusable for MoonDateTimePickerState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<MoonDateTimePickerEvent> for MoonDateTimePickerState {}

impl Render for MoonDateTimePickerState {
    /// Render nothing: the state is a value holder driven by [`MoonDateTimePicker`].
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

impl MoonDateTimePickerState {
    /// Create an empty date-time picker state starting at midnight.
    ///
    /// Args:
    ///     window: Window used to subscribe to the internal calendar.
    ///     cx: Context of the new state entity.
    ///
    /// Returns:
    ///     A state with no day selected and the clock at `00:00`.
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let calendar = cx.new(|cx| CalendarState::new(window, cx));

        let _subscriptions = vec![cx.subscribe_in(
            &calendar,
            window,
            |this, _, event: &CalendarEvent, window, cx| match event {
                CalendarEvent::Selected(date) => {
                    if let Some(day) = date.start() {
                        this.select_day(day, window, cx);
                    }
                }
            },
        )];

        Self {
            focus_handle: cx.focus_handle(),
            calendar,
            date: None,
            time: NaiveTime::MIN,
            open: false,
            date_format: DEFAULT_DATE_FORMAT.into(),
            time_format: DEFAULT_TIME_FORMAT.into(),
            minute_step: 1,
            hour_carry: 0.0,
            minute_carry: 0.0,
            _subscriptions,
        }
    }

    /// Set the `chrono` format of the date half of the field label, default `"%Y/%m/%d"`.
    pub fn date_format(mut self, format: impl Into<SharedString>) -> Self {
        self.date_format = format.into();
        self
    }

    /// Set the `chrono` format of the time half of the field label, default `"%H:%M"`.
    pub fn time_format(mut self, format: impl Into<SharedString>) -> Self {
        self.time_format = format.into();
        self
    }

    /// Set how many minutes one minute-drum row covers, default `1`.
    ///
    /// Args:
    ///     step: Requested step; `0` is raised to `1` so the drum always moves.
    pub fn minute_step(mut self, step: u32) -> Self {
        self.minute_step = normalized_minute_step(step);
        self
    }

    /// The combined value, or `None` while no day is selected.
    pub fn value(&self) -> Option<NaiveDateTime> {
        compose(self.date, self.time)
    }

    /// The selected day, or `None` while the calendar has no selection.
    pub fn date(&self) -> Option<NaiveDate> {
        self.date
    }

    /// The selected clock time, which always has a value.
    pub fn time(&self) -> NaiveTime {
        self.time
    }

    /// Whether the calendar popup is open.
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Replace the whole value and re-sync the calendar.
    ///
    /// Args:
    ///     value: New timestamp, or `None` to clear the day and reset the clock to midnight.
    ///     window: Window forwarded to the calendar.
    ///     cx: Context used to emit [`MoonDateTimePickerEvent::Change`].
    pub fn set_value(
        &mut self,
        value: Option<NaiveDateTime>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.date = value.map(|value| value.date());
        self.time = value.map(|value| value.time()).unwrap_or(NaiveTime::MIN);
        self.sync_calendar(window, cx);
        self.emit_change(cx);
    }

    /// Replace the clock time, keeping the selected day.
    ///
    /// Args:
    ///     time: New clock time.
    ///     cx: Context used to emit [`MoonDateTimePickerEvent::Change`].
    pub fn set_time(&mut self, time: NaiveTime, cx: &mut Context<Self>) {
        self.time = time;
        self.emit_change(cx);
    }

    /// Open or close the popup without changing the value.
    ///
    /// Args:
    ///     open: Requested open state.
    ///     cx: Context used to request a repaint.
    pub fn set_open(&mut self, open: bool, cx: &mut Context<Self>) {
        if self.open == open {
            return;
        }
        self.open = open;
        if !open {
            self.hour_carry = 0.0;
            self.minute_carry = 0.0;
        }
        cx.notify();
    }

    /// Clear the day and reset the clock to midnight.
    ///
    /// Args:
    ///     window: Window forwarded to the calendar.
    ///     cx: Context used to emit [`MoonDateTimePickerEvent::Change`].
    pub fn clear(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.set_value(None, window, cx);
    }

    /// Apply a day picked in the calendar, keeping the popup open for the time drums.
    fn select_day(&mut self, day: NaiveDate, window: &mut Window, cx: &mut Context<Self>) {
        self.date = Some(day);
        self.sync_calendar(window, cx);
        self.emit_change(cx);
    }

    /// Spin the hour drum by one wheel event.
    ///
    /// A discrete notch is one hour, whatever the platform's lines-per-notch setting says;
    /// continuous movement accumulates until it adds up to a whole row.
    pub(crate) fn wheel_hours(&mut self, movement: MoonWheelMove, cx: &mut Context<Self>) {
        match movement {
            MoonWheelMove::Notches(notches) if notches != 0 => self.step_hours(notches, cx),
            MoonWheelMove::Notches(_) => {}
            MoonWheelMove::Rows(rows) => {
                let (steps, carry) = wheel_steps(rows, self.hour_carry);
                self.hour_carry = carry;
                if steps != 0 {
                    self.step_hours(steps, cx);
                }
            }
        }
    }

    /// Spin the minute drum by one wheel event.
    pub(crate) fn wheel_minutes(&mut self, movement: MoonWheelMove, cx: &mut Context<Self>) {
        match movement {
            MoonWheelMove::Notches(notches) if notches != 0 => self.step_minutes(notches, cx),
            MoonWheelMove::Notches(_) => {}
            MoonWheelMove::Rows(rows) => {
                let (steps, carry) = wheel_steps(rows, self.minute_carry);
                self.minute_carry = carry;
                if steps != 0 {
                    self.step_minutes(steps, cx);
                }
            }
        }
    }

    /// Walk the hour drum by whole rows, wrapping at midnight.
    pub(crate) fn step_hours(&mut self, steps: i32, cx: &mut Context<Self>) {
        let hour = wrap_value(self.time.hour() as i64 + steps as i64, HOURS_PER_DAY);
        self.set_hour(hour, cx);
    }

    /// Walk the minute drum by whole rows of `minute_step`, wrapping at the hour.
    pub(crate) fn step_minutes(&mut self, steps: i32, cx: &mut Context<Self>) {
        let step = self.resolved_minute_step() as i64;
        let minute = wrap_value(
            self.time.minute() as i64 + steps as i64 * step,
            MINUTES_PER_HOUR,
        );
        self.set_minute(minute, cx);
    }

    /// Select an hour directly.
    pub(crate) fn set_hour(&mut self, hour: u32, cx: &mut Context<Self>) {
        let Some(time) = self.time.with_hour(hour.min(HOURS_PER_DAY - 1)) else {
            return;
        };
        self.time = time;
        self.emit_change(cx);
    }

    /// Select a minute directly.
    pub(crate) fn set_minute(&mut self, minute: u32, cx: &mut Context<Self>) {
        let Some(time) = self.time.with_minute(minute.min(MINUTES_PER_HOUR - 1)) else {
            return;
        };
        self.time = time;
        self.emit_change(cx);
    }

    /// The minute step actually used by the drum.
    pub(crate) fn resolved_minute_step(&self) -> u32 {
        normalized_minute_step(self.minute_step)
    }

    /// Push the current day into the mirrored calendar state.
    fn sync_calendar(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let date = Date::Single(self.date);
        self.calendar.update(cx, |state, cx| {
            state.set_date(date, window, cx);
        });
    }

    /// Emit the current value and request a repaint of the field.
    fn emit_change(&mut self, cx: &mut Context<Self>) {
        cx.emit(MoonDateTimePickerEvent::Change(self.value()));
        cx.notify();
    }
}

/// A single control that picks a day and a clock time in one popup.
#[derive(IntoElement)]
pub struct MoonDateTimePicker {
    id: SharedString,
    state: Entity<MoonDateTimePickerState>,
    placeholder: SharedString,
    number_of_months: usize,
    size: Size,
    disabled: bool,
    cleanable: bool,
    width: Option<f32>,
}

impl MoonDateTimePicker {
    /// Create a picker bound to its state.
    ///
    /// Args:
    ///     id: Stable identity for the field, popup and drums.
    ///     state: The state entity holding the value and the open flag.
    ///
    /// Returns:
    ///     A default picker builder showing one month.
    pub fn new(id: impl Into<SharedString>, state: &Entity<MoonDateTimePickerState>) -> Self {
        Self {
            id: id.into(),
            state: state.clone(),
            placeholder: SharedString::from("Select date and time"),
            number_of_months: 1,
            size: Size::default(),
            disabled: false,
            cleanable: false,
            width: None,
        }
    }

    /// Set the text shown while no day is selected.
    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Set how many months the calendar shows, default `1`.
    pub fn number_of_months(mut self, number_of_months: usize) -> Self {
        self.number_of_months = number_of_months.max(1);
        self
    }

    /// Set the control size, shared with the date picker's input metrics.
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Show the clear button while the control holds a value, like the date picker's `cleanable`.
    pub fn cleanable(mut self, cleanable: bool) -> Self {
        self.cleanable = cleanable;
        self
    }

    /// Disable the control: the field stops opening the popup.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set an explicit rendered field width; by default the field fills its parent.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Render the picker.
    pub fn render(self) -> impl IntoElement {
        self
    }
}

impl RenderOnce for MoonDateTimePicker {
    /// Render the field plus the calendar-and-drums popup.
    ///
    /// Args:
    ///     window: Window owning the popover overlay.
    ///     cx: Application context used to resolve theme tokens and read the state.
    ///
    /// Returns:
    ///     The rendered single control.
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = MoonTheme::active_tokens(cx);
        let p = tokens.palette;
        let disabled = self.disabled;
        // The popover is fully controlled, so it does not repaint the hosting view itself.
        let parent_view = window.current_view();

        let state = self.state.read(cx);
        let calendar = state.calendar.clone();
        let open = state.open;
        let hour = state.time.hour();
        let minute = state.time.minute();
        let minute_step = state.resolved_minute_step();
        let focused = state.focus_handle.contains_focused(window, cx);
        let label = format_value(
            state.date,
            state.time,
            &state.date_format,
            &state.time_format,
        );
        let has_value = label.is_some();
        let label = label.unwrap_or_else(|| self.placeholder.clone());

        let trailing = if self.cleanable && has_value && !disabled {
            let state = self.state.clone();
            MoonPickerFieldTrailing::Clear(Box::new(move |_, window, cx| {
                state.update(cx, |this, cx| this.clear(window, cx));
                cx.stop_propagation();
                cx.notify(parent_view);
            }))
        } else {
            MoonPickerFieldTrailing::Icon(IconName::Calendar)
        };

        let mut field = moon_picker_field(
            ElementId::from(SharedString::from(format!("{}:field", self.id))),
            label,
            has_value,
            trailing,
            self.size,
            disabled,
            focused,
            true,
            cx,
        );
        field = match self.width {
            Some(width) => field.w(px(width)),
            None => field.w_full(),
        };

        let drums = moon_time_wheel_pair(
            self.id.clone(),
            hour,
            minute,
            minute_step,
            disabled,
            cx,
            {
                let state = self.state.clone();
                move |lines, _, cx| {
                    state.update(cx, |this, cx| this.wheel_hours(lines, cx));
                    cx.notify(parent_view);
                }
            },
            {
                let state = self.state.clone();
                move |value, _, cx| {
                    state.update(cx, |this, cx| this.set_hour(value, cx));
                    cx.notify(parent_view);
                }
            },
            {
                let state = self.state.clone();
                move |lines, _, cx| {
                    state.update(cx, |this, cx| this.wheel_minutes(lines, cx));
                    cx.notify(parent_view);
                }
            },
            {
                let state = self.state.clone();
                move |value, _, cx| {
                    state.update(cx, |this, cx| this.set_minute(value, cx));
                    cx.notify(parent_view);
                }
            },
        );

        let content = v_flex()
            .child(
                Calendar::new(&calendar)
                    .number_of_months(self.number_of_months)
                    .border_0()
                    .rounded_none()
                    .p_0(),
            )
            .child(
                div()
                    .my(px(tokens.ui(6.0)))
                    .h(px(tokens.ui(1.0)))
                    .w_full()
                    .bg(rgba_from(p.border_soft, 0.9)),
            )
            // The popup column is auto-sized by the calendar, so the drums are stretched across it
            // rather than asked for a percentage of an indefinite width.
            .child(drums.self_stretch());

        MoonPopover::new(self.id.clone())
            .placement(MoonPopoverPlacement::BottomStart)
            // Same popup surface as the Mirror date picker standing next to it.
            .chrome(MoonPopoverChrome::Picker)
            .fit_content()
            .disabled(disabled)
            .open(open)
            .close_on_content_click(false)
            .trigger(field)
            .content(content)
            .on_open_change({
                let state = self.state.clone();
                move |open, _, cx| {
                    state.update(cx, |this, cx| this.set_open(open, cx));
                    cx.notify(parent_view);
                }
            })
    }
}

#[cfg(test)]
mod tests;
