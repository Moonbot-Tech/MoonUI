use gpui::prelude::FluentBuilder;
use gpui::*;
use instant::Duration;

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
    secondary: bool,
    animated: bool,
}

/// Rendering decisions derived from the skeleton's builder controls.
#[derive(Clone, Copy, Debug, PartialEq)]
struct SkeletonRenderPlan {
    alpha: f32,
    animated: bool,
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
            secondary: false,
            animated: true,
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

    pub fn secondary(mut self) -> Self {
        self.secondary = true;
        self
    }

    pub fn animated(mut self, animated: bool) -> Self {
        self.animated = animated;
        self
    }

    /// Resolve the effective fill alpha and whether the pulse animation is enabled.
    ///
    /// Returns:
    ///     The rendering decisions consumed by [`RenderOnce::render`].
    fn render_plan(&self) -> SkeletonRenderPlan {
        SkeletonRenderPlan {
            alpha: if self.secondary {
                self.alpha * 0.5
            } else {
                self.alpha
            },
            animated: self.animated,
        }
    }
}

impl RenderOnce for MoonSkeleton {
    /// Render the configured placeholder with its resolved emphasis and animation state.
    ///
    /// Args:
    ///     _window: Window that owns the rendered skeleton.
    ///     cx: Application context used to resolve active theme tokens.
    ///
    /// Returns:
    ///     The styled skeleton element, optionally wrapped in its pulse animation.
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = MoonTheme::active_tokens(cx);
        let p = tokens.palette;
        let plan = self.render_plan();
        let mut root = div()
            .id(ElementId::from(self.id))
            .relative()
            .h(px(tokens.ui(self.height)))
            .rounded(px(tokens.ui(self.radius)))
            .overflow_hidden()
            .bg(rgba_from(p.panel_high, plan.alpha))
            .when_some(self.width, |this, width| this.w(px(tokens.ui(width))));

        if let Some(bounds) = self.bounds {
            root = root
                .absolute()
                .left(px(bounds.x))
                .top(px(bounds.y))
                .w(px(bounds.w))
                .h(px(bounds.h));
        }

        root.map(|this| {
            if plan.animated {
                this.with_animation(
                    "moon-skeleton",
                    Animation::new(Duration::from_secs(2))
                        .repeat()
                        .with_easing(bounce(ease_in_out)),
                    |this, delta| this.opacity(1.0 - delta * 0.32),
                )
                .into_any_element()
            } else {
                this.into_any_element()
            }
        })
    }
}

#[cfg(test)]
mod tests;
