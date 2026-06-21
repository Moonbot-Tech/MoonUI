use gpui::prelude::FluentBuilder;
use gpui::*;

use super::{
    theme::MoonTheme,
    tokens::{MoonRect, rgba_from},
};

#[derive(IntoElement)]
pub struct MoonSkeleton {
    id: SharedString,
    bounds: Option<MoonRect>,
    width: Option<f32>,
    height: f32,
    radius: f32,
    alpha: f32,
}

impl MoonSkeleton {
    pub fn new(id: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            bounds: None,
            width: None,
            height: 14.0,
            radius: 4.0,
            alpha: 0.52,
        }
    }

    pub fn bounds(mut self, bounds: MoonRect) -> Self {
        self.bounds = Some(bounds);
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    pub fn alpha(mut self, alpha: f32) -> Self {
        self.alpha = alpha;
        self
    }
}

impl RenderOnce for MoonSkeleton {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = MoonTheme::active_tokens(cx);
        let p = tokens.palette;
        let mut root = div()
            .id(ElementId::from(self.id))
            .relative()
            .h(px(tokens.ui(self.height)))
            .rounded(px(tokens.ui(self.radius)))
            .overflow_hidden()
            .bg(rgba_from(p.panel_high, self.alpha))
            .when_some(self.width, |this, width| this.w(px(tokens.ui(width))));

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
