use gpui::prelude::FluentBuilder;
use gpui::*;

use super::{
    text::MoonText,
    theme::MoonTheme,
    tokens::{MoonRect, MoonTone, rgba_from},
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MoonRadioSize {
    Compact,
    Normal,
    Custom {
        dot_size: f32,
        font_size: f32,
        line_height: f32,
        gap: f32,
    },
}

#[derive(Clone, Copy, Debug)]
struct RadioMetrics {
    outer_size: f32,
    inner_size: f32,
    font_size: f32,
    line_height: f32,
    gap: f32,
}

fn moon_radio_click_value(disabled: bool) -> Option<bool> {
    if disabled { None } else { Some(true) }
}

#[derive(IntoElement)]
pub struct MoonRadio {
    id: SharedString,
    bounds: Option<MoonRect>,
    label: Option<SharedString>,
    checked: bool,
    disabled: bool,
    size: MoonRadioSize,
    tone: MoonTone,
    mono: bool,
    on_change: Option<std::rc::Rc<dyn Fn(&bool, &mut Window, &mut App)>>,
}

impl MoonRadio {
    pub fn new(id: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            bounds: None,
            label: None,
            checked: false,
            disabled: false,
            size: MoonRadioSize::Normal,
            tone: MoonTone::Info,
            mono: true,
            on_change: None,
        }
    }

    pub fn bounds(mut self, bounds: MoonRect) -> Self {
        self.bounds = Some(bounds);
        self
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn size(mut self, size: MoonRadioSize) -> Self {
        self.size = size;
        self
    }

    pub fn tone(mut self, tone: MoonTone) -> Self {
        self.tone = tone;
        self
    }

    pub fn mono(mut self, mono: bool) -> Self {
        self.mono = mono;
        self
    }

    pub fn on_change(mut self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(std::rc::Rc::new(handler));
        self
    }

    fn metrics(&self) -> RadioMetrics {
        match self.size {
            MoonRadioSize::Compact => RadioMetrics {
                outer_size: 12.0,
                inner_size: 5.0,
                font_size: 9.5,
                line_height: 12.0,
                gap: 6.0,
            },
            MoonRadioSize::Normal => RadioMetrics {
                outer_size: 14.0,
                inner_size: 6.0,
                font_size: 10.5,
                line_height: 13.0,
                gap: 7.0,
            },
            MoonRadioSize::Custom {
                dot_size,
                font_size,
                line_height,
                gap,
            } => RadioMetrics {
                outer_size: dot_size,
                inner_size: (dot_size * 0.44).round(),
                font_size,
                line_height,
                gap,
            },
        }
    }
}

impl RenderOnce for MoonRadio {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = MoonTheme::active_tokens(cx);
        let metrics = self.metrics();
        let p = tokens.palette;
        let accent = self.tone.color(p);
        let alpha = if self.disabled { 0.45 } else { 1.0 };
        let disabled = self.disabled;
        let checked = self.checked;
        let outer_size = tokens.ui(metrics.outer_size);
        let inner_size = tokens.ui(metrics.inner_size);
        let mut mark = div()
            .relative()
            .w(px(outer_size))
            .h(px(outer_size))
            .rounded(px(outer_size * 0.5))
            .border(px(tokens.ui(1.0)))
            .border_color(rgba_from(
                if checked { accent } else { p.border },
                0.82 * alpha,
            ))
            .bg(rgba_from(
                if checked { accent } else { p.shell_high },
                if checked { 0.14 } else { 0.95 } * alpha,
            ));

        if checked {
            mark = mark.child(
                div()
                    .absolute()
                    .left(px((outer_size - inner_size) * 0.5))
                    .top(px((outer_size - inner_size) * 0.5))
                    .w(px(inner_size))
                    .h(px(inner_size))
                    .rounded(px(inner_size * 0.5))
                    .bg(rgba_from(accent, alpha)),
            );
        }

        let mut root = div()
            .id(ElementId::from(SharedString::from(format!(
                "{}:root",
                self.id
            ))))
            .relative()
            .flex()
            .items_center()
            .gap(px(tokens.ui(metrics.gap)))
            .rounded(px(tokens.ui(4.0)))
            .cursor_default()
            .when(!disabled, |this| {
                this.hover(|this| this.bg(rgba_from(p.overlay, 0.025)))
                    .active(|this| this.bg(rgba_from(p.overlay, 0.015)))
            })
            .child(mark);

        if let Some(label) = self.label {
            let text = tokens.text(metrics.font_size, metrics.line_height);
            root = root.child(
                MoonText::new(label)
                    .color(if disabled { p.text_muted } else { p.text_soft })
                    .alpha(alpha)
                    .font_size(text.font_size)
                    .line_height(text.line_height)
                    .weight(400.0)
                    .mono(self.mono)
                    .uppercase(false)
                    .render(),
            );
        }

        if let Some(bounds) = self.bounds {
            root = root
                .absolute()
                .left(px(bounds.x))
                .top(px(bounds.y))
                .w(px(bounds.w))
                .h(px(bounds.h));
        }

        root = root.on_mouse_down(MouseButton::Left, move |_, window, cx| {
            cx.stop_propagation();
            if disabled {
                return;
            }
            window.prevent_default();
        });

        if let Some(on_change) = self.on_change {
            root = root.on_click(move |_, window, cx| {
                let Some(value) = moon_radio_click_value(disabled) else {
                    cx.stop_propagation();
                    return;
                };
                on_change(&value, window, cx);
            });
        }

        root
    }
}

#[cfg(test)]
mod tests {
    use super::{MoonRadio, MoonRadioSize, moon_radio_click_value};

    #[test]
    fn radio_metrics_match_designer_reference() {
        let compact = MoonRadio::new("compact").size(MoonRadioSize::Compact);
        assert_eq!(compact.metrics().outer_size, 12.0);
        assert_eq!(compact.metrics().inner_size, 5.0);
        let normal = MoonRadio::new("normal");
        assert_eq!(normal.metrics().outer_size, 14.0);
        assert_eq!(normal.metrics().inner_size, 6.0);
    }

    #[test]
    fn radio_click_value_respects_disabled_state() {
        assert_eq!(moon_radio_click_value(false), Some(true));
        assert_eq!(moon_radio_click_value(true), None);
    }
}
