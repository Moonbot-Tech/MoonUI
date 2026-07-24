use gpui::prelude::FluentBuilder;
use gpui::*;
use std::rc::Rc;

use super::{
    foundation::{MoonIndexedClickHandler, accent_underline_colored},
    text::{MoonText, fit_text_to_width, measure_text_width},
    theme::{MoonTheme, MoonThemeTokens},
    tokens::{MoonPalette, MoonRect, rgba_from},
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

/// WCAG 2.x relative luminance of an `0xRRGGBB` colour.
///
/// Deliberately not routed through this crate's other sRGB decode (`theme::color`'s oklab path):
/// that one uses the sRGB spec's 0.04045 knee, while WCAG 2.x specifies 0.03928. The two agree to
/// well within a rounding step for every colour in the palettes, but this function exists to answer
/// a WCAG question, so it follows the WCAG definition rather than borrowing a near-neighbour.
fn relative_luminance(color: u32) -> f32 {
    let channel = |shift: u32| {
        let c = ((color >> shift) & 0xFF) as f32 / 255.0;
        if c <= 0.03928 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(16) + 0.7152 * channel(8) + 0.0722 * channel(0)
}

/// WCAG 2.x contrast ratio between two opaque `0xRRGGBB` colours, from 1.0 to 21.0.
///
/// Symmetric in its arguments — the brighter colour is found, not assumed to be either one.
fn contrast_ratio(a: u32, b: u32) -> f32 {
    let (la, lb) = (relative_luminance(a), relative_luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
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
mod tests {
    // NOT `use super::*`: the glob would pull in the `gpui::test` macro, and `#[test]` would
    // expand into itself (recursion limit).
    use super::{
        LABEL_CONTRAST_FLOOR, MoonAccent, MoonPalette, MoonSegmentItem, MoonSegmentedControl,
        SEGMENT_GAP, SEGMENT_LABEL_SIZE, SEGMENT_PAD_X, SegmentCellChrome, SegmentCellInteraction,
        contrast_ratio, fit_segment_item, segment_cell_chrome, segment_cell_interaction,
    };
    use crate::moon::{MoonScale, MoonTheme, MoonThemeTokens};
    use gpui::{
        Context, InteractiveElement as _, IntoElement, Modifiers, ParentElement as _, Point,
        Render, ScrollDelta, ScrollWheelEvent, StatefulInteractiveElement as _, Styled as _,
        VisualTestContext, Window, div, px,
    };
    use std::{cell::RefCell, rc::Rc};

    const ACCENTS: [MoonAccent; 4] = [
        MoonAccent::Amber,
        MoonAccent::Blue,
        MoonAccent::Green,
        MoonAccent::Red,
    ];

    #[test]
    fn contrast_ratio_matches_the_published_anchors() {
        // Anchors the arithmetic every other assertion here leans on, against values published
        // with the WCAG 2.x definition rather than against this file's own output: the extremes
        // are exactly 21:1 and 1:1, and #767676 / #595959 on white are the canonical greys that
        // sit right on the AA and AAA thresholds.
        assert!((contrast_ratio(0x000000, 0xFFFFFF) - 21.0).abs() < 0.01);
        assert!((contrast_ratio(0x808080, 0x808080) - 1.0).abs() < 0.01);
        assert!((contrast_ratio(0x767676, 0xFFFFFF) - 4.54).abs() < 0.01);
        assert!((contrast_ratio(0x595959, 0xFFFFFF) - 7.00).abs() < 0.01);
        // Symmetric: which colour is brighter is found, not assumed.
        assert_eq!(
            contrast_ratio(0x000000, 0xFFFFFF),
            contrast_ratio(0xFFFFFF, 0x000000)
        );
    }

    #[test]
    fn every_accent_label_clears_the_contrast_floor_in_both_themes() {
        // The product decision this pins: an accent may tint the label only while the label stays
        // readable. The plausible edit that breaks it is letting the label follow the underline
        // hue unconditionally on the theory that a whole theme is
        // "safe". It is not: on the dark panel green and red measure 3.71:1 and 4.02:1, and on the
        // light one amber measures 3.48:1, so that edit reddens here on three separate accents.
        for p in [MoonPalette::TERMINAL, MoonPalette::LIGHT] {
            for accent in ACCENTS {
                let ratio = contrast_ratio(accent.colors(p).text, p.panel);
                assert!(
                    ratio >= LABEL_CONTRAST_FLOOR,
                    "{accent:?} label is {ratio:.2}:1 on the panel (is_light={}), under the \
                     {LABEL_CONTRAST_FLOOR}:1 floor",
                    p.is_light()
                );
            }
        }
    }

    #[test]
    fn each_accent_keeps_its_own_underline() {
        // The underline is what survives the legibility rule above — it always carries the hue, so
        // two differently-accented strips stay tellable apart even where their labels collapse onto
        // the same readable colour. Checks every pair, not one: a resolver that special-cased a
        // single accent would slip past an amber-vs-blue spot check.
        for p in [MoonPalette::TERMINAL, MoonPalette::LIGHT] {
            for (i, a) in ACCENTS.iter().enumerate() {
                for b in &ACCENTS[i + 1..] {
                    assert_ne!(
                        a.colors(p).underline,
                        b.colors(p).underline,
                        "{a:?} and {b:?} share an underline (is_light={})",
                        p.is_light()
                    );
                }
            }
        }
    }

    #[test]
    fn render_resolves_colours_from_the_configured_accent() {
        // Guards the CALL SITE, which no resolver test can reach: this assertion reddens when
        // render stops reading `self.accent` even if the resolver itself remains correct.
        let source = include_str!("segment.rs");
        let implementation = source.split("#[cfg(test)]").next().unwrap_or(source);

        assert!(
            implementation.contains("self.accent.colors(")
                && implementation.contains("accent_underline_colored("),
            "selected-segment colours must be resolved from self.accent, not from a palette-wide role"
        );
    }

    /// `segment.rs:fit_segment_item` must preserve the exact maximum boundary and truncate a
    /// one-character overflow without changing the final cell width. Removing the max-label fit
    /// lets one anomalous preset stretch the toolbar and invalidates its pre-render row budget.
    #[test]
    fn fitted_item_preserves_the_boundary_and_ellipsizes_one_past_it() {
        let measure = |text: &str| text.chars().count() as f32;
        let chrome = 5.0;
        let min = 8.0;
        let max = 15.0;

        let exact = fit_segment_item("", "1234567890", min, max, chrome, measure, measure);
        let overflow = fit_segment_item("", "1234567890X", min, max, chrome, measure, measure);

        assert_eq!(exact, ("1234567890".to_string(), max));
        assert_eq!(overflow, ("123456789\u{2026}".to_string(), max));

        for palette in [MoonPalette::TERMINAL, MoonPalette::LIGHT] {
            for (ui, font, font_delta) in [(0.5, 1.75, 0.0), (2.5, 0.75, 4.0), (2.5, 0.25, 0.0)] {
                let mut tokens = MoonThemeTokens {
                    palette,
                    ..MoonThemeTokens::default()
                };
                tokens.scale = MoonScale {
                    ui,
                    font,
                    font_delta,
                };
                let min = tokens.font_width(34.0);
                let max = tokens.font_width(104.0);
                let chrome = tokens.ui(SEGMENT_PAD_X) * 2.0 + tokens.ui(SEGMENT_GAP);
                let text_scale = tokens.font(SEGMENT_LABEL_SIZE) / SEGMENT_LABEL_SIZE;
                let measure = |text: &str| text.chars().count() as f32 * 7.0 * text_scale;
                let effective_min = min.max((chrome + measure("\u{2026}")).ceil());
                let effective_max = max.max(effective_min);
                let (label, width) = fit_segment_item(
                    "",
                    "a deliberately overlong preset value for scale coverage",
                    min,
                    max,
                    chrome,
                    measure,
                    measure,
                );

                assert!(width >= effective_min && width <= effective_max);
                assert!(
                    chrome + measure(&label) <= width,
                    "fitted label overflowed at ui={ui}, font={font}, delta={font_delta}"
                );
                assert!(label.ends_with('\u{2026}'));
            }
        }
    }

    /// `segment.rs:segment_cell_interaction` must make disabled and replacement cells inert.
    /// Allowing either state through dispatches a double-click or Ctrl+wheel to the preset while
    /// the user is editing it, or while no trading core is available.
    #[test]
    fn disabled_and_replaced_cells_expose_no_native_interactions() {
        let active = SegmentCellInteraction {
            click: true,
            stop_click: false,
            scroll: true,
            tooltip: true,
        };
        let inert = SegmentCellInteraction {
            click: false,
            stop_click: false,
            scroll: false,
            tooltip: false,
        };
        let stopped = SegmentCellInteraction {
            stop_click: true,
            ..inert
        };

        assert_eq!(
            segment_cell_interaction(false, false, true, true, true),
            active
        );
        assert_eq!(
            segment_cell_interaction(true, false, true, true, true),
            stopped
        );
        assert_eq!(
            segment_cell_interaction(false, true, true, true, true),
            inert
        );
    }

    /// `segment.rs:segment_cell_chrome` must remove normal label padding from an inline
    /// replacement. Reusing normal chrome for a replacement squeezes a 34px preset editor to only
    /// 12px at the default scale, making the value effectively uneditable.
    #[test]
    fn replaced_cell_releases_its_full_width_to_the_inline_editor() {
        let tokens = MoonThemeTokens::default();
        let normal = segment_cell_chrome(&tokens, false);
        let replaced = segment_cell_chrome(&tokens, true);

        assert!(normal.gap > 0.0 && normal.pad_x > 0.0);
        assert_eq!(
            replaced,
            SegmentCellChrome {
                gap: 0.0,
                pad_x: 0.0,
            }
        );
    }

    /// Root view with active, disabled, and inline-replaced segment cells inside a clickable
    /// ancestor.
    struct SegmentInteractionHarness {
        events: Rc<RefCell<Vec<(&'static str, usize)>>>,
    }

    impl Render for SegmentInteractionHarness {
        /// Render the three cell states with native click and scroll callbacks.
        ///
        /// Args:
        ///     _window: Test window.
        ///     _cx: Test view context.
        ///
        /// Returns:
        ///     The rendered ancestor and segmented control.
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let click_events = self.events.clone();
            let scroll_events = self.events.clone();
            let parent_events = self.events.clone();
            let segment = MoonSegmentedControl::new("segment-interaction-harness")
                .items([
                    MoonSegmentItem::new("", "active").width(80.0),
                    MoonSegmentItem::new("", "disabled")
                        .width(90.0)
                        .disabled(true),
                    MoonSegmentItem::new("", "replaced").width(100.0),
                ])
                .on_click(move |index, _, _, _| {
                    click_events.borrow_mut().push(("click", index));
                })
                .on_scroll(move |index, _, _, _| {
                    scroll_events.borrow_mut().push(("scroll", index));
                })
                .replace_item(
                    2,
                    div()
                        .debug_selector(|| "moon-segment-editor".to_string())
                        .w_full()
                        .h_full(),
                );

            div()
                .id("segment-interaction-parent")
                .on_click(move |_, _, _| {
                    parent_events.borrow_mut().push(("parent", usize::MAX));
                })
                .child(segment)
        }
    }

    /// `segment.rs:MoonSegmentedControl::render_with_theme` must render each resolved width,
    /// release replacement content from label padding, dispatch the active index, and stop a
    /// disabled cell before a clickable ancestor. Bypassing either interaction/chrome plan breaks
    /// at least one named assertion.
    #[gpui::test]
    fn rendered_cells_preserve_width_and_gate_native_interactions(cx: &mut gpui::TestAppContext) {
        cx.update(crate::init);
        let events = Rc::new(RefCell::new(Vec::new()));
        let window = cx.add_window({
            let events = events.clone();
            move |_, _| SegmentInteractionHarness { events }
        });
        let mut cx = VisualTestContext::from_window(window.into(), cx);
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();

        let cells: Vec<_> = [
            "moon-segment-cell:0",
            "moon-segment-cell:1",
            "moon-segment-cell:2",
        ]
        .into_iter()
        .map(|selector| {
            cx.debug_bounds(selector)
                .expect("every segment cell must register rendered bounds")
        })
        .collect();
        assert_eq!(cells[0].size.width, px(80.0));
        assert_eq!(cells[1].size.width, px(90.0));
        assert_eq!(cells[2].size.width, px(100.0));
        let editor = cx
            .debug_bounds("moon-segment-editor")
            .expect("inline replacement must render");
        assert_eq!(editor.size.width, cells[2].size.width);

        cx.simulate_click(cells[0].center(), Modifiers::default());
        assert!(
            events.borrow().contains(&("click", 0)),
            "active cell must dispatch its native index"
        );
        events.borrow_mut().clear();

        cx.simulate_event(ScrollWheelEvent {
            position: cells[0].center(),
            delta: ScrollDelta::Lines(Point { x: 0.0, y: 1.0 }),
            ..Default::default()
        });
        assert_eq!(events.borrow().as_slice(), [("scroll", 0)]);
        events.borrow_mut().clear();

        cx.simulate_click(cells[1].center(), Modifiers::default());
        assert!(
            events.borrow().is_empty(),
            "disabled cell must neither dispatch nor bubble to its clickable ancestor"
        );

        cx.simulate_click(cells[2].center(), Modifiers::default());
        assert!(
            !events.borrow().iter().any(|event| event.0 == "click"),
            "inline replacement must not dispatch the segment click handler"
        );
    }

    /// Root view containing one previously fitted segment item.
    struct FittedSegmentHarness {
        item: MoonSegmentItem,
    }

    impl Render for FittedSegmentHarness {
        /// Render the fitted item without recomputing its width.
        ///
        /// Args:
        ///     _window: Test window.
        ///     _cx: Test view context.
        ///
        /// Returns:
        ///     The rendered segmented control.
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            MoonSegmentedControl::new("fitted-segment-harness").item(self.item.clone())
        }
    }

    /// `segment.rs:MoonSegmentItem::fit_width` must expand a font-scaled ceiling when high UI scale
    /// makes the component's own chrome wider, and the rendered cell must consume exactly that
    /// reported width. Clamping back to the requested ceiling makes toolbar budgeting lie.
    #[gpui::test]
    fn fitted_segment_width_survives_high_ui_low_font_render(cx: &mut gpui::TestAppContext) {
        cx.update(crate::init);
        let scale = MoonScale {
            ui: 2.5,
            font: 0.25,
            font_delta: 0.0,
        };
        let item = cx.update(|cx| {
            MoonTheme::global_mut(cx).scale = scale;
            MoonSegmentItem::new("", "a long preset").fit_width(cx, 34.0, 104.0)
        });
        let resolved = item.resolved_width();
        let mut tokens = MoonThemeTokens::default();
        tokens.scale = scale;
        let chrome = tokens.ui(SEGMENT_PAD_X) * 2.0 + tokens.ui(SEGMENT_GAP);

        assert!(resolved >= chrome.ceil());
        assert!(
            resolved > tokens.font_width(104.0),
            "effective width must exceed an impossible font-scaled ceiling"
        );

        let window = cx.add_window(move |_, _| FittedSegmentHarness { item: item.clone() });
        let mut cx = VisualTestContext::from_window(window.into(), cx);
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
        let cell = cx
            .debug_bounds("moon-segment-cell:0")
            .expect("fitted segment cell must render");

        assert_eq!(cell.size.width, px(resolved));
    }
}
