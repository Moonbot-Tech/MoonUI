use gpui::*;

use super::{
    theme::MoonTheme,
    tokens::{MoonRect, MoonTone, rgba_from},
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MoonSpinnerSize {
    Compact,
    Normal,
    Custom(f32),
}

#[derive(IntoElement)]
pub struct MoonSpinner {
    bounds: Option<MoonRect>,
    size: MoonSpinnerSize,
    tone: MoonTone,
}

impl MoonSpinner {
    pub fn new() -> Self {
        Self {
            bounds: None,
            size: MoonSpinnerSize::Normal,
            tone: MoonTone::Info,
        }
    }

    pub fn bounds(mut self, bounds: MoonRect) -> Self {
        self.bounds = Some(bounds);
        self
    }

    pub fn size(mut self, size: MoonSpinnerSize) -> Self {
        self.size = size;
        self
    }

    pub fn tone(mut self, tone: MoonTone) -> Self {
        self.tone = tone;
        self
    }

    fn base_size(&self) -> f32 {
        match self.size {
            MoonSpinnerSize::Compact => 12.0,
            MoonSpinnerSize::Normal => 16.0,
            MoonSpinnerSize::Custom(size) => size,
        }
    }
}

impl Default for MoonSpinner {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderOnce for MoonSpinner {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = MoonTheme::active_tokens(cx);
        let p = tokens.palette;
        let size = tokens.ui(self.base_size());
        let accent = self.tone.color(p);
        let border = (size / 8.0).clamp(1.5, 3.0);
        let mut root = div()
            .relative()
            .w(px(size))
            .h(px(size))
            .rounded(px(size * 0.5))
            .border(px(border))
            .border_color(rgba_from(p.border, 0.76))
            .child(
                div()
                    .absolute()
                    .right(px(size * 0.08))
                    .top(px(size * 0.08))
                    .w(px(size * 0.36))
                    .h(px(border))
                    .rounded(px(border * 0.5))
                    .bg(rgba_from(accent, 1.0)),
            )
            .child(
                div()
                    .absolute()
                    .right(px(size * 0.08))
                    .top(px(size * 0.08))
                    .w(px(border))
                    .h(px(size * 0.36))
                    .rounded(px(border * 0.5))
                    .bg(rgba_from(accent, 0.58)),
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
