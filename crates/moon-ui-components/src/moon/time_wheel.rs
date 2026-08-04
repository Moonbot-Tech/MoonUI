//! The scrollable value drum shared by the Moon time pickers.
//!
//! A drum shows a short window of values around the selected one, highlights the centre, and is
//! spun with the mouse wheel — the interaction people expect from a phone time picker. Two of them
//! side by side make an `hh:mm` editor: [`MoonTimePicker`](super::MoonTimePicker) uses a pair on
//! its own, [`MoonDateTimePicker`](super::MoonDateTimePicker) puts the same pair under its
//! calendar.

use std::rc::Rc;

use gpui::{prelude::FluentBuilder as _, *};

use super::{
    text::MoonText,
    theme::MoonTheme,
    tokens::{MoonPalette, rgba_from},
};

/// Design-reference height of one drum row; also the wheel distance that walks one value.
pub(crate) const WHEEL_ROW_HEIGHT: f32 = 22.0;
/// Design-reference minimum width of one drum column; the columns share the popup width above it.
pub(crate) const WHEEL_WIDTH: f32 = 52.0;
/// Design-reference corner radius of a drum row, matching a calendar day cell.
const WHEEL_ROW_RADIUS: f32 = 4.0;
/// Design-reference text size of a drum row, matching the theme body size the calendar inherits.
const WHEEL_FONT_SIZE: f32 = 12.0;
/// Design-reference line height of a drum row.
const WHEEL_LINE_HEIGHT: f32 = 15.0;
/// Rows drawn above and below the selected value.
const WHEEL_HALO: i32 = 2;

/// Wrap a drum position into `0..len`.
///
/// Drums are circular: stepping down from `00` reaches the last value, which is what a phone-style
/// picker does and what keeps a long spin from sticking at an end.
///
/// Args:
///     position: Possibly out-of-range position, including negatives.
///     len: Number of values on the drum; `0` is treated as `1`.
///
/// Returns:
///     The equivalent position inside `0..len`.
pub(crate) fn wrap_value(position: i64, len: u32) -> u32 {
    let len = len.max(1) as i64;
    position.rem_euclid(len) as u32
}

/// One wheel event translated into drum movement, in drum direction: positive walks forward.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum MoonWheelMove {
    /// A discrete mouse-wheel notch.
    ///
    /// Windows multiplies a notch by the system "lines per scroll" setting — three by default
    /// ([`SPI_GETWHEELSCROLLLINES`]) — so honouring the raw line count would jump the drum three
    /// values per notch. A value drum wants one notch to be one value, whatever that setting says.
    ///
    /// [`SPI_GETWHEELSCROLLLINES`]: https://learn.microsoft.com/windows/win32/api/winuser/nf-winuser-systemparametersinfow
    Notches(i32),
    /// Continuous movement in rows, as a trackpad delivers it.
    Rows(f32),
}

/// Translate a raw scroll delta into drum movement.
///
/// GPUI scroll offsets grow when the content moves down, so a positive delta is a scroll **up**
/// and must walk the drum backwards, the way every native picker behaves.
///
/// Args:
///     delta: Raw wheel delta from the scroll event.
///     row_height: Rendered height of one drum row, used to measure continuous movement.
///
/// Returns:
///     The movement this event asks for.
pub(crate) fn wheel_move(delta: &ScrollDelta, row_height: f32) -> MoonWheelMove {
    match delta {
        ScrollDelta::Lines(lines) => {
            let direction = if lines.y > 0.0 {
                -1
            } else if lines.y < 0.0 {
                1
            } else {
                0
            };
            MoonWheelMove::Notches(direction)
        }
        ScrollDelta::Pixels(pixels) => {
            let rows = f32::from(pixels.y) / row_height.max(1.0);
            MoonWheelMove::Rows(-rows)
        }
    }
}

/// Turn accumulated continuous movement into whole drum steps.
///
/// A trackpad delivers many sub-row deltas, so the remainder is carried to the next event instead
/// of being rounded away — otherwise slow scrolling never moves the drum at all.
///
/// Args:
///     rows: Continuous movement in rows, positive walking forward.
///     carry: Sub-row remainder left by the previous event.
///
/// Returns:
///     Whole steps to apply and the new remainder.
pub(crate) fn wheel_steps(rows: f32, carry: f32) -> (i32, f32) {
    if !rows.is_finite() {
        return (0, carry);
    }
    let total = carry + rows;
    let steps = total.trunc();
    (steps as i32, total - steps)
}

