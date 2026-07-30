//! Dense themed status-bar primitives with text, inline dots, group dividers, and edge regions.

use std::fmt;

use crate::status_bar::StatusBar as CoreStatusBar;
use gpui::*;

use super::{
    foundation::MoonClickHandler,
    text::MoonText,
    theme::{MoonTheme, MoonThemeTokens},
    tokens::{MoonPalette, MoonRect, MoonTone, rgba_from},
    tooltip::MoonTooltipView,
};

/// Visual roles available to an item in the compact status row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MoonStatusItemKind {
    /// A monospaced label or value.
    Text,
    /// A compact dot separating closely related values inside one group.
    Separator,
    /// A vertical rule separating distinct semantic groups.
    GroupSeparator,
}

/// One status-bar label, compact separator, or semantic group divider.
///
/// Text items may own their stable identity, tooltip, and click handler so applications do not
/// need overlay hitboxes whose geometry can drift from the rendered label.
#[derive(Clone)]
pub struct MoonStatusItem {
    kind: MoonStatusItemKind,
    id: Option<SharedString>,
    text: SharedString,
    color: Option<u32>,
    tone: Option<MoonTone>,
    alpha: f32,
    weight: f32,
    gap_after: Option<f32>,
    tooltip: Option<SharedString>,
    on_click: Option<MoonClickHandler>,
}

impl fmt::Debug for MoonStatusItem {
    /// Format the item without attempting to print its opaque callback.
    ///
    /// Args:
    ///     f: Debug formatter receiving the visible configuration and interaction presence.
    ///
    /// Returns:
    ///     The formatter result.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MoonStatusItem")
            .field("kind", &self.kind)
            .field("id", &self.id)
            .field("text", &self.text)
            .field("color", &self.color)
            .field("tone", &self.tone)
            .field("alpha", &self.alpha)
            .field("weight", &self.weight)
            .field("gap_after", &self.gap_after)
            .field("tooltip", &self.tooltip)
            .field("interactive", &self.on_click.is_some())
            .finish()
    }
}

