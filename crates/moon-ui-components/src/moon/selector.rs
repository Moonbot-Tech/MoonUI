use gpui::prelude::FluentBuilder;
use gpui::*;

use super::{
    foundation::box_shadow,
    icons::{MOON_ICON_CARET_DOWN, moon_icon},
    text::MoonText,
    theme::MoonTheme,
    tokens::{MoonRect, rgba_from},
};

#[derive(Clone, Debug)]
pub struct MoonSelectorSegment {
    text: SharedString,
    color: Option<u32>,
    alpha: f32,
    font_size: f32,
    line_height: f32,
    weight: f32,
    margin_top: f32,
}

impl MoonSelectorSegment {
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self {
            text: text.into(),
            color: None,
            alpha: 1.0,
            font_size: 10.5,
            line_height: 13.0,
            weight: 400.0,
            margin_top: 1.0,
        }
    }

    pub fn color(mut self, color: u32) -> Self {
        self.color = Some(color);
        self
    }

    pub fn alpha(mut self, alpha: f32) -> Self {
        self.alpha = alpha;
        self
    }

    pub fn font_size(mut self, font_size: f32) -> Self {
        self.font_size = font_size;
        self
    }

    pub fn line_height(mut self, line_height: f32) -> Self {
        self.line_height = line_height;
        self
    }

    pub fn weight(mut self, weight: f32) -> Self {
        self.weight = weight;
        self
    }

    pub fn margin_top(mut self, margin_top: f32) -> Self {
        self.margin_top = margin_top;
        self
    }
}

#[derive(IntoElement)]
pub struct MoonSelectorPill {
    id: ElementId,
    bounds: Option<MoonRect>,
    segments: Vec<MoonSelectorSegment>,
    leading_dot: Option<u32>,
    disabled: bool,
    height: f32,
    radius: f32,
    pad_left: f32,
    pad_right: f32,
    gap: f32,
    dot_size: f32,
    dot_glow: f32,
    caret: bool,
    caret_right: f32,
    caret_top: f32,
}

impl MoonSelectorPill {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            bounds: None,
            segments: Vec::new(),
            leading_dot: None,
            disabled: false,
            height: 24.0,
            radius: 12.0,
            pad_left: 9.0,
            pad_right: 23.0,
            gap: 6.0,
            dot_size: 4.0,
            dot_glow: 6.0,
            caret: true,
            caret_right: 11.0,
            caret_top: 5.0,
        }
    }

    pub fn bounds(mut self, bounds: MoonRect) -> Self {
        self.bounds = Some(bounds);
        self
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.segments = vec![MoonSelectorSegment::new(label)];
        self
    }

    pub fn segment(mut self, segment: MoonSelectorSegment) -> Self {
        self.segments.push(segment);
        self
    }

    pub fn segments(mut self, segments: impl IntoIterator<Item = MoonSelectorSegment>) -> Self {
        self.segments.extend(segments);
        self
    }

    pub fn leading_dot(mut self, color: u32) -> Self {
        self.leading_dot = Some(color);
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

    pub fn padding(mut self, left: f32, right: f32) -> Self {
        self.pad_left = left;
        self.pad_right = right;
        self
    }

    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    pub fn dot_size(mut self, dot_size: f32) -> Self {
        self.dot_size = dot_size;
        self
    }

    pub fn dot_glow(mut self, dot_glow: f32) -> Self {
        self.dot_glow = dot_glow;
        self
    }

    pub fn caret(mut self, caret: bool) -> Self {
        self.caret = caret;
        self
    }

    pub fn caret_offset(mut self, right: f32, top: f32) -> Self {
        self.caret_right = right;
        self.caret_top = top;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn render(self) -> impl IntoElement {
        self
    }
}

impl RenderOnce for MoonSelectorPill {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = MoonTheme::active_tokens(cx);
        let p = tokens.palette;
        let disabled = self.disabled;
        let text_alpha = if disabled { 0.45 } else { 1.0 };

        let mut root = div()
            .id(self.id)
            .relative()
            .h(px(tokens.ui(self.height)))
            .rounded(px(tokens.ui(self.radius)))
            .border(px(tokens.ui(1.0)))
            .border_color(rgba_from(p.border, if disabled { 0.55 } else { 1.0 }))
            .bg(rgba_from(p.panel, if disabled { 0.48 } else { 1.0 }))
            .flex()
            .items_center()
            .pl(px(tokens.ui(self.pad_left)))
            .pr(px(tokens.ui(self.pad_right)))
            .gap(px(tokens.ui(self.gap)))
            .cursor_default()
            .when(!disabled, |this| {
                this.hover(|this| {
                    this.bg(rgba_from(p.panel_high, 1.0))
                        .border_color(rgba_from(p.border_hover, 1.0))
                })
                .active(|this| this.bg(rgba_from(p.shell_high, 1.0)))
            });

        if let Some(bounds) = self.bounds {
            root = root
                .absolute()
                .left(px(bounds.x))
                .top(px(bounds.y))
                .w(px(bounds.w))
                .h(px(bounds.h));
        }

        if let Some(dot) = self.leading_dot {
            root = root.child(
                div()
                    .w(px(tokens.ui(self.dot_size)))
                    .h(px(tokens.ui(self.dot_size)))
                    .rounded(px(tokens.ui(self.dot_size * 0.5)))
                    .bg(rgba_from(dot, if disabled { 0.45 } else { 1.0 }))
                    .shadow(vec![box_shadow(
                        px(0.0),
                        px(0.0),
                        px(tokens.ui(self.dot_glow)),
                        px(0.0),
                        rgba_from(dot, 0.65),
                    )]),
            );
        }

        for segment in self.segments {
            let color = segment.color.unwrap_or(p.text_soft);
            root = root.child(
                div().mt(px(tokens.ui(segment.margin_top))).child(
                    MoonText::new(segment.text)
                        .uppercase(false)
                        .mono(true)
                        .color(color)
                        .alpha(text_alpha * segment.alpha)
                        .font_size(segment.font_size)
                        .line_height(segment.line_height)
                        .weight(segment.weight)
                        .render(),
                ),
            );
        }

        if self.caret {
            root = root.child(
                moon_icon(
                    MOON_ICON_CARET_DOWN,
                    tokens.ui(8.0),
                    p.text_muted,
                    if disabled { 0.45 } else { 1.0 },
                )
                .absolute()
                .right(px(tokens.ui(self.caret_right)))
                .top(px(tokens.ui(self.caret_top))),
            );
        }

        root
    }
}
