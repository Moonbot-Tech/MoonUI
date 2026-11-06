use gpui::prelude::FluentBuilder as _;
use gpui::*;

use super::{
    icons::{MOON_ICON_TOOLTIP_ARROW, moon_icon},
    skin::{MoonFacet, MoonFacetKind, facet},
    text::MoonText,
    theme::{MoonTheme, MoonThemeTokens},
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

impl TooltipMetrics {
    fn scaled(self, tokens: &MoonThemeTokens) -> Self {
        let line_height = tokens.line_height(self.line_height);
        Self {
            min_height: tokens
                .ui(self.min_height)
                .max(line_height + tokens.ui(self.pad_y) * 2.0),
            font_size: self.font_size,
            line_height: self.line_height,
            pad_x: tokens.ui(self.pad_x),
            pad_y: tokens.ui(self.pad_y),
            radius: tokens.ui(self.radius),
            gap: tokens.ui(self.gap),
            arrow_size: tokens.ui(self.arrow_size),
        }
    }
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
    /// Render the tooltip box, its text or custom children, an optional shortcut and the arrow.
    ///
    /// The box is the solid Moon surface under the Flat skin and a popup surface under the
    /// Facet skin; see [`tooltip_facet`].
    ///
    /// Args:
    ///     _window: Window the tooltip renders in; unused.
    ///     cx: Application context used to resolve the active palette and theme tokens.
    ///
    /// Returns:
    ///     The rendered tooltip.
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = MoonPalette::active(cx);
        let tokens = MoonTheme::active_tokens(cx);
        let metrics = self.metrics().scaled(&tokens);
        let accent = self.tone.color(p);
        let has_custom_children = !self.children.is_empty();
        let placement = self.placement;

        let shadow = super::foundation::box_shadow(
            px(0.0),
            px(8.0),
            px(18.0),
            px(0.0),
            rgba_from(p.shadow, 0.46),
        );

        let mut root = div()
            .id(ElementId::from(self.id.clone()))
            .relative()
            .min_h(px(metrics.min_height))
            // `None` is the Flat skin. A faceted tooltip is a popup surface: gradient fill, lit
            // edge and the deeper shadow of something floating above the window.
            .map(|this| match tooltip_facet(&tokens, self.size) {
                Some((surface, radius)) => this
                    .rounded(px(radius))
                    .border(px(1.0))
                    .border_color(surface.edge)
                    .bg(surface.fill)
                    .shadow(surface.shadows),
                None => this
                    .rounded(px(metrics.radius))
                    .border(px(1.0))
                    .border_color(rgba_from(p.border, 1.0))
                    .bg(rgba_from(p.shell_high, 0.98))
                    .shadow(vec![shadow]),
            })
            .occlude()
            .flex()
            .items_center()
            .gap(px(metrics.gap))
            .px(px(metrics.pad_x))
            .py(px(metrics.pad_y))
            .font_family(tokens.font_family(self.mono))
            .text_size(px(tokens.font(metrics.font_size)))
            .line_height(px(tokens.line_height(metrics.line_height)))
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

        let mut body = div()
            .flex()
            .flex_col()
            .min_w_0()
            .gap(px(tokens.ui(1.0)))
            .flex_1();
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
                        .wrap()
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
                        .wrap()
                        .render(),
                );
            }
        }
        root = root.child(body);

        if let Some(shortcut) = self.shortcut {
            root = root.child(
                div()
                    .flex_shrink_0()
                    .rounded(px(tokens.ui(3.0)))
                    .border(px(1.0))
                    .border_color(rgba_from(p.border, 0.9))
                    .bg(rgba_from(p.panel, 0.88))
                    .px(px(tokens.ui(5.0)))
                    .py(px(tokens.ui(1.0)))
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
            root = root.child(Self::render_arrow(p, metrics, placement, &tokens));
        }

        root
    }
}

/// Design-reference corner radius of a normal tooltip under the Facet skin.
const FACET_TOOLTIP_RADIUS: f32 = 8.0;
/// Design-reference corner radius of a compact tooltip under the Facet skin.
const FACET_TOOLTIP_COMPACT_RADIUS: f32 = 6.0;

/// Resolve the surface a tooltip paints under the Facet skin.
///
/// Args:
///     tokens: Active theme tokens; supply the skin, the palette and the UI scale.
///     size: The tooltip's size; a custom size keeps the radius its caller chose.
///
/// Returns:
///     The popup surface and the corner radius in rendered pixels, or `None` under the Flat
///     skin, where the tooltip keeps its solid fill, its palette border and its soft shadow.
fn tooltip_facet(tokens: &MoonThemeTokens, size: MoonTooltipSize) -> Option<(MoonFacet, f32)> {
    if !tokens.skin.is_facet() {
        return None;
    }
    let radius = match size {
        MoonTooltipSize::Compact => FACET_TOOLTIP_COMPACT_RADIUS,
        MoonTooltipSize::Normal => FACET_TOOLTIP_RADIUS,
        MoonTooltipSize::Custom { radius, .. } => radius,
    };
    Some((facet(MoonFacetKind::Popup, tokens), tokens.ui(radius)))
}

/// Resolve the colour of the tooltip arrow, as `0xRRGGBB` and alpha.
///
/// The arrow continues the tooltip's fill past one of its edges. Under the Flat skin that fill
/// is one colour. A faceted tooltip is lit from above — its fill runs from `panel_high` at the
/// top down to `card` — so the arrow takes the colour of the edge it hangs from: `card` below
/// a tooltip placed above its target, `panel_high` on the top edge and near the top of a side.
///
/// Args:
///     p: Palette the tooltip paints with.
///     tokens: Active theme tokens; supply the skin.
///     placement: Where the tooltip sits relative to its target.
///
/// Returns:
///     The arrow's colour and alpha.
fn tooltip_arrow_color(
    p: MoonPalette,
    tokens: &MoonThemeTokens,
    placement: MoonTooltipPlacement,
) -> (u32, f32) {
    if !tokens.skin.is_facet() {
        return (p.shell_high, 0.98);
    }
    match placement {
        MoonTooltipPlacement::Top => (p.card, 1.0),
        MoonTooltipPlacement::Bottom | MoonTooltipPlacement::Left | MoonTooltipPlacement::Right => {
            (p.panel_high, 1.0)
        }
    }
}

impl MoonTooltip {
    /// Render the arrow that points from the tooltip to its target.
    ///
    /// Args:
    ///     p: Palette the tooltip paints with.
    ///     metrics: Scaled tooltip metrics; supply the arrow size.
    ///     placement: Where the tooltip sits relative to its target, which picks the edge the
    ///         arrow hangs from and its rotation.
    ///     tokens: Active theme tokens; supply the UI scale and the skin.
    ///
    /// Returns:
    ///     The absolutely positioned arrow, coloured to continue the tooltip's fill.
    fn render_arrow(
        p: MoonPalette,
        metrics: TooltipMetrics,
        placement: MoonTooltipPlacement,
        tokens: &MoonThemeTokens,
    ) -> Svg {
        let half = metrics.arrow_size * 0.5;
        let offset = tokens.ui(18.0);
        let (arrow_color, arrow_alpha) = tooltip_arrow_color(p, tokens, placement);
        let mut arrow = moon_icon(
            MOON_ICON_TOOLTIP_ARROW,
            metrics.arrow_size,
            arrow_color,
            arrow_alpha,
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

#[cfg(test)]
mod tests;
