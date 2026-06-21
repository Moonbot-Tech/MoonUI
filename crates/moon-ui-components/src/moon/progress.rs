use gpui::*;

use crate::progress::Progress;

use super::{
    theme::MoonTheme,
    tokens::{MoonTone, rgba_from},
};

#[derive(IntoElement)]
pub struct MoonProgress {
    id: ElementId,
    value: f32,
    loading: bool,
    tone: MoonTone,
    color: Option<u32>,
    height: f32,
    radius: Option<f32>,
}

impl MoonProgress {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            value: 0.0,
            loading: false,
            tone: MoonTone::Positive,
            color: None,
            height: 4.0,
            radius: None,
        }
    }

    pub fn value(mut self, value: f32) -> Self {
        self.value = value.clamp(0.0, 100.0);
        self
    }

    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    pub fn tone(mut self, tone: MoonTone) -> Self {
        self.tone = tone;
        self
    }

    pub fn color(mut self, color: u32) -> Self {
        self.color = Some(color);
        self
    }

    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = Some(radius);
        self
    }

    pub fn render(self) -> impl IntoElement {
        self
    }
}

impl RenderOnce for MoonProgress {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = MoonTheme::active_tokens(cx);
        let p = tokens.palette;
        let height = tokens.ui(self.height);
        let radius = self
            .radius
            .map(|radius| tokens.ui(radius))
            .unwrap_or(height * 0.5);
        let color = self.color.unwrap_or_else(|| self.tone.color(p));

        Progress::new(self.id)
            .value(self.value)
            .loading(self.loading)
            .color(rgba_from(color, 1.0))
            .h(px(height))
            .rounded(px(radius))
    }
}
