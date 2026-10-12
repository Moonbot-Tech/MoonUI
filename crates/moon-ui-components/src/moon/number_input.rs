//! Typed numeric entry with - / + buttons, Up/Down stepping, range clamping and a unit suffix.

use gpui::*;

use crate::input::{NumberInput, NumberStep};
use crate::{Disableable as _, Sizable as _, Size};

use super::{
    foundation::MoonSize, input::MoonInputState, text::MoonText, theme::MoonTheme, tokens::MoonRect,
};

/// Moon numeric input: a typed value between a - and a + button, with an optional unit suffix.
///
/// The value lives in a [`MoonInputState`]. Pass one with [`Self::state`] to read the value and
/// subscribe to [`super::MoonInputEvent::Change`]; without it the control keeps a keyed state of
/// its own, seeded from [`Self::default_value`]. Up/Down step the value while the field is
/// focused, the buttons step it on click, and both clamp to [`Self::min`] / [`Self::max`]. A typed
/// out-of-range value is clamped when the field loses focus.
#[derive(IntoElement)]
pub struct MoonNumberInput {
    id: SharedString,
    state: Option<Entity<MoonInputState>>,
    bounds: Option<MoonRect>,
    default_value: Option<f64>,
    placeholder: SharedString,
    min: Option<f64>,
    max: Option<f64>,
    step: f64,
    unit: Option<SharedString>,
    size: Option<MoonSize>,
    disabled: bool,
}

impl MoonNumberInput {
    /// Create an unbounded number input stepping by 1 that follows the active density tier.
    ///
    /// Args:
    ///     id: Stable identity; also keys the control's own state when none is passed.
    ///
    /// Returns:
    ///     A default number-input builder.
    pub fn new(id: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            state: None,
            bounds: None,
            default_value: None,
            placeholder: SharedString::default(),
            min: None,
            max: None,
            step: 1.0,
            unit: None,
            size: None,
            disabled: false,
        }
    }

    /// Bind the control to a caller-owned input state.
    ///
    /// Args:
    ///     state: State holding the typed text; [`Self::read_value`] parses it.
    ///
    /// Returns:
    ///     The updated builder.
    pub fn state(mut self, state: &Entity<MoonInputState>) -> Self {
        self.state = Some(state.clone());
        self
    }

    /// Position the control absolutely.
    ///
    /// Args:
    ///     bounds: Rendered-pixel rectangle.
    ///
    /// Returns:
    ///     The updated builder.
    pub fn bounds(mut self, bounds: MoonRect) -> Self {
        self.bounds = Some(bounds);
        self
    }

    /// Seed the control's own state; ignored when a state is passed with [`Self::state`].
    ///
    /// Args:
    ///     value: Initial value, clamped to the configured range.
    ///
    /// Returns:
    ///     The updated builder.
    pub fn default_value(mut self, value: f64) -> Self {
        self.default_value = Some(value);
        self
    }

    /// Set the placeholder of the control's own state.
    ///
    /// Args:
    ///     placeholder: Text shown while the field is empty.
    ///
    /// Returns:
    ///     The updated builder.
    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Set the lowest accepted value.
    ///
    /// Args:
    ///     min: Inclusive lower bound.
    ///
    /// Returns:
    ///     The updated builder.
    pub fn min(mut self, min: f64) -> Self {
        self.min = Some(min);
        self
    }

    /// Set the highest accepted value.
    ///
    /// Args:
    ///     max: Inclusive upper bound.
    ///
    /// Returns:
    ///     The updated builder.
    pub fn max(mut self, max: f64) -> Self {
        self.max = Some(max);
        self
    }

    /// Set both bounds at once.
    ///
    /// Args:
    ///     min: Inclusive lower bound.
    ///     max: Inclusive upper bound.
    ///
    /// Returns:
    ///     The updated builder.
    pub fn range(self, min: f64, max: f64) -> Self {
        self.min(min).max(max)
    }

    /// Set the amount one button press or one Up/Down key moves the value.
    ///
    /// Args:
    ///     step: Step size; non-positive or non-finite values fall back to 1.
    ///
    /// Returns:
    ///     The updated builder.
    pub fn step(mut self, step: f64) -> Self {
        self.step = normalize_step(step);
        self
    }

    /// Show a unit after the value, such as `%` or `ms`.
    ///
    /// Args:
    ///     unit: Unit label.
    ///
    /// Returns:
    ///     The updated builder.
    pub fn unit(mut self, unit: impl Into<SharedString>) -> Self {
        self.unit = Some(unit.into());
        self
    }

    /// Override the density tier; Xs, Sm and Md are supported and larger tiers snap to Md.
    ///
    /// Args:
    ///     size: Requested tier.
    ///
    /// Returns:
    ///     The updated builder.
    pub fn size(mut self, size: MoonSize) -> Self {
        self.size = Some(size);
        self
    }

    /// Disable typing and both buttons.
    ///
    /// Args:
    ///     disabled: Whether the control is disabled.
    ///
    /// Returns:
    ///     The updated builder.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Parse the number held by a number-input state.
    ///
    /// Args:
    ///     state: State bound with [`Self::state`].
    ///     cx: Application context.
    ///
    /// Returns:
    ///     The value, or `None` while the field is empty or holds a partial entry such as `-`.
    pub fn read_value(state: &Entity<MoonInputState>, cx: &App) -> Option<f64> {
        parse_number(state.read(cx).value().as_ref())
    }
}

