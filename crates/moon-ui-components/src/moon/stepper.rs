//! Density-tier numeric stepper with bounded, controlled or local value changes.

use crate::{
    Disableable,
    button::{Button, ButtonRounded, ButtonVariant, ButtonVariants},
};
use gpui::*;

use super::{
    foundation::{MoonF32ChangeHandler, MoonSize},
    theme::{MoonTheme, MoonThemeTokens},
    tokens::{MoonRect, MoonTone, rgba_from},
};

#[derive(Default)]
struct MoonStepperRuntimeState {
    value: f32,
}

/// Shared density tier or explicit legacy-scaled stepper dimensions.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MoonStepperSize {
    /// Xs, Sm and Md are supported; larger tiers snap to Md.
    Tier(MoonSize),
    Custom {
        height: f32,
        button_width: f32,
        value_width: f32,
        font_size: f32,
        line_height: f32,
    },
}

#[allow(non_upper_case_globals)]
impl MoonStepperSize {
    /// Compatibility name for the small tier.
    #[deprecated(note = "use MoonSize::Sm.into()")]
    pub const Compact: Self = Self::Tier(MoonSize::Sm);
    /// Compatibility name for the medium tier.
    #[deprecated(note = "use MoonSize::Md.into()")]
    pub const Normal: Self = Self::Tier(MoonSize::Md);
}

impl From<MoonSize> for MoonStepperSize {
    /// Convert a shared tier; unsupported tiers snap during rendering.
    fn from(size: MoonSize) -> Self {
        Self::Tier(size)
    }
}

/// Unscaled geometry and typography resolved from the requested size.
#[derive(Clone, Copy, Debug)]
struct StepperMetrics {
    radius: f32,
    height: f32,
    button_width: f32,
    value_width: f32,
    font_size: f32,
    line_height: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MoonStepperDirection {
    Decrement,
    Increment,
}

fn moon_stepper_next_value(
    value: f32,
    min: f32,
    max: f32,
    step: f32,
    direction: MoonStepperDirection,
) -> f32 {
    let step = step.max(f32::EPSILON);
    match direction {
        MoonStepperDirection::Decrement => value - step,
        MoonStepperDirection::Increment => value + step,
    }
    .clamp(min, max)
}

#[derive(IntoElement)]
pub struct MoonStepper {
    id: SharedString,
    bounds: Option<MoonRect>,
    value: Option<f32>,
    default_value: f32,
    min: f32,
    max: f32,
    step: f32,
    precision: usize,
    disabled: bool,
    size: Option<MoonStepperSize>,
    tone: MoonTone,
    on_change: Option<MoonF32ChangeHandler>,
}

impl MoonStepper {
    /// Create a stepper that follows the active density unless a size is set.
    pub fn new(id: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            bounds: None,
            value: None,
            default_value: 0.0,
            min: f32::NEG_INFINITY,
            max: f32::INFINITY,
            step: 1.0,
            precision: 0,
            disabled: false,
            size: None,
            tone: MoonTone::Info,
            on_change: None,
        }
    }

    pub fn bounds(mut self, bounds: MoonRect) -> Self {
        self.bounds = Some(bounds);
        self
    }

    pub fn value(mut self, value: f32) -> Self {
        self.value = Some(value);
        self
    }

    pub fn default_value(mut self, value: f32) -> Self {
        self.default_value = value;
        self
    }

    pub fn min(mut self, min: f32) -> Self {
        self.min = min;
        self
    }

    pub fn max(mut self, max: f32) -> Self {
        self.max = max;
        self
    }

    pub fn range(mut self, min: f32, max: f32) -> Self {
        self.min = min;
        self.max = max;
        self
    }

    pub fn step(mut self, step: f32) -> Self {
        self.step = step.max(f32::EPSILON);
        self
    }

