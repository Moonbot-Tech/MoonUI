//! Time-only picking: one field, one popup, two spinnable drums, `hh:mm` out.
//!
//! This is the standalone sibling of [`MoonDatePicker`](super::MoonDatePicker) and
//! [`MoonDateTimePicker`](super::MoonDateTimePicker): the field is drawn by the same
//! [`moon_picker_field`] as theirs, so a form row of "date" and "time" looks like one control
//! split in two, and the popup is the drum pair from [`super::time_wheel`].
//!
//! The value always exists — a time picker with nothing selected has no useful meaning — and
//! reads back as `hh:mm` through [`MoonTimePickerState::text`].

use chrono::{NaiveTime, Timelike};
use gpui::*;

use crate::{IconName, Size};

use super::{
    picker_field::{MoonPickerFieldTrailing, moon_picker_field},
    popover::{MoonPopover, MoonPopoverChrome, MoonPopoverPlacement},
    time_wheel::{
        HOURS_PER_DAY, MINUTES_PER_HOUR, MoonWheelMove, moon_time_wheel_pair, wheel_steps,
        wrap_value,
    },
};

/// Rendered format of the trigger label.
const TIME_FORMAT: &str = "%H:%M";
/// Design-reference content width of the drum popup.
const TIME_POPUP_CONTENT_WIDTH: f32 = 150.0;

/// Format a time as the `hh:mm` the control promises.
///
/// Args:
///     time: Time to render.
///
/// Returns:
///     Zero-padded 24-hour text.
pub(crate) fn format_hh_mm(time: NaiveTime) -> SharedString {
    SharedString::from(time.format(TIME_FORMAT).to_string())
}

/// Clamp a caller's minute step into a step that can actually move the drum.
///
/// Args:
///     step: Requested step.
///
/// Returns:
///     The step clamped into `1..=60`.
pub(crate) fn normalized_minute_step(step: u32) -> u32 {
    step.clamp(1, MINUTES_PER_HOUR)
}

/// Events emitted by [`MoonTimePickerState`].
pub enum MoonTimePickerEvent {
    /// The picked time changed.
    Change(NaiveTime),
}

/// State of a [`MoonTimePicker`].
pub struct MoonTimePickerState {
    focus_handle: FocusHandle,
    time: NaiveTime,
    open: bool,
    minute_step: u32,
    hour_carry: f32,
    minute_carry: f32,
}

impl Focusable for MoonTimePickerState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<MoonTimePickerEvent> for MoonTimePickerState {}

impl Render for MoonTimePickerState {
    /// Render nothing: the state is a value holder driven by [`MoonTimePicker`].
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

impl MoonTimePickerState {
    /// Create a time state at midnight.
    ///
    /// Args:
    ///     cx: Context of the new state entity.
    ///
    /// Returns:
    ///     A state holding `00:00` with a closed popup.
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            time: NaiveTime::MIN,
            open: false,
            minute_step: 1,
            hour_carry: 0.0,
            minute_carry: 0.0,
        }
    }

    /// Set how many minutes one minute-drum row covers, default `1`.
    ///
    /// Args:
    ///     step: Requested step; `0` is raised to `1` so the drum always moves.
    pub fn minute_step(mut self, step: u32) -> Self {
        self.minute_step = normalized_minute_step(step);
        self
    }

    /// Start at a given time instead of midnight.
    pub fn default_value(mut self, time: NaiveTime) -> Self {
        self.time = time;
        self
    }

    /// The picked time.
    pub fn value(&self) -> NaiveTime {
        self.time
    }

    /// The picked time as `hh:mm`.
    pub fn text(&self) -> SharedString {
        format_hh_mm(self.time)
    }

    /// Whether the drum popup is open.
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Replace the picked time.
    ///
    /// Args:
    ///     time: New time; seconds are preserved as given.
    ///     cx: Context used to emit [`MoonTimePickerEvent::Change`].
    pub fn set_value(&mut self, time: NaiveTime, cx: &mut Context<Self>) {
        self.time = time;
        self.emit_change(cx);
    }

    /// Open or close the popup without changing the value.
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
        let step = normalized_minute_step(self.minute_step) as i64;
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

    /// Emit the current value and request a repaint of the trigger.
    fn emit_change(&mut self, cx: &mut Context<Self>) {
        cx.emit(MoonTimePickerEvent::Change(self.time));
        cx.notify();
    }
}

/// A time field whose popup holds two spinnable drums; the value reads back as `hh:mm`.
#[derive(IntoElement)]
pub struct MoonTimePicker {
    id: SharedString,
    state: Entity<MoonTimePickerState>,
    size: Size,
    disabled: bool,
    width: Option<f32>,
}

impl MoonTimePicker {
    /// Create a time picker bound to its state.
    ///
    /// Args:
    ///     id: Stable identity for the field, popup and drums.
    ///     state: The state entity holding the time and the open flag.
    ///
    /// Returns:
    ///     A default picker builder.
    pub fn new(id: impl Into<SharedString>, state: &Entity<MoonTimePickerState>) -> Self {
        Self {
            id: id.into(),
            state: state.clone(),
            size: Size::default(),
            disabled: false,
            width: None,
        }
    }

    /// Set the control size, shared with the date pickers' input metrics.
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
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

impl RenderOnce for MoonTimePicker {
    /// Render the field plus its drum popup.
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        // The popover is fully controlled, so it does not repaint the hosting view itself.
        let parent_view = window.current_view();
        let disabled = self.disabled;

        let state = self.state.read(cx);
        let open = state.open;
        let hour = state.time.hour();
        let minute = state.time.minute();
        let minute_step = state.resolved_minute_step();
        let label = format_hh_mm(state.time);
        let focused = state.focus_handle.contains_focused(window, cx);

        let mut field = moon_picker_field(
            ElementId::from(SharedString::from(format!("{}:field", self.id))),
            label,
            true,
            MoonPickerFieldTrailing::Icon(IconName::ChevronsUpDown),
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

        MoonPopover::new(self.id.clone())
            .placement(MoonPopoverPlacement::BottomStart)
            // Same popup surface as the date pickers standing next to it.
            .chrome(MoonPopoverChrome::Picker)
            // The drums stretch to the popup, so the popup owns the width here; without a calendar
            // above them there is nothing else to derive it from.
            .content_width_ui(TIME_POPUP_CONTENT_WIDTH)
            .disabled(disabled)
            .open(open)
            .close_on_content_click(false)
            .trigger(field)
            // The popup owns a definite width here, so a plain full-width pair is enough.
            .content(drums.w_full())
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
