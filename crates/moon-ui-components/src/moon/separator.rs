use gpui::*;

use super::{
    theme::MoonTheme,
    tokens::{MoonRect, rgba_from},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoonSeparatorAxis {
    Horizontal,
    Vertical,
}

#[derive(IntoElement)]
pub struct MoonSeparator {
    axis: MoonSeparatorAxis,
    bounds: Option<MoonRect>,
    color: Option<u32>,
    alpha: f32,
    thickness: f32,
}

impl MoonSeparator {
    pub fn horizontal() -> Self {
        Self::new(MoonSeparatorAxis::Horizontal)
    }

    pub fn vertical() -> Self {
        Self::new(MoonSeparatorAxis::Vertical)
    }

    pub fn new(axis: MoonSeparatorAxis) -> Self {
        Self {
            axis,
            bounds: None,
            color: None,
            alpha: 1.0,
            thickness: 1.0,
        }
    }

    pub fn bounds(mut self, bounds: MoonRect) -> Self {
        self.bounds = Some(bounds);
        self
    }

    pub fn color(mut self, color: u32) -> Self {
        self.color = Some(color);
        self
    }

    pub fn alpha(mut self, alpha: f32) -> Self {
        self.alpha = alpha;
        self
    }

    pub fn thickness(mut self, thickness: f32) -> Self {
        self.thickness = thickness;
        self
    }
}

impl RenderOnce for MoonSeparator {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = MoonTheme::active_tokens(cx);
        let p = tokens.palette;
        let mut line = div()
            .relative()
            .flex_shrink_0()
            .bg(rgba_from(self.color.unwrap_or(p.border), self.alpha));

        line = match self.axis {
            MoonSeparatorAxis::Horizontal => line.w_full().h(px(tokens.ui(self.thickness))),
            MoonSeparatorAxis::Vertical => line.h_full().w(px(tokens.ui(self.thickness))),
        };

        if let Some(bounds) = self.bounds {
            line = line
                .absolute()
                .left(px(bounds.x))
                .top(px(bounds.y))
                .w(px(bounds.w))
                .h(px(bounds.h));
        }

        line
    }
}
