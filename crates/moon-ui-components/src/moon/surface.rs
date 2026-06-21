use gpui::*;

use super::{
    background::MoonBackgroundPolicy,
    theme::MoonTheme,
    tokens::{MoonRect, rgba_from},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoonSurfaceVariant {
    Card,
    Sidebar,
}

#[derive(Clone, Copy, Debug)]
struct MoonSurfaceStyle {
    bg: u32,
    bg_alpha: f32,
    border: u32,
    border_alpha: f32,
    radius: f32,
}

#[derive(IntoElement)]
pub struct MoonSurface {
    id: Option<ElementId>,
    bounds: Option<MoonRect>,
    variant: MoonSurfaceVariant,
    bg: Option<u32>,
    bg_alpha: Option<f32>,
    background_policy: MoonBackgroundPolicy,
    border: Option<u32>,
    border_alpha: Option<f32>,
    radius: Option<f32>,
    children: Vec<AnyElement>,
}

impl MoonSurface {
    pub fn new() -> Self {
        Self {
            id: None,
            bounds: None,
            variant: MoonSurfaceVariant::Card,
            bg: None,
            bg_alpha: None,
            background_policy: MoonBackgroundPolicy::Opaque,
            border: None,
            border_alpha: None,
            radius: None,
            children: Vec::new(),
        }
    }

    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn bounds(mut self, bounds: MoonRect) -> Self {
        self.bounds = Some(bounds);
        self
    }

    pub fn variant(mut self, variant: MoonSurfaceVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn bg_color(mut self, bg: u32) -> Self {
        self.bg = Some(bg);
        self
    }

    pub fn bg_alpha(mut self, alpha: f32) -> Self {
        self.bg_alpha = Some(alpha);
        self
    }

    pub fn background_policy(mut self, policy: MoonBackgroundPolicy) -> Self {
        self.background_policy = policy;
        self
    }

    pub fn border_color(mut self, border: u32) -> Self {
        self.border = Some(border);
        self
    }

    pub fn border_alpha(mut self, alpha: f32) -> Self {
        self.border_alpha = Some(alpha);
        self
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = Some(radius);
        self
    }

    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.children.push(child.into_any_element());
        self
    }

    fn style(&self, cx: &App) -> MoonSurfaceStyle {
        let p = MoonTheme::active_tokens(cx).palette;
        match self.variant {
            MoonSurfaceVariant::Card => MoonSurfaceStyle {
                bg: p.overlay,
                bg_alpha: 0.02,
                border: p.overlay,
                border_alpha: 0.05,
                radius: 3.0,
            },
            MoonSurfaceVariant::Sidebar => MoonSurfaceStyle {
                bg: p.shell_high,
                bg_alpha: 1.0,
                border: p.overlay,
                border_alpha: 0.05,
                radius: 0.0,
            },
        }
    }

    fn build(self, cx: &App) -> AnyElement {
        let tokens = MoonTheme::active_tokens(cx);
        let style = self.style(cx);
        let mut surface = div()
            .relative()
            .rounded(px(tokens.ui(self.radius.unwrap_or(style.radius))))
            .border(px(tokens.ui(1.0)))
            .border_color(rgba_from(
                self.border.unwrap_or(style.border),
                self.border_alpha.unwrap_or(style.border_alpha),
            ))
            .children(self.children);

        surface = self.background_policy.apply(
            surface,
            self.bg.unwrap_or(style.bg),
            self.bg_alpha.unwrap_or(style.bg_alpha),
        );

        if let Some(bounds) = self.bounds {
            surface = surface
                .absolute()
                .left(px(bounds.x))
                .top(px(bounds.y))
                .w(px(bounds.w))
                .h(px(bounds.h));
        }

        if let Some(id) = self.id {
            surface.id(id).into_any_element()
        } else {
            surface.into_any_element()
        }
    }
}

impl Default for MoonSurface {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for MoonSurface {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for MoonSurface {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        self.build(cx)
    }
}