/// Replace a step the base input cannot walk with the default step of 1.
///
/// Args:
///     step: Requested step.
///
/// Returns:
///     `step` when it is finite and positive, otherwise `1.0`.
fn normalize_step(step: f64) -> f64 {
    if step.is_finite() && step > 0.0 {
        step
    } else {
        1.0
    }
}

/// Parse typed text as a number.
///
/// Args:
///     text: Field text.
///
/// Returns:
///     The parsed finite value, or `None` for empty, partial or non-finite text.
fn parse_number(text: &str) -> Option<f64> {
    text.trim()
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
}

/// Clamp a seed value into the optional range.
///
/// Args:
///     value: Requested value.
///     min: Optional inclusive lower bound.
///     max: Optional inclusive upper bound.
///
/// Returns:
///     The value inside the range; the lower bound wins when the range is inverted.
fn clamp_to_range(value: f64, min: Option<f64>, max: Option<f64>) -> f64 {
    let value = max.map_or(value, |max| value.min(max));
    min.map_or(value, |min| value.max(min))
}

/// Map a density tier onto the base input size the control renders at.
///
/// Args:
///     tier: Requested or active tier.
///
/// Returns:
///     The base size of the field and its buttons.
fn base_size_for(tier: MoonSize) -> Size {
    match tier.nearest(&[MoonSize::Xs, MoonSize::Sm, MoonSize::Md]) {
        MoonSize::Xs => Size::XSmall,
        MoonSize::Sm => Size::Small,
        _ => Size::Medium,
    }
}

impl RenderOnce for MoonNumberInput {
    /// Render the base number input with Moon sizing, range and unit suffix.
    ///
    /// Args:
    ///     window: Window that owns the keyed state.
    ///     cx: Application context.
    ///
    /// Returns:
    ///     The rendered control.
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = MoonTheme::active_tokens(cx);
        let p = tokens.palette;
        let tier = self.size.unwrap_or_else(|| tokens.tier());
        let (min, max, step) = (self.min, self.max, self.step);
        let seed = self
            .default_value
            .map(|value| clamp_to_range(value, min, max).to_string())
            .unwrap_or_default();
        let placeholder = self.placeholder.clone();
        let state = self.state.unwrap_or_else(|| {
            window.use_keyed_state(
                ElementId::from(SharedString::from(format!("{}:state", self.id))),
                cx,
                move |window, cx| {
                    MoonInputState::new(window, cx)
                        .placeholder(placeholder)
                        .default_value(seed)
                },
            )
        });
        // Re-applied every render so a changed builder range reaches an existing state.
        state.update(cx, |state, cx| {
            state.set_step(NumberStep::Fixed(step), window, cx);
            state.set_min(min, window, cx);
            state.set_max(max, window, cx);
        });

        let metrics = tier
            .nearest(&[MoonSize::Xs, MoonSize::Sm, MoonSize::Md])
            .control_metrics();
        let mut input = NumberInput::new(&state)
            .with_size(base_size_for(tier))
            .disabled(self.disabled);
        if let Some(unit) = self.unit {
            input = input.suffix(
                MoonText::new(unit)
                    .color(p.text_muted)
                    .font_size(metrics.font_size - 2.0)
                    .line_height(metrics.line_height)
                    .mono(true)
                    .uppercase(false)
                    .render(),
            );
        }
        if let Some(bounds) = self.bounds {
            input = input
                .absolute()
                .left(px(bounds.x))
                .top(px(bounds.y))
                .w(px(bounds.w))
                .h(px(bounds.h));
        }
        input
    }
}

#[cfg(test)]
mod tests;
