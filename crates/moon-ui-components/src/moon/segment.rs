use gpui::prelude::FluentBuilder;
use gpui::*;
use std::rc::Rc;

use super::{
    foundation::{MoonIndexedClickHandler, accent_underline_colored},
    text::{MoonText, fit_text_to_width, measure_text_width},
    theme::{MoonTheme, MoonThemeTokens},
    tokens::{MoonPalette, MoonRect, contrast_ratio, rgba_from},
    tooltip::MoonTooltipView,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoonAccent {
    Amber,
    Blue,
    Green,
    Red,
}

/// The WCAG 2.x contrast floor the selected label must clear against the panel.
///
/// AA for normal-size text. The label renders at 11px, below the 18px/14px-bold threshold that
/// would let the 3:1 large-text floor apply.
const LABEL_CONTRAST_FLOOR: f32 = 4.5;
const SEGMENT_PAD_X: f32 = 11.0;
const SEGMENT_GAP: f32 = 5.0;
const SEGMENT_HOTKEY_SIZE: f32 = 8.5;
const SEGMENT_HOTKEY_WEIGHT: f32 = 400.0;
const SEGMENT_LABEL_SIZE: f32 = 11.0;
const SEGMENT_LABEL_FIT_WEIGHT: f32 = 500.0;

type MoonIndexedScrollHandler = Rc<dyn Fn(usize, &ScrollWheelEvent, &mut Window, &mut App)>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Native interaction capabilities resolved for one rendered segment cell.
struct SegmentCellInteraction {
    click: bool,
    stop_click: bool,
    scroll: bool,
    tooltip: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
/// Inner spacing resolved for one rendered segment cell.
struct SegmentCellChrome {
    gap: f32,
    pad_x: f32,
}

/// Resolve which native interactions a rendered segment cell may expose.
///
/// Disabled cells and cells replaced by an inline editor are intentionally inert. A replacement
/// must not dispatch the same pointer gesture to both the editor and the underlying preset.
///
/// Args:
///     disabled: Whether the item is disabled.
///     replaced: Whether inline content replaces the normal cell.
///     has_click: Whether the control has a click handler.
///     has_scroll: Whether the control has a scroll handler.
///     has_tooltip: Whether the item has tooltip text.
///
/// Returns:
///     The click, scroll, and tooltip capabilities allowed for the cell.
fn segment_cell_interaction(
    disabled: bool,
    replaced: bool,
    has_click: bool,
    has_scroll: bool,
    has_tooltip: bool,
) -> SegmentCellInteraction {
    let enabled = !disabled && !replaced;
    SegmentCellInteraction {
        click: enabled && has_click,
        stop_click: disabled && !replaced && has_click,
        scroll: enabled && has_scroll,
        tooltip: enabled && has_tooltip,
    }
}

/// Resolve normal cell spacing or release the full width to an inline replacement.
///
/// Args:
///     tokens: Active theme tokens used to scale normal cell spacing.
///     replaced: Whether inline content replaces the hotkey and label.
///
/// Returns:
///     Scaled gap and horizontal padding for the rendered cell.
fn segment_cell_chrome(tokens: &MoonThemeTokens, replaced: bool) -> SegmentCellChrome {
    if replaced {
        SegmentCellChrome {
            gap: 0.0,
            pad_x: 0.0,
        }
    } else {
        SegmentCellChrome {
            gap: tokens.ui(SEGMENT_GAP),
            pad_x: tokens.ui(SEGMENT_PAD_X),
        }
    }
}

/// Fit one segment item using already-resolved layout metrics.
///
/// The selected label weight is used even while the item is unselected so selecting it never
/// changes the strip's total width.
///
/// Args:
///     hotkey: Hotkey text that must remain intact.
///     label: Full label that may be truncated.
///     min_width: Minimum rendered cell width.
///     max_width: Maximum rendered cell width.
///     chrome_width: Resolved component padding and gap.
///     measure_hotkey: Width function matching the hotkey style.
///     measure_label: Width function matching the selected label style.
///
/// Returns:
///     The fitted label and its final rendered cell width.
fn fit_segment_item(
    hotkey: &str,
    label: &str,
    min_width: f32,
    max_width: f32,
    chrome_width: f32,
    measure_hotkey: impl Fn(&str) -> f32,
    measure_label: impl Fn(&str) -> f32,
) -> (String, f32) {
    let hotkey_width = measure_hotkey(hotkey);
    let minimum_label_width = if label.is_empty() {
        0.0
    } else {
        measure_label("\u{2026}")
    };
    let minimum_chrome_width = (chrome_width + hotkey_width + minimum_label_width).ceil();
    let min_width = min_width.max(minimum_chrome_width);
    let max_width = max_width.max(min_width);
    let label_budget = (max_width - chrome_width - hotkey_width).max(0.0);
    let (label, label_width) = fit_text_to_width(label, label_budget, measure_label);
    let width = (chrome_width + hotkey_width + label_width)
        .ceil()
        .clamp(min_width, max_width);
    (label, width)
}

/// The two colours a selected segment draws with.
///
/// They are separate roles, not a redundancy: `text` sits on the panel background and carries the
/// legibility burden, while `underline` is a saturated decoration bar whose contrast against the
/// panel does not matter. Collapsing them into one value costs legibility — see
/// [`MoonAccent::colors`].
#[derive(Clone, Copy)]
struct AccentColors {
    text: u32,
    underline: u32,
}

impl MoonAccent {
    /// Resolve this accent to its selected-segment colours for `p`.
    ///
    /// The underline always takes the accent's own hue, in both themes — that bar is what makes
    /// two differently-accented strips readable as different, and its own contrast does not matter.
    ///
    /// The label cannot follow it unconditionally, so this MEASURES rather than assumes. Whether a
    /// hue is legible depends on the hue and the panel together, and the answer is not the same per
    /// theme: amber reads 8.8:1 on the dark panel but 3.5:1 on the light one, while blue clears the
    /// floor on both; dark green and red labels measure only 3.7:1 and 4.0:1 without a fallback.
    ///
    /// So the label takes the first candidate that clears [`LABEL_CONTRAST_FLOOR`]: the accent's own
    /// hue, then the palette's darkened companion where that hue has one, then `accent_fg`. If none
    /// of them clears it the plain text colour is taken unconditionally — a palette that leaves
    /// `accent_fg` illegible on its own panel has nothing better left to offer. A custom palette
    /// therefore degrades to something readable instead of inheriting a policy tuned for the two
    /// built-in ones.
    fn colors(self, p: MoonPalette) -> AccentColors {
        let underline = match self {
            Self::Amber => p.amber,
            Self::Blue => p.blue,
            Self::Green => p.green,
            Self::Red => p.red,
        };
        // The palette's purpose-built readable variant of this hue — `None` where it has none, so
        // the cascade falls straight through to `accent_fg` rather than weighing it twice.
        let companion = match self {
            Self::Green => Some(p.green_text),
            Self::Red => Some(p.red_text),
            Self::Amber | Self::Blue => None,
        };
        let text = [Some(underline), companion, Some(p.accent_fg)]
            .into_iter()
            .flatten()
            .find(|candidate| contrast_ratio(*candidate, p.panel) >= LABEL_CONTRAST_FLOOR)
            .unwrap_or(p.text);
        AccentColors { text, underline }
    }
}

#[derive(Clone, Debug)]
/// One segmented-control cell with pre-render fitting and optional tooltip behavior.
pub struct MoonSegmentItem {
    hotkey: SharedString,
    label: SharedString,
    width: f32,
    selected: bool,
    disabled: bool,
    tooltip: Option<SharedString>,
}

impl MoonSegmentItem {
    /// Create one segment item with the default rendered width.
    ///
    /// Args:
    ///     hotkey: Stable shortcut label preserved during fitting.
    ///     label: Value label that may be truncated by [`Self::fit_width`].
    ///
    /// Returns:
    ///     A default segment item.
    pub fn new(hotkey: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            hotkey: hotkey.into(),
            label: label.into(),
            width: 64.0,
            selected: false,
            disabled: false,
            tooltip: None,
        }
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Fit this item's rendered width between font-scaled design-reference bounds.
    ///
    /// The method runs before render so a parent can use [`Self::resolved_width`] for layout
    /// budgeting. It truncates only the label; the hotkey and the component's own scaled padding
    /// and gap remain reserved.
    ///
    /// Args:
    ///     cx: Application context used for theme-aware text measurement.
    ///     min_width: Minimum width at the configured monospaced body reference size.
    ///     max_width: Maximum width at the configured monospaced body reference size.
    ///
    /// Returns:
    ///     The updated fitted item.
    pub fn fit_width(mut self, cx: &App, min_width: f32, max_width: f32) -> Self {
        let tokens = MoonTheme::active_tokens(cx);
        let min_width = tokens.font_width(min_width);
        let max_width = tokens.font_width(max_width).max(min_width);
        let chrome_width = tokens.ui(SEGMENT_PAD_X) * 2.0 + tokens.ui(SEGMENT_GAP);
        let hotkey = self.hotkey.clone();
        let label = self.label.clone();
        let (label, width) = fit_segment_item(
            hotkey.as_ref(),
            label.as_ref(),
            min_width,
            max_width,
            chrome_width,
            |text| {
                measure_text_width(
                    cx,
                    &tokens,
                    text,
                    SEGMENT_HOTKEY_SIZE,
                    SEGMENT_HOTKEY_WEIGHT,
                    true,
                )
            },
            |text| {
                measure_text_width(
                    cx,
                    &tokens,
                    text,
                    SEGMENT_LABEL_SIZE,
                    SEGMENT_LABEL_FIT_WEIGHT,
                    true,
                )
            },
        );
        self.label = SharedString::from(label);
        self.width = width;
        self
    }

    /// Return the final rendered width selected by [`Self::width`] or [`Self::fit_width`].
    ///
    /// Returns:
    ///     The cell width in rendered pixels.
    pub fn resolved_width(&self) -> f32 {
        self.width
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set the tooltip displayed by this item while it is enabled and not replaced.
    ///
    /// Args:
    ///     tooltip: Tooltip text.
    ///
    /// Returns:
    ///     The updated item.
    pub fn tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }
}

#[derive(IntoElement)]
/// Moon segmented control with indexed click/scroll handling and inline cell replacement.
pub struct MoonSegmentedControl {
    id: ElementId,
    bounds: Option<MoonRect>,
    items: Vec<MoonSegmentItem>,
    accent: MoonAccent,
    item_gap: f32,
    on_click: Option<MoonIndexedClickHandler>,
    on_scroll: Option<MoonIndexedScrollHandler>,
    replacements: Vec<(usize, AnyElement)>,
}

impl MoonSegmentedControl {
    /// Create an empty segmented control with amber selection styling.
    ///
    /// Args:
    ///     id: Stable identity for the rendered control.
    ///
    /// Returns:
    ///     A default segmented-control builder.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            bounds: None,
            items: Vec::new(),
            accent: MoonAccent::Amber,
            item_gap: 0.0,
            on_click: None,
            on_scroll: None,
            replacements: Vec::new(),
        }
    }

    pub fn bounds(mut self, bounds: MoonRect) -> Self {
        self.bounds = Some(bounds);
        self
    }

    pub fn accent(mut self, accent: MoonAccent) -> Self {
        self.accent = accent;
        self
    }

    pub fn item(mut self, item: MoonSegmentItem) -> Self {
        self.items.push(item);
        self
    }

    pub fn items(mut self, items: impl IntoIterator<Item = MoonSegmentItem>) -> Self {
        self.items.extend(items);
        self
    }

    pub fn item_gap(mut self, item_gap: f32) -> Self {
        self.item_gap = item_gap;
        self
    }

    pub fn on_click(
        mut self,
        handler: impl Fn(usize, &ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(std::rc::Rc::new(handler));
        self
    }

    /// Handle a scroll-wheel event from an enabled, non-replaced item.
    ///
    /// Args:
    ///     handler: Indexed native scroll callback.
    ///
    /// Returns:
    ///     The updated segmented control.
    pub fn on_scroll(
        mut self,
        handler: impl Fn(usize, &ScrollWheelEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_scroll = Some(Rc::new(handler));
        self
    }

    /// Replace one item's normal contents with an inline element.
    ///
    /// Replaced cells preserve their resolved width and selected underline but expose no segment
    /// click, scroll, hover, cursor, or tooltip behavior.
    ///
    /// Args:
    ///     index: Zero-based item index to replace.
    ///     replacement: Inline element rendered inside the resolved cell width.
    ///
    /// Returns:
    ///     The updated segmented control.
    pub fn replace_item(mut self, index: usize, replacement: impl IntoElement) -> Self {
        self.replacements.retain(|(existing, _)| *existing != index);
        self.replacements
            .push((index, replacement.into_any_element()));
        self
    }

    pub fn render(self) -> impl IntoElement {
        self
    }

    pub fn render_with_palette(self, p: MoonPalette) -> impl IntoElement {
        self.render_with_theme(p, MoonThemeTokens::default())
    }

    /// Render the control with an explicit palette and theme tokens.
    ///
    /// Args:
    ///     p: Palette used to paint the control.
    ///     tokens: Tokens used to resolve geometry and typography.
    ///
    /// Returns:
    ///     The rendered segmented control.
    pub fn render_with_theme(self, p: MoonPalette, tokens: MoonThemeTokens) -> impl IntoElement {
        let accent = self.accent.colors(p);
        let on_click = self.on_click.clone();
        let on_scroll = self.on_scroll.clone();
        let mut replacements = self.replacements;

        let mut root = div()
            .id(self.id)
            .relative()
            .flex()
            .items_center()
            .h(px(tokens.fit_height(26.0, 14.0, 6.0)))
            .gap(px(tokens.ui(self.item_gap)))
            .whitespace_nowrap();

        if let Some(bounds) = self.bounds {
            root = root
                .absolute()
                .left(px(bounds.x))
                .top(px(bounds.y))
                .w(px(bounds.w))
                .h(px(bounds.h));
        }

        for (ix, item) in self.items.into_iter().enumerate() {
            let selected = item.selected;
            let disabled = item.disabled;
            let replacement = replacements
                .iter()
                .position(|(index, _)| *index == ix)
                .map(|position| replacements.swap_remove(position).1);
            let replaced = replacement.is_some();
            let key_color = if selected { accent.text } else { p.text_muted };
            let key_alpha = if selected { 0.60 } else { 0.667 };
            let label_color = if selected { accent.text } else { p.text_muted };
            let item_click = on_click.clone();
            let item_scroll = on_scroll.clone();
            let interaction = segment_cell_interaction(
                disabled,
                replaced,
                item_click.is_some(),
                item_scroll.is_some(),
                item.tooltip.is_some(),
            );
            let chrome = segment_cell_chrome(&tokens, replaced);

            let mut cell = div()
                .id(("segment-item", ix))
                .debug_selector(move || format!("moon-segment-cell:{ix}"))
                .relative()
                .flex()
                .items_center()
                .justify_center()
                .gap(px(chrome.gap))
                .w(px(item.width))
                .h_full()
                .px(px(chrome.pad_x))
                .when(interaction.click || interaction.scroll, |this| {
                    this.cursor_pointer()
                })
                .when(!interaction.click && !interaction.scroll, |this| {
                    this.cursor_default()
                })
                .when(!selected && !disabled && !replaced, |this| {
                    this.hover(move |this| this.bg(rgba_from(p.overlay, 0.025)))
                        .active(move |this| this.bg(rgba_from(p.overlay, 0.016)))
                });

            if let Some(replacement) = replacement {
                cell = cell.child(replacement);
            } else {
                cell = cell
                    .child(
                        MoonText::new(item.hotkey)
                            .color(key_color)
                            .alpha(if disabled { 0.40 } else { key_alpha })
                            .font_size(SEGMENT_HOTKEY_SIZE)
                            .line_height(12.0)
                            .weight(SEGMENT_HOTKEY_WEIGHT)
                            .mono(true)
                            .uppercase(false)
                            .render(),
                    )
                    .child(
                        MoonText::new(item.label)
                            .color(label_color)
                            .alpha(if disabled { 0.40 } else { 1.0 })
                            .font_size(SEGMENT_LABEL_SIZE)
                            .line_height(14.0)
                            .weight(if selected {
                                SEGMENT_LABEL_FIT_WEIGHT
                            } else {
                                400.0
                            })
                            .mono(true)
                            .uppercase(false)
                            .render(),
                    );
            }

            if selected {
                cell = cell.child(accent_underline_colored(
                    accent.underline,
                    &tokens,
                    8.0,
                    8.0,
                    0.0,
                ));
            }

            if interaction.tooltip {
                let tooltip = item
                    .tooltip
                    .expect("tooltip presence checked by interaction plan");
                cell = cell.tooltip(move |_window, cx| {
                    cx.new(|_| MoonTooltipView::new(tooltip.clone())).into()
                });
            }

            if interaction.click {
                let on_click =
                    item_click.expect("click handler presence checked by interaction plan");
                cell = cell.on_click(move |event, window, cx| {
                    on_click(ix, event, window, cx);
                });
            } else if interaction.stop_click {
                cell = cell.on_click(|_, _, cx| {
                    cx.stop_propagation();
                });
            }

            if interaction.scroll {
                let on_scroll =
                    item_scroll.expect("scroll handler presence checked by interaction plan");
                cell = cell.on_scroll_wheel(move |event, window, cx| {
                    on_scroll(ix, event, window, cx);
                });
            }

            root = root.child(cell);
        }

        root
    }
}

impl RenderOnce for MoonSegmentedControl {
    /// Render the control with the active palette and theme tokens.
    ///
    /// Args:
    ///     cx: Application context used to resolve the active theme.
    ///
    /// Returns:
    ///     The rendered segmented control.
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = MoonTheme::active_tokens(cx);
        self.render_with_theme(MoonPalette::active(cx), tokens)
    }
}

#[cfg(test)]
mod tests;
