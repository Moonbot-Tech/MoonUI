use gpui::*;

use super::{
    icons::{MOON_ICON_TOOLTIP_ARROW, moon_icon},
    text::MoonText,
    tokens::{MoonPalette, MoonRect, MoonTone, rgba_from},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoonTooltipPlacement {
    Top,
    Bottom,
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MoonTooltipSize {
    Compact,
    Normal,
    Custom {
        min_height: f32,
        font_size: f32,
        line_height: f32,
        pad_x: f32,
        pad_y: f32,
        radius: f32,
        gap: f32,
        arrow_size: f32,
    },
}

#[derive(Clone, Copy, Debug)]
struct TooltipMetrics {
    min_height: f32,
    font_size: f32,
    line_height: f32,
    pad_x: f32,
    pad_y: f32,
    radius: f32,
    gap: f32,
    arrow_size: f32,
}

#[derive(IntoElement)]
pub struct MoonTooltip {
    id: SharedString,
    bounds: Option<MoonRect>,
    text: Option<SharedString>,
    detail: Option<SharedString>,
    shortcut: Option<SharedString>,
    children: Vec<AnyElement>,
    placement: MoonTooltipPlacement,
    size: MoonTooltipSize,
    tone: MoonTone,
    width: Option<f32>,
    max_width: Option<f32>,
    arrow: bool,
    mono: bool,
}

pub struct MoonTooltipView {
    text: SharedString,
    max_width: Option<f32>,
}

impl MoonTooltipView {
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self {
            text: text.into(),
            max_width: None,
        }
    }

    pub fn max_width(mut self, max_width: f32) -> Self {
        self.max_width = Some(max_width);
        self
    }
}

impl Render for MoonTooltipView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut tooltip = MoonTooltip::new(self.text.clone());
        if let Some(max_width) = self.max_width {
            tooltip = tooltip.max_width(max_width);
        }
        tooltip.render(window, cx)
    }
}

impl MoonTooltip {
    pub fn new(text: impl Into<SharedString>) -> Self {
        let text = text.into();
        Self {
            id: SharedString::from(format!("moon-tooltip:{}", text)),
            bounds: None,
            text: Some(text),
            detail: None,
            shortcut: None,
            children: Vec::new(),
            placement: MoonTooltipPlacement::Top,
            size: MoonTooltipSize::Normal,
            tone: MoonTone::Default,
            width: None,
            max_width: Some(260.0),
            arrow: true,
            mono: true,
        }
    }