    pub fn precision(mut self, precision: usize) -> Self {
        self.precision = precision;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Override density with explicit tier or custom design-reference dimensions.
    pub fn size(mut self, size: MoonStepperSize) -> Self {
        self.size = Some(size);
        self
    }

    pub fn tone(mut self, tone: MoonTone) -> Self {
        self.tone = tone;
        self
    }

    pub fn on_change(mut self, handler: impl Fn(f32, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(std::rc::Rc::new(handler));
        self
    }

    pub fn render(self) -> impl IntoElement {
        self
    }

    /// Return rendered text metrics, applying exactly one scale for the selected size mode.
    fn text_metrics(&self, tokens: &MoonThemeTokens) -> (f32, f32) {
        let m = self.metrics(tokens);
        if matches!(self.size, Some(MoonStepperSize::Custom { .. })) {
            (tokens.font(m.font_size), tokens.line_height(m.line_height))
        } else {
            (tokens.ui(m.font_size), tokens.ui(m.line_height))
        }
    }

    /// Resolve design-reference metrics using density when no override was supplied.
    fn metrics(&self, tokens: &MoonThemeTokens) -> StepperMetrics {
        match self.size.unwrap_or(MoonStepperSize::Tier(tokens.tier())) {
            MoonStepperSize::Tier(size) => {
                let tier = size.nearest(&[MoonSize::Xs, MoonSize::Sm, MoonSize::Md]);
                let m = tier.control_metrics();
                // Preserve the compact ratio below Md, and the normal ratio at Md.
                let ratio = if tier == MoonSize::Md {
                    64.0 / 26.0
                } else {
                    52.0 / 22.0
                };
                StepperMetrics {
                    height: m.height,
                    radius: m.radius,
                    button_width: m.height,
                    value_width: (m.height * ratio).round(),
                    font_size: m.font_size,
                    line_height: m.line_height,
                }
            }
            MoonStepperSize::Custom {
                height,
                button_width,
                value_width,
                font_size,
                line_height,
            } => StepperMetrics {
                radius: 4.0,
                height,
                button_width,
                value_width,
                font_size,
                line_height,
            },
        }
    }
}

impl RenderOnce for MoonStepper {
    /// Render tier geometry and text with UI zoom, retaining legacy text scaling for Custom.
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = MoonTheme::active_tokens(cx);
        let metrics = self.metrics(&tokens);
        let (font_size, line_height) = self.text_metrics(&tokens);
        let p = tokens.palette;
        let state = window.use_keyed_state(
            ElementId::from(SharedString::from(format!("{}:state", self.id))),
            cx,
            |_, _| MoonStepperRuntimeState {
                value: self.default_value.clamp(self.min, self.max),
            },
        );
        let value = self
            .value
            .unwrap_or_else(|| state.read(cx).value)
            .clamp(self.min, self.max);
        let controlled = self.value.is_some();
        let disabled = self.disabled;
        let min = self.min;
        let max = self.max;
        let step = self.step;
        let parent_view = window.current_view();
        let on_change_dec = self.on_change.clone();
        let on_change_inc = self.on_change.clone();
        let state_dec = state.clone();
        let state_inc = state.clone();
        let id = self.id.clone();
        let minus_id = SharedString::from(format!("{}:minus", self.id));
        let plus_id = SharedString::from(format!("{}:plus", self.id));
        let tone_color = self.tone.color(p);
        let value_text = format!("{:.*}", self.precision, value);
        let mut root = div()
            .id(ElementId::from(id))
            .relative()
            .h(px(tokens.ui(metrics.height)))
            .flex()
            .items_center()
            .overflow_hidden()
            .rounded(px(tokens.ui(metrics.radius)))
            .border(px(tokens.ui(1.0)))
            .border_color(rgba_from(p.border, if disabled { 0.45 } else { 1.0 }))
            .bg(rgba_from(p.shell_high, if disabled { 0.38 } else { 0.96 }))
            .child(
                Button::new(minus_id)
                    .with_variant(ButtonVariant::Ghost)
                    .rounded(ButtonRounded::None)
                    .h(px(tokens.ui(metrics.height)))
                    .disabled(disabled || value <= min)
                    .w(px(tokens.ui(metrics.button_width)))
                    .child(
                        div()
                            .text_size(px(font_size))
                            .line_height(px(line_height))
                            .font_weight(FontWeight(600.0))
                            .text_color(rgba_from(p.text_soft, 1.0))
                            .child("-"),
                    )
                    .on_click(move |_, window, cx| {
                        let next = moon_stepper_next_value(
                            value,
                            min,
                            max,
                            step,
                            MoonStepperDirection::Decrement,
                        );
                        if !controlled {
                            state_dec.update(cx, |state, _| state.value = next);
                        }
                        if let Some(on_change) = on_change_dec.as_ref() {
                            on_change(next, window, cx);
                        }
                        cx.notify(parent_view);
                    }),
            )
            .child(
                div()
                    .w(px(tokens.ui(metrics.value_width)))
                    .h_full()
                    .border_l(px(tokens.ui(1.0)))
                    .border_r(px(tokens.ui(1.0)))
                    .border_color(rgba_from(p.border, 0.72))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .text_color(rgba_from(
                                if disabled { p.text_muted } else { tone_color },
                                if disabled { 0.50 } else { 1.0 },
                            ))
                            .text_size(px(font_size))
                            .line_height(px(line_height))
                            .font_weight(FontWeight(600.0))
                            .font_family(tokens.font_family(true))
                            .child(value_text),
                    ),
            )
            .child(
                Button::new(plus_id)
                    .with_variant(ButtonVariant::Ghost)
                    .rounded(ButtonRounded::None)
                    .h(px(tokens.ui(metrics.height)))
                    .disabled(disabled || value >= max)
                    .w(px(tokens.ui(metrics.button_width)))
                    .child(
                        div()
                            .text_size(px(font_size))
                            .line_height(px(line_height))
                            .font_weight(FontWeight(600.0))
                            .text_color(rgba_from(p.text_soft, 1.0))
                            .child("+"),
                    )
                    .on_click(move |_, window, cx| {
                        let next = moon_stepper_next_value(
                            value,
                            min,
                            max,
                            step,
                            MoonStepperDirection::Increment,
                        );
                        if !controlled {
                            state_inc.update(cx, |state, _| state.value = next);
                        }
                        if let Some(on_change) = on_change_inc.as_ref() {
                            on_change(next, window, cx);
                        }
                        cx.notify(parent_view);
                    }),
            );

        if let Some(bounds) = self.bounds {
            root = root
                .absolute()
                .left(px(bounds.x))
                .top(px(bounds.y))
                .w(px(bounds.w))
                .h(px(bounds.h));
        }

        root
    }
}

#[cfg(test)]
mod tests;
