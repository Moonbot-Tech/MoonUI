use crate::status_bar::StatusBar as CoreStatusBar;
use gpui::*;

use super::{
    text::MoonText,
    theme::{MoonTheme, MoonThemeTokens},
    tokens::{MoonPalette, MoonRect, MoonTone, rgba_from},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MoonStatusItemKind {
    Text,
    Separator,
}

#[derive(Clone, Debug)]
pub struct MoonStatusItem {
    kind: MoonStatusItemKind,
    text: SharedString,
    color: Option<u32>,
    tone: Option<MoonTone>,
    alpha: f32,
    weight: f32,
    gap_after: Option<f32>,
}

impl MoonStatusItem {
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self {
            kind: MoonStatusItemKind::Text,
            text: text.into(),
            color: None,
            tone: None,
            alpha: 1.0,
            weight: 400.0,
            gap_after: None,
        }
    }

    pub fn separator() -> Self {
        Self {
            kind: MoonStatusItemKind::Separator,
            text: SharedString::from(""),
            color: None,
            tone: None,
            alpha: 0.74,
            weight: 400.0,
            gap_after: None,
        }
    }

    pub fn color(mut self, color: u32) -> Self {
        self.color = Some(color);
        self
    }

    pub fn tone(mut self, tone: MoonTone) -> Self {
        self.tone = Some(tone);
        self
    }

    pub fn alpha(mut self, alpha: f32) -> Self {
        self.alpha = alpha;
        self
    }

    pub fn weight(mut self, weight: f32) -> Self {
        self.weight = weight;
        self
    }

    pub fn gap_after(mut self, gap_after: f32) -> Self {
        self.gap_after = Some(gap_after);
        self
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MoonStatusIndicator {
    color: u32,
    alpha: f32,
    size: f32,
    glow: Option<(f32, f32)>,
}

impl MoonStatusIndicator {
    pub fn new(color: u32) -> Self {
        Self {
            color,
            alpha: 1.0,
            size: 6.0,
            glow: None,
        }
    }

    pub fn alpha(mut self, alpha: f32) -> Self {
        self.alpha = alpha;
        self
    }

    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    pub fn glow(mut self, radius: f32, alpha: f32) -> Self {
        self.glow = Some((radius, alpha));
        self
    }
}

#[derive(IntoElement)]
pub struct MoonStatusBar {
    id: ElementId,
    bounds: Option<MoonRect>,
    items: Vec<MoonStatusItem>,
    right_items: Vec<MoonStatusItem>,
    indicator: Option<MoonStatusIndicator>,
    height: f32,
    left_pad: f32,
    right_offset: f32,
    item_gap: f32,
    indicator_gap: f32,
    font_size: f32,
    line_height: f32,
    bg: u32,
    border: u32,
}

impl MoonStatusBar {
    pub fn new(id: impl Into<ElementId>) -> Self {
        let p = MoonPalette::TERMINAL;
        Self {
            id: id.into(),
            bounds: None,
            items: Vec::new(),
            right_items: Vec::new(),
            indicator: None,
            height: 22.0,
            left_pad: 12.0,
            right_offset: 8.0,
            item_gap: 10.0,
            indicator_gap: 6.0,
            font_size: 10.0,
            line_height: 13.0,
            bg: p.shell_high,
            border: p.border,
        }
    }

    pub fn bounds(mut self, bounds: MoonRect) -> Self {
        self.bounds = Some(bounds);
        self
    }

    pub fn indicator(mut self, indicator: MoonStatusIndicator) -> Self {
        self.indicator = Some(indicator);
        self
    }

    pub fn item(mut self, item: MoonStatusItem) -> Self {
        self.items.push(item);
        self
    }

    pub fn items(mut self, items: impl IntoIterator<Item = MoonStatusItem>) -> Self {
        self.items.extend(items);
        self
    }

    pub fn right_item(mut self, item: MoonStatusItem) -> Self {
        self.right_items.push(item);
        self
    }

    pub fn right_items(mut self, items: impl IntoIterator<Item = MoonStatusItem>) -> Self {
        self.right_items.extend(items);
        self
    }

    pub fn right_offset(mut self, right_offset: f32) -> Self {
        self.right_offset = right_offset;
        self
    }

    pub fn item_gap(mut self, item_gap: f32) -> Self {
        self.item_gap = item_gap;
        self
    }

    pub fn render(self) -> impl IntoElement {
        self
    }

    pub fn render_with_palette(self, p: MoonPalette) -> impl IntoElement {
        self.render_with_theme(p, MoonThemeTokens::default())
    }

    pub fn render_with_theme(self, p: MoonPalette, tokens: MoonThemeTokens) -> impl IntoElement {
        let Self {
            id,
            bounds,
            items,
            right_items,
            indicator,
            height,
            left_pad,
            right_offset,
            item_gap,
            indicator_gap,
            font_size,
            line_height,
            bg,
            border,
        } = self;
        let text = tokens.text(font_size, line_height);
        let height = tokens
            .ui(height)
            .max(text.line_height + tokens.ui(((height - line_height) * 0.5).max(0.0)) * 2.0);
        let left_pad = tokens.ui(left_pad);
        let right_offset = tokens.ui(right_offset);
        let item_gap = tokens.ui(item_gap);
        let indicator_gap = tokens.ui(indicator_gap);
        let bg = if bg == MoonPalette::TERMINAL.shell_high {
            p.shell_high
        } else {
            bg
        };
        let border = if border == MoonPalette::TERMINAL.border {
            p.border
        } else {
            border
        };

        let mut root = div().id(id).relative().overflow_hidden().h(px(height));

        if let Some(bounds) = bounds {
            root = root
                .absolute()
                .left(px(bounds.x))
                .top(px(bounds.y))
                .w(px(bounds.w))
                .h(px(bounds.h));
        }

        let mut left_row = div().ml(px(left_pad)).h_full().flex().items_center();

        if let Some(indicator) = indicator {
            let indicator_size = tokens.ui(indicator.size);
            let mut dot = div()
                .w(px(indicator_size))
                .h(px(indicator_size))
                .mr(px(indicator_gap))
                .rounded(px(indicator_size * 0.5))
                .bg(rgba_from(indicator.color, indicator.alpha));

            if let Some((radius, alpha)) = indicator.glow {
                dot = dot.shadow(vec![super::foundation::box_shadow(
                    px(0.0),
                    px(0.0),
                    px(tokens.ui(radius)),
                    px(0.0),
                    rgba_from(indicator.color, alpha),
                )]);
            }

            left_row = left_row.child(dot);
        }

        let left_row = Self::render_items(
            left_row,
            items,
            item_gap,
            font_size,
            line_height,
            p,
            &tokens,
        );

        let mut status = CoreStatusBar::new()
            .left(left_row)
            .h(px(height))
            .w_full()
            .py(px(0.0))
            .px(px(0.0))
            .gap(px(0.0))
            .bg(rgba_from(bg, 1.0))
            .border_color(rgba_from(border, 1.0));

        if !right_items.is_empty() {
            let right_row = Self::render_items(
                div().mr(px(right_offset)).h_full().flex().items_center(),
                right_items,
                item_gap,
                font_size,
                line_height,
                p,
                &tokens,
            );
            status = status.right(right_row);
        }

        root.child(status)
    }

    fn render_items(
        mut row: Div,
        items: Vec<MoonStatusItem>,
        item_gap: f32,
        font_size: f32,
        line_height: f32,
        p: MoonPalette,
        tokens: &MoonThemeTokens,
    ) -> Div {
        for item in items {
            let color = item
                .color
                .or_else(|| item.tone.map(|tone| tone.color(p)))
                .unwrap_or(p.text_soft);
            let gap = tokens.ui(item.gap_after.unwrap_or(item_gap));
            match item.kind {
                MoonStatusItemKind::Text => {
                    row = row.child(
                        div().mr(px(gap)).child(
                            MoonText::new(item.text)
                                .uppercase(false)
                                .mono(true)
                                .color(color)
                                .alpha(item.alpha)
                                .font_size(font_size)
                                .line_height(line_height)
                                .weight(item.weight)
                                .render(),
                        ),
                    );
                }
                MoonStatusItemKind::Separator => {
                    let size = tokens.ui(2.0);
                    row = row.child(
                        div()
                            .w(px(size))
                            .h(px(size))
                            .rounded(px(size * 0.5))
                            .bg(rgba_from(color, item.alpha))
                            .mr(px(gap)),
                    );
                }
            }
        }
        row
    }
}

impl RenderOnce for MoonStatusBar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = MoonTheme::active_tokens(cx);
        self.render_with_theme(MoonPalette::active(cx), tokens)
    }
}
