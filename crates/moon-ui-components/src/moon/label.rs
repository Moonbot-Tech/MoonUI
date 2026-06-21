use gpui::*;

use super::{
    text::{MoonText, MoonTextStyle},
    tokens::MoonRect,
};

pub struct MoonLabel {
    text: MoonText,
}

impl MoonLabel {
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self {
            text: MoonText::new(text),
        }
    }

    pub fn bounds(mut self, bounds: MoonRect) -> Self {
        self.text = self.text.bounds(bounds);
        self
    }

    pub fn style(mut self, style: MoonTextStyle) -> Self {
        self.text = self.text.style(style);
        self
    }

    pub fn color(mut self, color: u32) -> Self {
        self.text = self.text.color(color);
        self
    }

    pub fn alpha(mut self, alpha: f32) -> Self {
        self.text = self.text.alpha(alpha);
        self
    }

    pub fn font_size(mut self, font_size: f32) -> Self {
        self.text = self.text.font_size(font_size);
        self
    }

    pub fn line_height(mut self, line_height: f32) -> Self {
        self.text = self.text.line_height(line_height);
        self
    }

    pub fn weight(mut self, weight: f32) -> Self {
        self.text = self.text.weight(weight);
        self
    }

    pub fn tracking(mut self, tracking: f32) -> Self {
        self.text = self.text.tracking(tracking);
        self
    }

    pub fn uppercase(mut self, uppercase: bool) -> Self {
        self.text = self.text.uppercase(uppercase);
        self
    }

    pub fn mono(mut self, mono: bool) -> Self {
        self.text = self.text.mono(mono);
        self
    }

    pub fn render(self) -> impl IntoElement {
        self.text.render()
    }
}