impl MoonStatusItem {
    /// Build a non-interactive text item.
    ///
    /// Args:
    ///     text: Monospaced label or value rendered in the status row.
    ///
    /// Returns:
    ///     A text item that can optionally receive identity, tooltip, and click behavior.
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self {
            kind: MoonStatusItemKind::Text,
            id: None,
            text: text.into(),
            color: None,
            tone: None,
            alpha: 1.0,
            weight: 400.0,
            gap_after: None,
            tooltip: None,
            on_click: None,
        }
    }

    /// Build the compact dot used between closely related status values.
    ///
    /// Returns:
    ///     A non-interactive separator item.
    pub fn separator() -> Self {
        Self {
            kind: MoonStatusItemKind::Separator,
            id: None,
            text: SharedString::from(""),
            color: None,
            tone: None,
            alpha: 0.74,
            weight: 400.0,
            gap_after: None,
            tooltip: None,
            on_click: None,
        }
    }

    /// Build a vertical rule between distinct semantic groups.
    ///
    /// Returns:
    ///     A non-shrinking divider item that follows the active palette and UI scale.
    pub fn group_separator() -> Self {
        Self {
            kind: MoonStatusItemKind::GroupSeparator,
            id: None,
            text: SharedString::from(""),
            color: None,
            tone: None,
            alpha: 1.0,
            weight: 400.0,
            gap_after: None,
            tooltip: None,
            on_click: None,
        }
    }

    /// Set a stable element and debug-selector identity for this item.
    ///
    /// Args:
    ///     id: Identity used by GPUI interaction state and rendered-geometry probes.
    ///
    /// Returns:
    ///     The updated item.
    pub fn id(mut self, id: impl Into<SharedString>) -> Self {
        self.id = Some(id.into());
        self
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

    /// Attach a Moon tooltip to a text item.
    ///
    /// Separators keep their visual-only role and ignore this setting.
    ///
    /// Args:
    ///     tooltip: Text shown while the pointer hovers the rendered item.
    ///
    /// Returns:
    ///     The updated item.
    pub fn tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// Make a text item dispatch native click events from its rendered bounds.
    ///
    /// Separators keep their visual-only role and ignore this setting.
    ///
    /// Args:
    ///     handler: Callback invoked for a click on the rendered text item.
    ///
    /// Returns:
    ///     The updated item.
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(std::rc::Rc::new(handler));
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

        let item_id_prefix = id.to_string();
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
            &item_id_prefix,
            "left",
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
                &item_id_prefix,
                "right",
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

    /// Render ordered text and separator items into one edge region.
    ///
    /// Args:
    ///     row: Flex row that receives the rendered items.
    ///     items: Ordered status content for this region.
    ///     status_bar_id: Parent identity used to scope fallback item identities.
    ///     region: Stable edge-region name used by fallback item identities.
    ///     item_gap: Default trailing gap in unscaled design units.
    ///     font_size: Monospaced text size in unscaled design units.
    ///     line_height: Text line height in unscaled design units.
    ///     p: Active palette for default colors and tones.
    ///     tokens: Active theme scaling and typography tokens.
    ///
    /// Returns:
    ///     The supplied row with all status items appended.
    fn render_items(
        mut row: Div,
        items: Vec<MoonStatusItem>,
        status_bar_id: &str,
        region: &'static str,
        item_gap: f32,
        font_size: f32,
        line_height: f32,
        p: MoonPalette,
        tokens: &MoonThemeTokens,
    ) -> Div {
        for (index, item) in items.into_iter().enumerate() {
            let color = item
                .color
                .or_else(|| item.tone.map(|tone| tone.color(p)))
                .unwrap_or(match item.kind {
                    MoonStatusItemKind::GroupSeparator => p.border_hover,
                    MoonStatusItemKind::Text | MoonStatusItemKind::Separator => p.text_soft,
                });
            let gap = tokens.ui(item.gap_after.unwrap_or(item_gap));
            match item.kind {
                MoonStatusItemKind::Text => {
                    let text_item = div().mr(px(gap)).child(
                        MoonText::new(item.text)
                            .uppercase(false)
                            .mono(true)
                            .color(color)
                            .alpha(item.alpha)
                            .font_size(font_size)
                            .line_height(line_height)
                            .weight(item.weight)
                            .render(),
                    );
                    let id = item.id.or_else(|| {
                        (item.tooltip.is_some() || item.on_click.is_some()).then(|| {
                            SharedString::from(format!("{status_bar_id}:item:{region}:{index}"))
                        })
                    });
                    if let Some(id) = id {
                        let debug_id = id.to_string();
                        let mut interactive_item = text_item
                            .id(ElementId::from(id))
                            .debug_selector(move || debug_id.clone());
                        if let Some(tooltip) = item.tooltip {
                            interactive_item = interactive_item.tooltip(move |_window, cx| {
                                cx.new(|_| MoonTooltipView::new(tooltip.clone())).into()
                            });
                        }
                        if let Some(on_click) = item.on_click {
                            interactive_item = interactive_item
                                .cursor_pointer()
                                .on_click(move |event, window, cx| on_click(event, window, cx));
                        }
                        row = row.child(interactive_item);
                    } else {
                        row = row.child(text_item);
                    }
                }
                MoonStatusItemKind::Separator => {
                    let size = tokens.ui(2.0);
                    let separator = div()
                        .w(px(size))
                        .h(px(size))
                        .rounded(px(size * 0.5))
                        .bg(rgba_from(color, item.alpha))
                        .mr(px(gap));
                    if let Some(id) = item.id {
                        let debug_id = id.to_string();
                        row = row.child(
                            separator
                                .id(ElementId::from(id))
                                .debug_selector(move || debug_id.clone()),
                        );
                    } else {
                        row = row.child(separator);
                    }
                }
                MoonStatusItemKind::GroupSeparator => {
                    let separator = div()
                        .flex_none()
                        .w(px(tokens.ui(1.0)))
                        .h(px(tokens.ui(12.0)))
                        .bg(rgba_from(color, item.alpha))
                        .mr(px(gap));
                    if let Some(id) = item.id {
                        let debug_id = id.to_string();
                        row = row.child(
                            separator
                                .id(ElementId::from(id))
                                .debug_selector(move || debug_id.clone()),
                        );
                    } else {
                        row = row.child(separator);
                    }
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

#[cfg(test)]
mod tests;
