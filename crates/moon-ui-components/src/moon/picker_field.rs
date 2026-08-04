//! The trigger field shared by every Moon picker.
//!
//! [`MoonDatePicker`](super::MoonDatePicker) is a Mirror of the Longbridge date picker, so its
//! trigger is drawn by donor code in `time/date_picker.rs`. Any Moon picker that stands next to it
//! in a form — the date+time picker, the time picker — must not invent its own field, or the row
//! ends up with two backgrounds, two borders and two heights. This module draws exactly the donor
//! chain, so the fields are the same control with a different payload.

use gpui::{prelude::FluentBuilder as _, *};

use crate::{
    ActiveTheme, Icon, IconName, Sizable as _, Size, StyleSized as _, StyledExt as _, h_flex,
    input::{clear_button, input_style},
};

/// The trailing affordance of a picker field.
pub(crate) enum MoonPickerFieldTrailing {
    /// Static icon shown when there is nothing to clear.
    Icon(IconName),
    /// Clear button; the handler receives the originating click.
    Clear(Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>),
}

/// Render a picker trigger that matches the Longbridge date-picker input.
///
/// The style chain is deliberately identical to `time/date_picker.rs`: same `input_style` pair,
/// same `cx.theme().input` border, same radius and shadow, same `input_size`/`input_text_size`
/// scaling. Only the label and the trailing affordance differ.
///
/// Args:
///     id: Stable element id of the field.
///     label: Text to render; pass the placeholder when there is no value.
///     has_value: Whether `label` is a real value, which drives the muted placeholder color.
///     trailing: Icon or clear button drawn at the trailing edge.
///     size: Control size, forwarded to the shared input metrics.
///     disabled: Whether the field is disabled.
///     focused: Whether the owning control holds focus, which paints the focus border.
///     appearance: `false` renders the minimal, chrome-less variant.
///     cx: Application context used to resolve the active theme.
///
/// Returns:
///     The field element, ready to be used as a popover trigger.
// The argument list mirrors the donor's own trigger state; folding it into a struct would hide
// which knobs exist to keep the fields identical.
#[allow(clippy::too_many_arguments)]
pub(crate) fn moon_picker_field(
    id: impl Into<ElementId>,
    label: SharedString,
    has_value: bool,
    trailing: MoonPickerFieldTrailing,
    size: Size,
    disabled: bool,
    focused: bool,
    appearance: bool,
    cx: &App,
) -> Stateful<Div> {
    let (bg, fg) = input_style(disabled, cx);

    div()
        .id(id)
        .relative()
        .flex()
        .items_center()
        .justify_between()
        .when(appearance, |this| {
            this.bg(bg)
                .text_color(fg)
                .when(disabled, |this| this.opacity(0.5))
                .border_1()
                .border_color(cx.theme().input)
                .rounded(cx.theme().radius)
                .when(cx.theme().shadow, |this| this.shadow_xs())
                .when(focused, |this| this.focused_border(cx))
        })
        .overflow_hidden()
        .input_text_size(size)
        .input_size(size)
        .child(
            h_flex()
                .w_full()
                .items_center()
                .justify_between()
                .gap_1()
                .child(
                    div()
                        .w_full()
                        .overflow_hidden()
                        .when(!has_value, |this| {
                            this.text_color(cx.theme().muted_foreground)
                        })
                        .child(label),
                )
                .when(!disabled, |this| match trailing {
                    MoonPickerFieldTrailing::Icon(icon) => this.child(
                        Icon::new(icon)
                            .xsmall()
                            .text_color(cx.theme().muted_foreground),
                    ),
                    MoonPickerFieldTrailing::Clear(handler) => {
                        this.child(clear_button(cx).on_click(move |event, window, cx| {
                            handler(event, window, cx);
                        }))
                    }
                }),
        )
}
