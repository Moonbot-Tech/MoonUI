use gpui::*;

use super::{
    button::{MoonButton, MoonButtonSegment, MoonButtonSize, MoonButtonVariant},
    foundation::MoonF32ChangeHandler,
    text::MoonText,
    theme::MoonTheme,
    tokens::{MoonRect, MoonTone, rgba_from},
};

#[derive(Default)]
struct MoonStepperRuntimeState {
    value: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MoonStepperSize {
    Compact,
    Normal,
    Custom {
        height: f32,
        button_width: f32,
        value_width: f32,
        font_size: f32,
        line_height: f32,
    },
}

#[derive(Clone, Copy, Debug)]
struct StepperMetrics {
    height: f32,
    button_width: f32,
    value_width: f32,
    font_size: f32,
    line_height: f32,
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
    size: MoonStepperSize,
    tone: MoonTone,
    on_change: Option<MoonF32ChangeHandler>,
}

impl MoonStepper {
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
            size: MoonStepperSize::Normal,
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

    pub fn size(mut self, size: MoonStepperSize) -> Self {
        self.size = size;
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

    fn metrics(&self) -> StepperMetrics {
        match self.size {
            MoonStepperSize::Compact => StepperMetrics {
                height: 22.0,
                button_width: 24.0,
                value_width: 52.0,
                font_size: 10.0,
                line_height: 13.0,
            },
            MoonStepperSize::Normal => StepperMetrics {
                height: 26.0,
                button_width: 28.0,
                value_width: 64.0,
                font_size: 10.5,
                line_height: 14.0,
            },
            MoonStepperSize::Custom {
                height,
                button_width,
                value_width,
                font_size,
                line_height,
            } => StepperMetrics {
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
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let metrics = self.metrics();
        let tokens = MoonTheme::active_tokens(cx);
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
            .rounded(px(tokens.ui(4.0)))
            .border(px(tokens.ui(1.0)))
            .border_color(rgba_from(p.border, if disabled { 0.45 } else { 1.0 }))
            .bg(rgba_from(p.shell_high, if disabled { 0.38 } else { 0.96 }))
            .child(
                MoonButton::new(minus_id)
                    .variant(MoonButtonVariant::Ghost)
                    .size(MoonButtonSize::Custom {
                        height: metrics.height,
                        radius: 0.0,
                        font_size: metrics.font_size,
                        line_height: metrics.line_height,
                        gap: 0.0,
                    })
                    .disabled(disabled || value <= min)
                    .width(metrics.button_width)
                    .segment(MoonButtonSegment::new("-").color(p.text_soft).weight(600.0))
                    .on_click(move |_, window, cx| {
                        let next = (value - step).clamp(min, max);
                        if !controlled {
                            state_dec.update(cx, |state, _| state.value = next);
                        }
                        if let Some(on_change) = on_change_dec.as_ref() {
                            on_change(next, window, cx);
                        }
                        cx.notify(parent_view);
                    })
                    .render(),
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
                        MoonText::new(value_text)
                            .color(if disabled { p.text_muted } else { tone_color })
                            .alpha(if disabled { 0.50 } else { 1.0 })
                            .font_size(metrics.font_size)
                            .line_height(metrics.line_height)
                            .weight(600.0)
                            .mono(true)
                            .uppercase(false)
                            .render(),
                    ),
            )
            .child(
                MoonButton::new(plus_id)
                    .variant(MoonButtonVariant::Ghost)
                    .size(MoonButtonSize::Custom {
                        height: metrics.height,
                        radius: 0.0,
                        font_size: metrics.font_size,
                        line_height: metrics.line_height,
                        gap: 0.0,
                    })
                    .disabled(disabled || value >= max)
                    .width(metrics.button_width)
                    .segment(MoonButtonSegment::new("+").color(p.text_soft).weight(600.0))
                    .on_click(move |_, window, cx| {
                        let next = (value + step).clamp(min, max);
                        if !controlled {
                            state_inc.update(cx, |state, _| state.value = next);
                        }
                        if let Some(on_change) = on_change_inc.as_ref() {
                            on_change(next, window, cx);
                        }
                        cx.notify(parent_view);
                    })
                    .render(),
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
mod tests {
    use super::{MoonStepper, MoonStepperSize};

    #[test]
    fn stepper_metrics_match_designer_reference() {
        let compact = MoonStepper::new("compact").size(MoonStepperSize::Compact);
        assert_eq!(compact.metrics().height, 22.0);
        assert_eq!(compact.metrics().button_width, 24.0);
        let normal = MoonStepper::new("normal");
        assert_eq!(normal.metrics().height, 26.0);
        assert_eq!(normal.metrics().value_width, 64.0);
    }
}
