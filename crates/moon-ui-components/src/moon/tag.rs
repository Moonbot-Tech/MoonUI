use gpui::prelude::FluentBuilder;
use gpui::*;

use crate::h_flex;

use super::{
    theme::MoonTheme,
    tokens::{MoonTone, rgba_from},
};

#[derive(IntoElement)]
pub struct MoonTag {
    tone: MoonTone,
    outline: bool,
    rounded_full: bool,
    mono: bool,
    children: Vec<AnyElement>,
}

impl MoonTag {
    pub fn new() -> Self {
        Self {
            tone: MoonTone::Default,
            outline: false,
            rounded_full: false,
            mono: true,
            children: Vec::new(),
        }
    }

    pub fn positive() -> Self {
        Self::new().tone(MoonTone::Positive)
    }

    pub fn warning() -> Self {
        Self::new().tone(MoonTone::Warning)
    }

    pub fn danger() -> Self {
        Self::new().tone(MoonTone::Danger)
    }

    pub fn info() -> Self {
        Self::new().tone(MoonTone::Info)
    }

    pub fn tone(mut self, tone: MoonTone) -> Self {
        self.tone = tone;
        self
    }

    pub fn outline(mut self) -> Self {
        self.outline = true;
        self
    }

    pub fn rounded_full(mut self) -> Self {
        self.rounded_full = true;
        self
    }

    pub fn mono(mut self, mono: bool) -> Self {
        self.mono = mono;
        self
    }

    pub fn render(self) -> impl IntoElement {
        self
    }
}

impl ParentElement for MoonTag {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for MoonTag {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = MoonTheme::active_tokens(cx);
        let p = tokens.palette;
        let color = self.tone.color(p);
        let text = tokens.text(11.0, 14.0);
        let radius = if self.rounded_full { 999.0 } else { 4.0 };

        h_flex()
            .flex_none()
            .items_center()
            .gap(px(tokens.ui(6.0)))
            .min_h(px(tokens.fit_height(22.0, 14.0, 4.0)))
            .px(px(tokens.ui(10.0)))
            .rounded(px(tokens.ui(radius)))
            .border_1()
            .border_color(rgba_from(color, if self.outline { 0.48 } else { 0.34 }))
            .bg(rgba_from(color, if self.outline { 0.0 } else { 0.14 }))
            .text_color(rgba_from(color, 1.0))
            .font_family(tokens.font_family(self.mono))
            .text_size(px(text.font_size))
            .line_height(px(text.line_height))
            .when(!self.outline, |this| this.shadow_sm())
            .children(self.children)
    }
}