    pub fn with_id(id: impl Into<SharedString>, text: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            ..Self::new(text)
        }
    }

    pub fn empty(id: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            bounds: None,
            text: None,
            detail: None,
            shortcut: None,
            children: Vec::new(),
            placement: MoonTooltipPlacement::Top,
            size: MoonTooltipSize::Normal,
            tone: MoonTone::Default,
            width: None,
            max_width: Some(260.0),
            arrow: true,
            mono: true,
        }
    }

    pub fn bounds(mut self, bounds: MoonRect) -> Self {
        self.bounds = Some(bounds);
        self
    }

    pub fn detail(mut self, detail: impl Into<SharedString>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn shortcut(mut self, shortcut: impl Into<SharedString>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    pub fn placement(mut self, placement: MoonTooltipPlacement) -> Self {
        self.placement = placement;
        self
    }

    pub fn size(mut self, size: MoonTooltipSize) -> Self {
        self.size = size;
        self
    }

    pub fn tone(mut self, tone: MoonTone) -> Self {
        self.tone = tone;
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    pub fn max_width(mut self, max_width: f32) -> Self {
        self.max_width = Some(max_width);
        self
    }

    pub fn arrow(mut self, arrow: bool) -> Self {
        self.arrow = arrow;
        self
    }

    pub fn mono(mut self, mono: bool) -> Self {
        self.mono = mono;
        self
    }

    fn metrics(&self) -> TooltipMetrics {
        match self.size {
            MoonTooltipSize::Compact => TooltipMetrics {
                min_height: 22.0,
                font_size: 10.0,
                line_height: 13.0,
                pad_x: 8.0,
                pad_y: 4.0,
                radius: 4.0,
                gap: 8.0,
                arrow_size: 7.0,
            },
            MoonTooltipSize::Normal => TooltipMetrics {
                min_height: 28.0,
                font_size: 10.5,
                line_height: 14.0,
                pad_x: 9.0,
                pad_y: 5.0,
                radius: 5.0,
                gap: 10.0,
                arrow_size: 8.0,
            },
            MoonTooltipSize::Custom {
                min_height,
                font_size,
                line_height,
                pad_x,
                pad_y,
                radius,
                gap,
                arrow_size,
            } => TooltipMetrics {
                min_height,
                font_size,
                line_height,
                pad_x,
                pad_y,
                radius,
                gap,
                arrow_size,
            },
        }
    }
}

impl ParentElement for MoonTooltip {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for MoonTooltip {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = MoonPalette::active(cx);
        let metrics = self.metrics();
        let accent = self.tone.color(p);
        let has_custom_children = !self.children.is_empty();
        let placement = self.placement;

        let shadow = super::foundation::box_shadow(
            px(0.0),
            px(8.0),
            px(18.0),
            px(0.0),
            rgba_from(0x000000, 0.46),
        );

        let mut root = div()
            .id(ElementId::from(self.id.clone()))
            .relative()
            .min_h(px(metrics.min_height))
            .rounded(px(metrics.radius))
            .border(px(1.0))
            .border_color(rgba_from(p.border, 1.0))
            .bg(rgba_from(p.shell_high, 0.98))
            .shadow(vec![shadow])
            .occlude()
            .flex()
            .items_center()
            .gap(px(metrics.gap))
            .px(px(metrics.pad_x))
            .py(px(metrics.pad_y))
            .font_family(if self.mono { "Geist Mono" } else { "Inter" })
            .text_size(px(metrics.font_size))
            .line_height(px(metrics.line_height))
            .text_color(rgba_from(p.text_soft, 1.0));

        if let Some(bounds) = self.bounds {
            root = root
                .absolute()
                .left(px(bounds.x))
                .top(px(bounds.y))
                .w(px(bounds.w))
                .h(px(bounds.h));
        } else {
            if let Some(width) = self.width {
                root = root.w(px(width));
            }
            if let Some(max_width) = self.max_width {
                root = root.max_w(px(max_width));
            }
        }

        let mut body = div().flex().flex_col().gap(px(1.0)).flex_1();
        if has_custom_children {
            body = body.children(self.children);
        } else {
            if let Some(text) = self.text {
                body = body.child(
                    MoonText::new(text)
                        .color(accent)
                        .font_size(metrics.font_size)
                        .line_height(metrics.line_height)
                        .weight(500.0)
                        .mono(self.mono)
                        .uppercase(false)
                        .render(),
                );
            }
            if let Some(detail) = self.detail {
                body = body.child(
                    MoonText::new(detail)
                        .color(p.text_muted)
                        .font_size(metrics.font_size - 1.0)
                        .line_height(metrics.line_height - 1.0)
                        .weight(400.0)
                        .mono(self.mono)
                        .uppercase(false)
                        .render(),
                );
            }
        }
        root = root.child(body);

        if let Some(shortcut) = self.shortcut {
            root = root.child(
                div()
                    .flex_shrink_0()
                    .rounded(px(3.0))
                    .border(px(1.0))
                    .border_color(rgba_from(p.border, 0.9))
                    .bg(rgba_from(p.panel, 0.88))
                    .px(px(5.0))
                    .py(px(1.0))
                    .child(
                        MoonText::new(shortcut)
                            .color(p.text_muted)
                            .font_size(metrics.font_size - 1.0)
                            .line_height(metrics.line_height - 1.0)
                            .weight(500.0)
                            .mono(self.mono)
                            .uppercase(false)
                            .render(),
                    ),
            );
        }

        if self.arrow {
            root = root.child(Self::render_arrow(p, metrics, placement));
        }

        root
    }
}

impl MoonTooltip {
    fn render_arrow(
        p: MoonPalette,
        metrics: TooltipMetrics,
        placement: MoonTooltipPlacement,
    ) -> Svg {
        let half = metrics.arrow_size * 0.5;
        let offset = 18.0;
        let mut arrow = moon_icon(
            MOON_ICON_TOOLTIP_ARROW,
            metrics.arrow_size,
            p.shell_high,
            0.98,
        )
        .absolute();

        match placement {
            MoonTooltipPlacement::Top => {
                arrow = arrow.left(px(offset)).bottom(px(-half));
            }
            MoonTooltipPlacement::Bottom => {
                arrow = arrow
                    .left(px(offset))
                    .top(px(-half))
                    .with_transformation(Transformation::rotate(percentage(0.5)));
            }
            MoonTooltipPlacement::Left => {
                arrow = arrow
                    .right(px(-half))
                    .top(px(offset))
                    .with_transformation(Transformation::rotate(percentage(0.75)));
            }
            MoonTooltipPlacement::Right => {
                arrow = arrow
                    .left(px(-half))
                    .top(px(offset))
                    .with_transformation(Transformation::rotate(percentage(0.25)));
            }
        }

        arrow
    }
}
