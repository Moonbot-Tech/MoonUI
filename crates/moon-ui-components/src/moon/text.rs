use gpui::prelude::FluentBuilder;
use gpui::*;

use super::{
    theme::MoonTheme,
    tokens::{MoonPalette, MoonRect, rgba_from},
};

#[derive(Clone, Copy, Debug)]
pub struct MoonTextStyle {
    pub color: u32,
    pub alpha: f32,
    pub font_size: f32,
    pub line_height: f32,
    pub weight: f32,
    pub tracking: f32,
    pub uppercase: bool,
    pub mono: bool,
}

impl Default for MoonTextStyle {
    fn default() -> Self {
        Self {
            color: MoonPalette::TERMINAL.text_muted,
            alpha: 1.0,
            font_size: 9.0,
            line_height: 11.0,
            weight: 400.0,
            tracking: 0.0,
            uppercase: true,
            mono: false,
        }
    }
}

#[derive(IntoElement)]
pub struct MoonText {
    bounds: Option<MoonRect>,
    text: SharedString,
    style: MoonTextStyle,
    wrap: bool,
}

impl MoonText {
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self {
            bounds: None,
            text: text.into(),
            style: MoonTextStyle::default(),
            wrap: false,
        }
    }

    pub fn bounds(mut self, bounds: MoonRect) -> Self {
        self.bounds = Some(bounds);
        self
    }

    pub fn style(mut self, style: MoonTextStyle) -> Self {
        self.style = style;
        self
    }

    pub fn color(mut self, color: u32) -> Self {
        self.style.color = color;
        self
    }

    pub fn alpha(mut self, alpha: f32) -> Self {
        self.style.alpha = alpha;
        self
    }

    /// Base (unscaled) font size. `MoonText` scales it with `tokens.font()` at
    /// render time — pass design-reference values (e.g. `9.0`, `11.0`), never
    /// values that were already scaled (e.g. via a theme-scale helper), or the UI
    /// font scale gets applied twice.
    pub fn font_size(mut self, font_size: f32) -> Self {
        self.style.font_size = font_size;
        self
    }

    /// Base (unscaled) line height — scaled at render like [`Self::font_size`].
    pub fn line_height(mut self, line_height: f32) -> Self {
        self.style.line_height = line_height;
        self
    }

    pub fn weight(mut self, weight: f32) -> Self {
        self.style.weight = weight;
        self
    }

    pub fn tracking(mut self, tracking: f32) -> Self {
        self.style.tracking = tracking;
        self
    }

    pub fn uppercase(mut self, uppercase: bool) -> Self {
        self.style.uppercase = uppercase;
        self
    }

    pub fn mono(mut self, mono: bool) -> Self {
        self.style.mono = mono;
        self
    }

    /// Allow the text to wrap and grow vertically instead of forcing one line.
    pub fn wrap(mut self) -> Self {
        self.wrap = true;
        self
    }

    pub fn render(self) -> Self {
        self
    }
}

impl RenderOnce for MoonText {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let style = self.style;
        let tokens = MoonTheme::active_tokens(cx);
        let text_metrics = tokens.text(style.font_size, style.line_height);
        let text = if style.uppercase {
            self.text.as_ref().to_uppercase()
        } else {
            self.text.as_ref().to_string()
        };

        let wrap = self.wrap;
        let mut element = div()
            .relative()
            .font_family(tokens.font_family(style.mono))
            .text_size(px(text_metrics.font_size))
            .line_height(px(text_metrics.line_height))
            .font_weight(FontWeight(style.weight))
            .text_color(rgba_from(style.color, style.alpha))
            .when(!wrap, |this| {
                this.flex()
                    .items_center()
                    .h(px(text_metrics.line_height))
                    .whitespace_nowrap()
            })
            .when(wrap, |this| this.block().whitespace_normal())
            .when(!wrap && style.tracking.abs() > f32::EPSILON, |this| {
                this.gap(px(tokens.ui(style.tracking)))
            });

        if let Some(bounds) = self.bounds {
            element = element
                .absolute()
                .left(px(bounds.x))
                .top(px(bounds.y))
                .w(px(bounds.w))
                .h(px(bounds.h));
        }

        if !wrap && style.tracking.abs() > f32::EPSILON {
            for ch in text.chars() {
                element = element.child(ch.to_string());
            }
        } else {
            element = element.child(text);
        }

        element
    }
}