/// Render one drum column.
///
/// Args:
///     id: Stable element id of the column.
///     len: Number of values on the drum (`24` for hours, `60` for minutes).
///     selected: Currently selected value.
///     step: Distance between neighbouring rows, so a five-minute drum shows 05/10/15.
///     disabled: Whether interaction is suppressed.
///     cx: Application context used to resolve theme tokens.
///     on_wheel: Receives wheel movement in rows, positive for a scroll up.
///     on_pick: Receives the value of a clicked row.
///
/// Returns:
///     The column element.
#[allow(clippy::too_many_arguments)]
pub(crate) fn moon_time_wheel(
    id: impl Into<SharedString>,
    len: u32,
    selected: u32,
    step: u32,
    disabled: bool,
    cx: &App,
    on_wheel: impl Fn(MoonWheelMove, &mut Window, &mut App) + 'static,
    on_pick: impl Fn(u32, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let tokens = MoonTheme::active_tokens(cx);
    let p = MoonPalette::active(cx);
    let id: SharedString = id.into();
    let row_height = tokens.ui(WHEEL_ROW_HEIGHT);
    let step = step.max(1) as i64;
    let on_wheel = Rc::new(on_wheel);
    let on_pick = Rc::new(on_pick);

    // No surface of its own: the popup already paints one, and a second, darker box inside it is
    // exactly what the calendar above the drums does not do.
    let mut column = div()
        .id(ElementId::from(id.clone()))
        .relative()
        .flex_1()
        .min_w(px(tokens.ui(WHEEL_WIDTH)))
        .flex()
        .flex_col()
        .items_center()
        .overflow_hidden();

    if !disabled {
        column = column.on_scroll_wheel({
            let on_wheel = on_wheel.clone();
            move |event, window, cx| {
                let movement = wheel_move(&event.delta, row_height);
                if movement != MoonWheelMove::Notches(0) {
                    on_wheel(movement, window, cx);
                    // The drum lives inside a scrollable page: without this the page scrolls too.
                    cx.stop_propagation();
                }
            }
        });
    }

    for offset in -WHEEL_HALO..=WHEEL_HALO {
        let value = wrap_value(selected as i64 + offset as i64 * step, len);
        let is_selected = offset == 0;
        let distance = offset.unsigned_abs() as f32;
        let alpha = if disabled {
            0.35
        } else if is_selected {
            1.0
        } else {
            (0.62 - distance * 0.18).max(0.2)
        };

        column = column.child(
            div()
                .id(ElementId::from(SharedString::from(format!(
                    "{id}:row:{offset}"
                ))))
                .w_full()
                .h(px(row_height))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(tokens.ui(WHEEL_ROW_RADIUS)))
                // The selected row is painted like the calendar's selected day: solid accent with
                // the palette's selected foreground, so one popup does not hold two idioms.
                .when(is_selected, |this| this.bg(rgba_from(p.accent, 1.0)))
                .when(!disabled && !is_selected, |this| {
                    // Hover keeps the accent hue but stays translucent, so the row under the
                    // pointer never reads as the selected one.
                    this.cursor_pointer()
                        .hover(|this| this.bg(rgba_from(p.accent, 0.30)))
                        .on_click({
                            let on_pick = on_pick.clone();
                            move |_, window, cx| {
                                on_pick(value, window, cx);
                            }
                        })
                })
                .child(
                    MoonText::new(format!("{value:02}"))
                        .color(if is_selected {
                            // This row's fill IS the solid accent, unlike a tinted selected list
                            // row, so the ink is measured against it.
                            p.ink_on(p.accent)
                        } else {
                            p.text_muted
                        })
                        .alpha(alpha)
                        .font_size(WHEEL_FONT_SIZE)
                        .line_height(WHEEL_LINE_HEIGHT)
                        .weight(if is_selected { 600.0 } else { 400.0 })
                        .uppercase(false)
                        .render(),
                ),
        );
    }

    column
}

/// Render the `hh : mm` drum pair.
///
/// Both Moon time surfaces use this, so the hour and minute drums never drift apart in size,
/// spacing or separator.
///
/// Args:
///     id: Stable id prefix; the drums derive their own ids from it.
///     hour: Selected hour.
///     minute: Selected minute.
///     minute_step: Distance between neighbouring minute rows.
///     disabled: Whether interaction is suppressed.
///     cx: Application context used to resolve theme tokens.
///     on_hour_wheel / on_hour_pick: Hour drum callbacks, see [`moon_time_wheel`].
///     on_minute_wheel / on_minute_pick: Minute drum callbacks.
///
/// Returns:
///     The paired drums element.
#[allow(clippy::too_many_arguments)]
pub(crate) fn moon_time_wheel_pair(
    id: impl Into<SharedString>,
    hour: u32,
    minute: u32,
    minute_step: u32,
    disabled: bool,
    cx: &App,
    on_hour_wheel: impl Fn(MoonWheelMove, &mut Window, &mut App) + 'static,
    on_hour_pick: impl Fn(u32, &mut Window, &mut App) + 'static,
    on_minute_wheel: impl Fn(MoonWheelMove, &mut Window, &mut App) + 'static,
    on_minute_pick: impl Fn(u32, &mut Window, &mut App) + 'static,
) -> Div {
    let tokens = MoonTheme::active_tokens(cx);
    let p = MoonPalette::active(cx);
    let id: SharedString = id.into();

    // Width is the caller's call: `w_full` only resolves against a parent with a definite width,
    // and the date+time popup is auto-sized by its calendar, so that caller stretches the pair
    // instead. Getting it wrong here is what left the drums hugging the left edge.
    super::foundation::h_flex()
        .items_center()
        .justify_center()
        .gap(px(tokens.ui(6.0)))
        .child(moon_time_wheel(
            format!("{id}:hours"),
            HOURS_PER_DAY,
            hour,
            1,
            disabled,
            cx,
            on_hour_wheel,
            on_hour_pick,
        ))
        .child(
            MoonText::new(":")
                .color(p.text_muted)
                .alpha(if disabled { 0.45 } else { 1.0 })
                .font_size(WHEEL_FONT_SIZE)
                .line_height(WHEEL_LINE_HEIGHT)
                .weight(600.0)
                .uppercase(false)
                .render(),
        )
        .child(moon_time_wheel(
            format!("{id}:minutes"),
            MINUTES_PER_HOUR,
            minute,
            minute_step,
            disabled,
            cx,
            on_minute_wheel,
            on_minute_pick,
        ))
}

/// Values on the hour drum.
pub(crate) const HOURS_PER_DAY: u32 = 24;
/// Values on the minute drum.
pub(crate) const MINUTES_PER_HOUR: u32 = 60;

#[cfg(test)]
mod tests;
