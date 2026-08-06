use crate::popover::Popover as CorePopover;
use gpui::prelude::FluentBuilder;
use gpui::*;

use super::{
    button::{
        MoonButton, MoonButtonIconSlot, MoonButtonSegment, MoonButtonSize, MoonButtonVariant,
        button_leading_icon_reservation, button_text_metrics,
    },
    foundation::{MoonClickHandler, MoonSelectHandler, selected_background},
    icons::{MOON_ICON_CHECK, moon_icon},
    text::{MoonText, fit_text_to_width, fit_text_with_suffix, measure_text_width},
    theme::{MoonTheme, MoonThemeTokens},
    tokens::{MoonPalette, MoonRect, MoonTone, rgba_from},
};

#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering};

const MENU_PADDING: f32 = 4.0;
const MENU_BORDER: f32 = 1.0;
const MENU_GAP: f32 = 2.0;
pub(crate) const MENU_CHECK_WIDTH: f32 = 12.0;
const SUBMENU_OFFSET_X: f32 = 2.0;
const DROPDOWN_TRIGGER_PAD_X: f32 = 14.0;
// Keep this caret as a text suffix rather than a `MoonDisclosure` element: its text advance is part
// of the width measured by `fit_dropdown_trigger_label` and exposed through
// `MoonDropdown::fitted_trigger_label`. An element has no text advance and therefore cannot satisfy
// the fitted-label width contract.
const DROPDOWN_CARET: &str = " \u{25be}";
const DROPDOWN_TRIGGER_MONO: bool = true;
const VIRTUAL_MENU_ITEM_THRESHOLD: usize = 64;
const DEFAULT_VIRTUAL_MENU_MAX_HEIGHT: f32 = 320.0;
#[cfg(test)]
const MENU_MEASUREMENT_PROBE_PREFIX: &str = "moon-menu-measurement-probe-";
#[cfg(test)]
const MENU_CLONE_PROBE_PREFIX: &str = "moon-menu-clone-probe-";
#[cfg(test)]
const MENU_DROPDOWN_HANDLER_PROBE_PREFIX: &str = "moon-menu-handler-probe-";
#[cfg(test)]
const MENU_PALETTE_PROBE_PREFIX: &str = "moon-menu-palette-probe-";
#[cfg(test)]
static MENU_MEASUREMENT_PROBE_COUNT: AtomicUsize = AtomicUsize::new(0);
#[cfg(test)]
static MENU_ITEM_CLONE_PROBE_COUNT: AtomicUsize = AtomicUsize::new(0);
#[cfg(test)]
static MENU_DROPDOWN_HANDLER_PROBE_COUNT: AtomicUsize = AtomicUsize::new(0);
#[cfg(test)]
static MENU_PALETTE_PROBE_SHELL: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Copy, Debug, PartialEq)]
/// Width policy for one popup-menu level.
enum MoonMenuWidth {
    Rendered(f32),
    Scaled(f32),
    Fit { min: f32, max: f32 },
}

#[derive(Clone, Copy, Debug, PartialEq)]
/// Maximum-height policy for a popup menu.
enum MoonMenuMaxHeight {
    Rendered(f32),
    Ui(f32),
}

impl MoonMenuMaxHeight {
    /// Resolve the maximum rendered menu height.
    ///
    /// Args:
    ///     tokens: Active theme tokens used by UI-scaled policies.
    ///
    /// Returns:
    ///     Maximum menu height in rendered pixels.
    fn resolve(self, tokens: &MoonThemeTokens) -> f32 {
        match self {
            Self::Rendered(height) => height,
            Self::Ui(height) => tokens.ui(height),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
/// Width policy for a dropdown's plain-label trigger.
enum MoonDropdownTriggerWidth {
    Intrinsic,
    Rendered(f32),
    Scaled(f32),
    Fit { min: f32, max: f32 },
}

/// Resolve and, when necessary, truncate one plain dropdown trigger label.
///
/// Args:
///     label: Full caller-supplied label without the component-owned suffix.
///     suffix: Required suffix, normally the dropdown caret.
///     width: Trigger width policy.
///     tokens: Active theme tokens.
///     font_size: Design-reference trigger font size.
///     reserved_content_width: Rendered leading content and gap width excluded from text fitting.
///     measure: Width function matching the rendered trigger font.
///
/// Returns:
///     The fitted label including `suffix` and an optional explicit rendered width.
fn fit_dropdown_trigger_label(
    label: &str,
    suffix: &str,
    width: MoonDropdownTriggerWidth,
    tokens: &MoonThemeTokens,
    font_size: f32,
    reserved_content_width: f32,
    measure: impl Fn(&str) -> f32,
) -> (SharedString, Option<f32>) {
    let full = format!("{label}{suffix}");
    match width {
        MoonDropdownTriggerWidth::Intrinsic => return (SharedString::from(full), None),
        MoonDropdownTriggerWidth::Rendered(width) => {
            return (SharedString::from(full), Some(width));
        }
        MoonDropdownTriggerWidth::Scaled(_) | MoonDropdownTriggerWidth::Fit { .. } => {}
    }

    let text_scale = tokens.font(font_size) / font_size.max(1.0);
    let scaled = |value: f32| value * text_scale;
    let visual_padding = tokens.ui(DROPDOWN_TRIGGER_PAD_X) + reserved_content_width;
    let minimum_text = if label.is_empty() {
        suffix.to_string()
    } else {
        format!("\u{2026}{suffix}")
    };
    let minimum_width = visual_padding + measure(&minimum_text);
    match width {
        MoonDropdownTriggerWidth::Scaled(width) => {
            let width = scaled(width).max(minimum_width);
            let text_width = (width - visual_padding).max(0.0);
            let fitted = fit_text_with_suffix(label, suffix, text_width, measure).0;
            (SharedString::from(fitted), Some(width))
        }
        MoonDropdownTriggerWidth::Fit { min, max } => {
            let min = scaled(min).max(minimum_width);
            let max = scaled(max).max(min);
            let natural = (measure(&full) + visual_padding).ceil();
            let width = natural.clamp(min, max);
            let text_width = (width - visual_padding).max(0.0);
            let fitted = fit_text_with_suffix(label, suffix, text_width, measure).0;
            (SharedString::from(fitted), Some(width))
        }
        MoonDropdownTriggerWidth::Intrinsic | MoonDropdownTriggerWidth::Rendered(_) => {
            unreachable!("unmeasured trigger width handled before text measurement")
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoonMenuItemKind {
    Item,
    Label,
    Separator,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Ordered menu-row signature accumulated during builder insertion.
struct MenuLayoutFingerprint {
    item_count: usize,
    kind_hash: u64,
}

impl MenuLayoutFingerprint {
    /// Create the empty ordered menu-layout fingerprint.
    ///
    /// Returns:
    ///     A fingerprint ready to receive row kinds in display order.
    fn new() -> Self {
        Self {
            item_count: 0,
            kind_hash: 0xcbf2_9ce4_8422_2325,
        }
    }

    /// Add one row kind without requiring a second pass over the completed item collection.
    ///
    /// Args:
    ///     kind: Visual role of the appended menu row.
    ///
    /// Returns:
    ///     Nothing; the fingerprint is updated in place.
    fn push(&mut self, kind: MoonMenuItemKind) {
        let kind_tag = match kind {
            MoonMenuItemKind::Item => 1_u64,
            MoonMenuItemKind::Label => 2_u64,
            MoonMenuItemKind::Separator => 3_u64,
        };
        self.item_count += 1;
        self.kind_hash ^= kind_tag;
        self.kind_hash = self.kind_hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
}

impl Default for MenuLayoutFingerprint {
    /// Return the canonical empty menu-layout fingerprint.
    ///
    /// Returns:
    ///     The same value as [`Self::new`].
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
/// Shared immutable rows and their pre-accumulated variable-height layout signature.
pub(super) struct MoonMenuLevel {
    items: std::rc::Rc<Vec<MoonMenuItem>>,
    layout: MenuLayoutFingerprint,
}

impl MoonMenuLevel {
    /// Create an empty menu level.
    ///
    /// Returns:
    ///     Shared empty row storage with the canonical empty layout.
    fn empty() -> Self {
        Self {
            items: std::rc::Rc::new(Vec::new()),
            layout: MenuLayoutFingerprint::new(),
        }
    }

    /// Build shared menu storage and its layout signature in one pass.
    ///
    /// Args:
    ///     items: Rows in display order.
    ///
    /// Returns:
    ///     A reusable immutable menu level.
    pub(super) fn new(items: impl IntoIterator<Item = MoonMenuItem>) -> Self {
        let mut level = Self::empty();
        level.extend(items);
        level
    }

    /// Reuse already shared rows whose signature was accumulated by another builder.
    ///
    /// Args:
    ///     items: Shared rows in display order.
    ///     layout: Signature matching `items`.
    ///
    /// Returns:
    ///     A reusable immutable menu level.
    fn from_parts(items: std::rc::Rc<Vec<MoonMenuItem>>, layout: MenuLayoutFingerprint) -> Self {
        Self { items, layout }
    }

    /// Append rows while extending the layout signature in the same pass.
    ///
    /// Args:
    ///     items: Rows to append in display order.
    ///
    /// Returns:
    ///     Nothing; this level is updated in place.
    pub(super) fn extend(&mut self, items: impl IntoIterator<Item = MoonMenuItem>) {
        let target = std::rc::Rc::make_mut(&mut self.items);
        for item in items {
            self.layout.push(item.kind);
            target.push(item);
        }
    }

    /// Return the number of rows in this level.
    ///
    /// Returns:
    ///     Shared row count.
    pub(super) fn len(&self) -> usize {
        self.items.len()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MoonMenuSize {
    Compact,
    Normal,
    Custom {
        row_height: f32,
        font_size: f32,
        line_height: f32,
        radius: f32,
        pad_x: f32,
        gap: f32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
/// Resolved per-row geometry for one menu size and theme scale.
pub(crate) struct MenuMetrics {
    pub(crate) row_height: f32,
    pub(crate) font_size: f32,
    pub(crate) line_height: f32,
    pub(crate) radius: f32,
    pub(crate) pad_x: f32,
    pub(crate) gap: f32,
}

/// Return the row metrics of a menu of this size, resolved for the active scale.
///
/// Crate-visible so a list that must READ as a menu draws its rows on the menu's own geometry
/// rather than approximating it — the searchable combobox popup, which opens in the same rows as
/// dropdown menus and would otherwise differ in row height and label size.
pub(crate) fn menu_row_metrics(size: MoonMenuSize, tokens: &MoonThemeTokens) -> MenuMetrics {
    MoonPopupMenu::new("menu-metrics")
        .size(size)
        .metrics()
        .scaled(tokens)
}

/// Shared immutable inputs used to render every row in one menu level.
struct MenuLevelRenderContext {
    menu_id: SharedString,
    mono: bool,
    metrics: MenuMetrics,
    width_policy: MoonMenuWidth,
    size: MoonMenuSize,
    width: f32,
    truncate_labels: bool,
    palette: MoonPalette,
    tokens: MoonThemeTokens,
    dropdown_selection: Option<std::rc::Rc<MoonDropdownSelectionContext>>,
}

/// Retained variable-height list state for one large popup-menu level.
struct MoonPopupMenuVirtualState {
    list: ListState,
    layout: MenuLayoutFingerprint,
    metrics: MenuMetrics,
}

impl MoonPopupMenuVirtualState {
    /// Create retained list state matching one menu-level shape.
    ///
    /// Args:
    ///     layout: Ordered row-layout fingerprint accumulated while building the menu.
    ///     metrics: Scaled row metrics for ordinary items and labels.
    ///
    /// Returns:
    ///     A top-aligned variable-height list with two rows of overdraw.
    fn new(layout: MenuLayoutFingerprint, metrics: MenuMetrics) -> Self {
        Self {
            list: ListState::new(
                layout.item_count,
                ListAlignment::Top,
                px(metrics.row_height * 2.0),
            ),
            layout,
            metrics,
        }
    }

    /// Synchronize cached heights while retaining scroll position across ordinary repaints.
    ///
    /// Args:
    ///     layout: Current ordered row-layout fingerprint.
    ///     metrics: Current scaled row metrics.
    ///
    /// Returns:
    ///     A clone of the retained list handle for the current render.
    fn sync(&mut self, layout: MenuLayoutFingerprint, metrics: MenuMetrics) -> ListState {
        if self.layout != layout {
            self.list.reset(layout.item_count);
            self.layout = layout;
        } else if self.metrics != metrics {
            self.list.remeasure();
        }
        self.metrics = metrics;
        self.list.clone()
    }
}

impl MenuMetrics {
    /// Scale menu geometry while leaving font inputs in design-reference units for MoonText.
    ///
    /// Args:
    ///     tokens: Active theme tokens used to scale row geometry.
    ///
    /// Returns:
    ///     Menu metrics resolved for the active UI scale.
    fn scaled(self, tokens: &MoonThemeTokens) -> Self {
        let line_height = tokens.line_height(self.line_height);
        Self {
            row_height: tokens.ui(self.row_height).max(line_height + tokens.ui(4.0)),
            font_size: self.font_size,
            line_height: self.line_height,
            radius: tokens.ui(self.radius),
            pad_x: tokens.ui(self.pad_x),
            gap: tokens.ui(self.gap),
        }
    }
}

/// Return the fixed outer chrome around a menu row.
///
/// Args:
///     tokens: Active theme tokens used to scale menu padding.
///
/// Returns:
///     Combined menu padding and border width.
fn menu_outer_chrome(tokens: &MoonThemeTokens) -> f32 {
    tokens.ui(MENU_PADDING) * 2.0 + MENU_BORDER * 2.0
}

/// Return whether a menu level is large enough to require bounded element construction.
///
/// Args:
///     item_count: Number of rows at this menu level.
///
/// Returns:
///     `true` when this level should use GPUI's retained variable-height list.
fn menu_level_is_virtualized(item_count: usize) -> bool {
    item_count >= VIRTUAL_MENU_ITEM_THRESHOLD
}

/// Compute a mixed menu level's natural height only until its viewport is full.
///
/// Args:
///     kinds: Ordered item, label, and separator roles.
///     metrics: Scaled ordinary-row geometry.
///     tokens: Active theme tokens used for inter-row gaps.
///     content_max: Maximum useful viewport height.
///
/// Returns:
///     Natural content height capped at `content_max`.
fn capped_menu_items_height(
    kinds: impl Iterator<Item = MoonMenuItemKind>,
    metrics: MenuMetrics,
    tokens: &MoonThemeTokens,
    content_max: f32,
) -> f32 {
    let mut height = 0.0;
    for (ix, kind) in kinds.enumerate() {
        if ix > 0 {
            height += tokens.ui(MENU_GAP);
        }
        height += match kind {
            MoonMenuItemKind::Separator => 7.0,
            MoonMenuItemKind::Item | MoonMenuItemKind::Label => metrics.row_height,
        };
        if height >= content_max {
            return content_max;
        }
    }
    height
}

/// Resolve a bounded viewport height for a virtualized menu level.
///
/// Args:
///     items: Ordered mixed menu rows.
///     metrics: Scaled ordinary-row geometry.
///     max_height: Optional caller-supplied outer menu limit.
///     tokens: Active theme tokens.
///
/// Returns:
///     `(list viewport height, outer menu height limit)`.
fn virtual_menu_heights(
    items: &[MoonMenuItem],
    metrics: MenuMetrics,
    max_height: Option<MoonMenuMaxHeight>,
    tokens: &MoonThemeTokens,
) -> (f32, f32) {
    let outer_max = max_height
        .map(|height| height.resolve(tokens))
        .unwrap_or_else(|| tokens.ui(DEFAULT_VIRTUAL_MENU_MAX_HEIGHT));
    let content_max = (outer_max - menu_outer_chrome(tokens)).max(metrics.row_height);
    (
        capped_menu_items_height(
            items.iter().map(|item| item.kind),
            metrics,
            tokens,
            content_max,
        ),
        outer_max,
    )
}

/// Return the UI-scaled check-column width rendered by every actionable menu row.
///
/// Args:
///     tokens: Active theme tokens.
///
/// Returns:
///     Rendered check-column width.
fn menu_check_width(tokens: &MoonThemeTokens) -> f32 {
    tokens.ui(MENU_CHECK_WIDTH)
}

#[derive(Clone, Copy, Debug, PartialEq)]
/// Natural and minimum viable outer widths for one menu level.
struct MenuWidthRequirements {
    natural: f32,
    minimum: f32,
}

/// Measure natural and minimum viable widths in one pass over a menu level.
///
/// Args:
///     items: Rows belonging to this menu level.
///     metrics: Resolved row geometry.
///     tokens: Active theme tokens.
///     measure: Width function matching each row's text style.
///
/// Returns:
///     Rounded natural and minimum viable outer widths.
fn menu_width_requirements(
    items: &[MoonMenuItem],
    metrics: MenuMetrics,
    tokens: &MoonThemeTokens,
    mut measure: impl FnMut(&str, f32, f32) -> f32,
) -> MenuWidthRequirements {
    let (widest_natural, widest_minimum) = items
        .iter()
        .map(|item| match item.kind {
            MoonMenuItemKind::Separator => (0.0, 0.0),
            MoonMenuItemKind::Label => {
                let chrome = metrics.pad_x * 2.0;
                let natural = chrome + measure(item.label.as_ref(), metrics.font_size, 500.0);
                let marker = if item.label.is_empty() {
                    0.0
                } else {
                    measure("\u{2026}", metrics.font_size, 500.0)
                };
                (natural, chrome + marker)
            }
            MoonMenuItemKind::Item => {
                let (trailing_natural, trailing_minimum) =
                    if let Some(right_label) = item.right_label.as_ref() {
                        (
                            measure(right_label.as_ref(), metrics.font_size - 0.5, 400.0),
                            if right_label.is_empty() {
                                0.0
                            } else {
                                measure("\u{2026}", metrics.font_size - 0.5, 400.0)
                            },
                        )
                    } else if item.has_submenu() {
                        let glyph = measure("\u{203a}", metrics.font_size, 600.0);
                        (glyph, glyph)
                    } else {
                        (0.0, 0.0)
                    };
                let has_trailing = item.right_label.is_some() || item.has_submenu();
                let gaps = if has_trailing { 3.0 } else { 2.0 };
                let chrome = metrics.pad_x * 2.0 + menu_check_width(tokens) + metrics.gap * gaps;
                let label_natural = measure(item.label.as_ref(), metrics.font_size, 600.0);
                let label_minimum = if item.label.is_empty() {
                    0.0
                } else {
                    measure("\u{2026}", metrics.font_size, 600.0)
                };
                (
                    chrome + label_natural + trailing_natural,
                    chrome + label_minimum + trailing_minimum,
                )
            }
        })
        .fold((0.0_f32, 0.0_f32), |(natural, minimum), row| {
            (natural.max(row.0), minimum.max(row.1))
        });
    let outer = menu_outer_chrome(tokens);
    MenuWidthRequirements {
        natural: (outer + widest_natural).ceil(),
        minimum: (outer + widest_minimum).ceil(),
    }
}

/// Return the natural outer width required by the widest item in one menu level.
///
/// Args:
///     items: Rows belonging to this menu level.
///     metrics: Resolved row geometry.
///     tokens: Active theme tokens.
///     measure: Width function matching each row's text style.
///
/// Returns:
///     Rounded outer width required by the widest row.
#[cfg(test)]
fn natural_menu_width(
    items: &[MoonMenuItem],
    metrics: MenuMetrics,
    tokens: &MoonThemeTokens,
    measure: impl FnMut(&str, f32, f32) -> f32,
) -> f32 {
    menu_width_requirements(items, metrics, tokens, measure).natural
}

/// Truncate labels to the exact text budgets of one resolved menu level.
///
/// Args:
///     items: Mutable rows belonging to this menu level.
///     width: Resolved outer menu width.
///     metrics: Resolved row geometry.
///     tokens: Active theme tokens.
///     cx: Application context used for text measurement.
///     mono: Whether rows use the configured monospaced font.
///
/// Returns:
///     Nothing; row labels are updated in place.
#[cfg(test)]
fn fit_menu_item_labels(
    items: &mut [MoonMenuItem],
    width: f32,
    metrics: MenuMetrics,
    tokens: &MoonThemeTokens,
    cx: &App,
    mono: bool,
) {
    let outer = menu_outer_chrome(tokens);
    for item in items {
        fit_menu_item_label(item, width, metrics, tokens, cx, mono, outer);
    }
}

/// Truncate one menu row to the exact text budget of a resolved menu level.
///
/// Args:
///     item: Mutable row to fit.
///     width: Resolved outer menu width.
///     metrics: Resolved row geometry.
///     tokens: Active theme tokens.
///     cx: Application context used for text measurement.
///     mono: Whether the row uses the configured monospaced font.
///     outer: Precomputed menu border and padding width.
///
/// Returns:
///     Nothing; the row labels are updated in place.
fn fit_menu_item_label(
    item: &mut MoonMenuItem,
    width: f32,
    metrics: MenuMetrics,
    tokens: &MoonThemeTokens,
    cx: &App,
    mono: bool,
    outer: f32,
) {
    match item.kind {
        MoonMenuItemKind::Separator => {}
        MoonMenuItemKind::Label => {
            let budget = (width - outer - metrics.pad_x * 2.0).max(0.0);
            let fitted = fit_text_to_width(item.label.as_ref(), budget, |text| {
                measure_menu_text_width(cx, tokens, text, metrics.font_size, 500.0, mono)
            })
            .0;
            item.label = SharedString::from(fitted);
        }
        MoonMenuItemKind::Item => {
            let has_submenu = item.has_submenu();
            let trailing_gaps = if item.right_label.is_some() || has_submenu {
                3.0
            } else {
                2.0
            };
            let mut text_budget = (width
                - outer
                - metrics.pad_x * 2.0
                - menu_check_width(tokens)
                - metrics.gap * trailing_gaps)
                .max(0.0);

            if let Some(right_label) = item.right_label.take() {
                let main_ellipsis =
                    measure_menu_text_width(cx, tokens, "\u{2026}", metrics.font_size, 600.0, mono);
                let right_budget = (text_budget - main_ellipsis).max(0.0);
                let (right_label, right_width) =
                    fit_text_to_width(right_label.as_ref(), right_budget, |text| {
                        measure_menu_text_width(
                            cx,
                            tokens,
                            text,
                            metrics.font_size - 0.5,
                            400.0,
                            mono,
                        )
                    });
                item.right_label = Some(SharedString::from(right_label));
                text_budget = (text_budget - right_width).max(0.0);
            } else if has_submenu {
                text_budget = (text_budget
                    - measure_menu_text_width(
                        cx,
                        tokens,
                        "\u{203a}",
                        metrics.font_size,
                        600.0,
                        mono,
                    ))
                .max(0.0);
            }

            let fitted = fit_text_to_width(item.label.as_ref(), text_budget, |text| {
                measure_menu_text_width(cx, tokens, text, metrics.font_size, 600.0, mono)
            })
            .0;
            item.label = SharedString::from(fitted);
        }
    }
}

/// Measure menu text and expose a sentinel-only counter to regression tests.
///
/// Args:
///     cx: Application context owning the text system.
///     tokens: Active theme tokens.
///     text: Text being measured.
///     font_size: Design-reference font size.
///     weight: Font weight.
///     mono: Whether to use the configured monospaced family.
///
/// Returns:
///     Rendered text width.
fn measure_menu_text_width(
    cx: &App,
    tokens: &MoonThemeTokens,
    text: &str,
    font_size: f32,
    weight: f32,
    mono: bool,
) -> f32 {
    #[cfg(test)]
    if text.starts_with(MENU_MEASUREMENT_PROBE_PREFIX) {
        MENU_MEASUREMENT_PROBE_COUNT.fetch_add(1, Ordering::Relaxed);
    }
    measure_text_width(cx, tokens, text, font_size, weight, mono)
}

/// Return the current sentinel-only menu measurement count for visual regressions.
///
/// Returns:
///     Number of measurement calls whose text started with the probe prefix.
#[cfg(test)]
fn menu_measurement_probe_count() -> usize {
    MENU_MEASUREMENT_PROBE_COUNT.load(Ordering::Relaxed)
}

/// Reset and return the menu-row clone probe used by virtual repaint regressions.
///
/// Returns:
///     Number of row clones recorded since the previous reset.
#[cfg(test)]
pub(super) fn take_menu_item_clone_probe_count() -> usize {
    MENU_ITEM_CLONE_PROBE_COUNT.swap(0, Ordering::Relaxed)
}

/// Reset and return the visible-row dropdown-handler probe.
///
/// Returns:
///     Number of probe handlers resolved since the previous reset.
#[cfg(test)]
fn take_dropdown_handler_probe_count() -> usize {
    MENU_DROPDOWN_HANDLER_PROBE_COUNT.swap(0, Ordering::Relaxed)
}

/// Reset and return the last palette shell observed by a probe submenu row.
///
/// Returns:
///     Palette shell color, or zero when no probe row rendered.
#[cfg(test)]
fn take_palette_probe_shell() -> u32 {
    MENU_PALETTE_PROBE_SHELL.swap(0, Ordering::Relaxed) as u32
}

/// Resolve a menu width policy and whether its labels should be bounded to the result.
///
/// Args:
///     policy: Configured rendered, scaled, or fitted width policy.
///     items: Rows belonging to this menu level.
///     metrics: Resolved row geometry.
///     tokens: Active theme tokens.
///     cx: Application context used for text measurement.
///     mono: Whether rows use the configured monospaced font.
///
/// Returns:
///     Resolved outer width and whether labels must be truncated to it.
fn resolve_menu_width(
    policy: MoonMenuWidth,
    items: &[MoonMenuItem],
    metrics: MenuMetrics,
    tokens: &MoonThemeTokens,
    cx: &App,
    mono: bool,
) -> (f32, bool) {
    if let MoonMenuWidth::Rendered(width) = policy {
        return (width, false);
    }

    let text_scale = tokens.font(metrics.font_size) / metrics.font_size.max(1.0);
    let requirements = menu_width_requirements(items, metrics, tokens, |text, size, weight| {
        measure_menu_text_width(cx, tokens, text, size, weight, mono)
    });
    let width = match policy {
        MoonMenuWidth::Scaled(width) => (width * text_scale).max(requirements.minimum),
        MoonMenuWidth::Fit { min, max } => {
            let min = (min * text_scale).max(requirements.minimum);
            let max = (max * text_scale).max(min);
            requirements.natural.clamp(min, max)
        }
        MoonMenuWidth::Rendered(_) => {
            unreachable!("rendered menu width handled before text measurement")
        }
    };
    (width, width < requirements.natural)
}

/// Return the prefix of rows that can contribute to a virtual menu's initial viewport.
///
/// Args:
///     items: Ordered menu rows.
///     metrics: Resolved row geometry.
///     max_height: Optional caller-supplied outer height limit.
///     tokens: Active theme tokens.
///
/// Returns:
///     A bounded prefix ending at the row that fills the initial viewport.
fn virtual_menu_initial_rows<'a>(
    items: &'a [MoonMenuItem],
    metrics: MenuMetrics,
    max_height: Option<MoonMenuMaxHeight>,
    tokens: &MoonThemeTokens,
) -> &'a [MoonMenuItem] {
    let outer_max = max_height
        .map(|height| height.resolve(tokens))
        .unwrap_or_else(|| tokens.ui(DEFAULT_VIRTUAL_MENU_MAX_HEIGHT));
    let content_max = (outer_max - menu_outer_chrome(tokens)).max(metrics.row_height);
    let mut height = 0.0;
    for (ix, item) in items.iter().enumerate() {
        if ix > 0 {
            height += tokens.ui(MENU_GAP);
        }
        height += match item.kind {
            MoonMenuItemKind::Separator => 7.0,
            MoonMenuItemKind::Item | MoonMenuItemKind::Label => metrics.row_height,
        };
        if height >= content_max {
            return &items[..=ix];
        }
    }
    items
}

/// Resolve a large virtual menu from only the rows in its initial bounded viewport.
///
/// Later rows truncate against the resulting budget. This keeps fitting independent of total item
/// count while avoiding the threshold jump caused by always choosing the declared maximum width.
///
/// Args:
///     policy: Configured rendered, scaled, or fitted width policy.
///     items: Ordered rows belonging to this menu level.
///     metrics: Resolved row geometry.
///     max_height: Optional caller-supplied outer height limit.
///     tokens: Active theme tokens.
///     cx: Application context used for bounded text measurement.
///     mono: Whether rows use the configured monospaced font.
///
/// Returns:
///     Resolved outer width and whether visible labels must be truncated.
fn resolve_virtual_menu_width(
    policy: MoonMenuWidth,
    items: &[MoonMenuItem],
    metrics: MenuMetrics,
    max_height: Option<MoonMenuMaxHeight>,
    tokens: &MoonThemeTokens,
    cx: &App,
    mono: bool,
) -> (f32, bool) {
    if let MoonMenuWidth::Rendered(width) = policy {
        return (width, false);
    }

    let initial_rows = virtual_menu_initial_rows(items, metrics, max_height, tokens);
    let text_scale = tokens.font(metrics.font_size) / metrics.font_size.max(1.0);
    let requirements =
        menu_width_requirements(initial_rows, metrics, tokens, |text, size, weight| {
            measure_menu_text_width(cx, tokens, text, size, weight, mono)
        });
    let width = match policy {
        MoonMenuWidth::Scaled(width) => (width * text_scale).max(requirements.minimum),
        MoonMenuWidth::Fit { min, max } => {
            let min = (min * text_scale).max(requirements.minimum);
            let max = (max * text_scale).max(min);
            requirements.natural.clamp(min, max)
        }
        MoonMenuWidth::Rendered(_) => {
            unreachable!("rendered menu width handled before fixed glyph measurement")
        }
    };
    (
        width,
        initial_rows.len() < items.len() || width < requirements.natural,
    )
}

/// Return whether a menu row kind is allowed to dispatch click handlers.
///
/// Args:
///     kind: Visual role of the menu row.
///     disabled: Whether interaction is disabled for the row.
///     label_actionable: Whether a label explicitly opted into action behavior.
///
/// Returns:
///     `true` for enabled items and explicitly actionable enabled labels.
fn moon_menu_item_accepts_click(
    kind: MoonMenuItemKind,
    disabled: bool,
    label_actionable: bool,
) -> bool {
    !disabled
        && match kind {
            MoonMenuItemKind::Item => true,
            MoonMenuItemKind::Label => label_actionable,
            MoonMenuItemKind::Separator => false,
        }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MoonDropdownSelectPlan {
    close_menu: bool,
    update_internal_open: bool,
}

/// Shared dropdown behavior applied lazily only when a menu row is rendered.
struct MoonDropdownSelectionContext {
    close_on_select: bool,
    on_select: Option<MoonSelectHandler>,
    state: Entity<MoonDropdownState>,
    controlled_open: Option<bool>,
    on_open_change: Option<std::rc::Rc<dyn Fn(bool, &mut Window, &mut App)>>,
    parent_view: EntityId,
}

fn moon_dropdown_select_plan(
    close_on_select: bool,
    controlled_open: Option<bool>,
) -> MoonDropdownSelectPlan {
    MoonDropdownSelectPlan {
        close_menu: close_on_select,
        update_internal_open: close_on_select && controlled_open.is_none(),
    }
}

/// Resolve one rendered row's native and dropdown-level click behavior.
///
/// Args:
///     item: Visible row whose key and native handler may be dispatched.
///     dropdown: Optional dropdown behavior for a root popup level.
///
/// Returns:
///     A shared click handler, or `None` when neither behavior is present.
fn menu_item_click_handler(
    item: &MoonMenuItem,
    dropdown: Option<&std::rc::Rc<MoonDropdownSelectionContext>>,
) -> Option<MoonClickHandler> {
    #[cfg(test)]
    if dropdown.is_some() && item.label.starts_with(MENU_DROPDOWN_HANDLER_PROBE_PREFIX) {
        MENU_DROPDOWN_HANDLER_PROBE_COUNT.fetch_add(1, Ordering::Relaxed);
    }
    let existing_handler = item.on_click.clone();
    let Some(dropdown) = dropdown.cloned() else {
        return existing_handler;
    };
    let key = item.key.clone();
    Some(std::rc::Rc::new(move |event, window, cx| {
        let plan = moon_dropdown_select_plan(dropdown.close_on_select, dropdown.controlled_open);
        if let Some(existing_handler) = existing_handler.as_ref() {
            existing_handler(event, window, cx);
        }
        if let Some(on_select) = dropdown.on_select.as_ref() {
            on_select(&key, window, cx);
        }
        if plan.close_menu {
            if let Some(on_open_change) = dropdown.on_open_change.as_ref() {
                on_open_change(false, window, cx);
            }
            if plan.update_internal_open {
                dropdown.state.update(cx, |state, _| {
                    state.open = false;
                });
                cx.notify(dropdown.parent_view);
            }
        }
    }))
}

/// Immutable-render menu row with shared handlers and nested menu storage.
pub struct MoonMenuItem {
    key: SharedString,
    label: SharedString,
    kind: MoonMenuItemKind,
    right_label: Option<SharedString>,
    tone: MoonTone,
    selected: bool,
    checked: bool,
    disabled: bool,
    actionable: bool,
    submenu: MoonMenuLevel,
    on_click: Option<MoonClickHandler>,
}

impl Clone for MoonMenuItem {
    /// Clone one row model while letting regressions count repaint-time clone volume.
    ///
    /// Returns:
    ///     A row that shares immutable handlers and submenu storage with the source.
    fn clone(&self) -> Self {
        #[cfg(test)]
        if self.label.starts_with(MENU_CLONE_PROBE_PREFIX) {
            MENU_ITEM_CLONE_PROBE_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        Self {
            key: self.key.clone(),
            label: self.label.clone(),
            kind: self.kind,
            right_label: self.right_label.clone(),
            tone: self.tone,
            selected: self.selected,
            checked: self.checked,
            disabled: self.disabled,
            actionable: self.actionable,
            submenu: self.submenu.clone(),
            on_click: self.on_click.clone(),
        }
    }
}

impl MoonMenuItem {
    /// Create an enabled ordinary menu row whose key matches its label.
    ///
    /// Args:
    ///     label: Text and default selection key for the row.
    ///
    /// Returns:
    ///     A default actionable menu row.
    pub fn new(label: impl Into<SharedString>) -> Self {
        let label = label.into();
        Self {
            key: label.clone(),
            label,
            kind: MoonMenuItemKind::Item,
            right_label: None,
            tone: MoonTone::Default,
            selected: false,
            checked: false,
            disabled: false,
            actionable: true,
            submenu: MoonMenuLevel::empty(),
            on_click: None,
        }
    }

    /// Create an enabled ordinary menu row with a distinct selection key.
    ///
    /// Args:
    ///     key: Stable value reported by selection callbacks.
    ///     label: Text rendered for the row.
    ///
    /// Returns:
    ///     A default actionable menu row.
    pub fn with_key(key: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            kind: MoonMenuItemKind::Item,
            right_label: None,
            tone: MoonTone::Default,
            selected: false,
            checked: false,
            disabled: false,
            actionable: true,
            submenu: MoonMenuLevel::empty(),
            on_click: None,
        }
    }

    pub fn label(label: impl Into<SharedString>) -> Self {
        let mut item = Self::new(label);
        item.kind = MoonMenuItemKind::Label;
        item.disabled = true;
        item.actionable = false;
        item
    }

    /// Create an enabled section label that preserves label typography while accepting clicks.
    ///
    /// Args:
    ///     key: Stable selection key reported by dropdown callbacks.
    ///     label: Text rendered with section-label geometry and typography.
    ///
    /// Returns:
    ///     An enabled label row ready for an [`Self::on_click`] handler.
    pub fn action_label(key: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        let mut item = Self::with_key(key, label);
        item.kind = MoonMenuItemKind::Label;
        item
    }

    /// Create a compact inert separator row.
    ///
    /// Returns:
    ///     A disabled separator row with no interaction handler.
    pub fn separator() -> Self {
        Self {
            key: SharedString::from("separator"),
            label: SharedString::from(""),
            kind: MoonMenuItemKind::Separator,
            right_label: None,
            tone: MoonTone::Muted,
            selected: false,
            checked: false,
            disabled: true,
            actionable: false,
            submenu: MoonMenuLevel::empty(),
            on_click: None,
        }
    }

    pub fn key(&self) -> &SharedString {
        &self.key
    }

    pub fn right_label(mut self, right_label: impl Into<SharedString>) -> Self {
        self.right_label = Some(right_label.into());
        self
    }

    pub fn tone(mut self, tone: MoonTone) -> Self {
        self.tone = tone;
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Attach an immutable nested menu and accumulate its layout signature in one pass.
    ///
    /// Args:
    ///     items: Nested rows in display order.
    ///
    /// Returns:
    ///     The updated parent row.
    pub fn submenu(mut self, items: impl IntoIterator<Item = MoonMenuItem>) -> Self {
        self.submenu = MoonMenuLevel::new(items);
        self
    }

    /// Return whether this row owns at least one nested menu row.
    ///
    /// Returns:
    ///     `true` when the shared nested level is non-empty.
    pub fn has_submenu(&self) -> bool {
        !self.submenu.items.is_empty()
    }

    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(std::rc::Rc::new(handler));
        self
    }
}

#[derive(IntoElement)]
/// Moon-styled popup menu with rendered, scaled, or per-level fitted width policies.
pub struct MoonPopupMenu {
    id: SharedString,
    headers: Vec<AnyElement>,
    items: std::rc::Rc<Vec<MoonMenuItem>>,
    layout: MenuLayoutFingerprint,
    size: MoonMenuSize,
    width: MoonMenuWidth,
    max_height: Option<MoonMenuMaxHeight>,
    mono: bool,
    dropdown_selection: Option<std::rc::Rc<MoonDropdownSelectionContext>>,
}

#[derive(IntoElement)]
/// Deferred menu renderer that preserves resolved parent theme values and retained list state.
struct MoonPopupMenuResolvedTheme {
    menu: MoonPopupMenu,
    palette: MoonPalette,
    tokens: MoonThemeTokens,
}

impl MoonPopupMenu {
    /// Create a popup menu with normal rows and the legacy rendered default width.
    ///
    /// Args:
    ///     id: Stable element identity used by rows and nested submenus.
    ///
    /// Returns:
    ///     A default popup-menu builder.
    pub fn new(id: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            headers: Vec::new(),
            items: std::rc::Rc::new(Vec::new()),
            layout: MenuLayoutFingerprint::new(),
            size: MoonMenuSize::Normal,
            width: MoonMenuWidth::Rendered(160.0),
            max_height: None,
            mono: true,
            dropdown_selection: None,
        }
    }

    /// Append one menu row and update the retained-layout signature in the same pass.
    ///
    /// Args:
    ///     item: Menu row to append.
    ///
    /// Returns:
    ///     The updated menu.
    pub fn item(mut self, item: MoonMenuItem) -> Self {
        self.layout.push(item.kind);
        std::rc::Rc::make_mut(&mut self.items).push(item);
        self
    }

    pub fn header(mut self, header: impl IntoElement) -> Self {
        self.headers.push(header.into_any_element());
        self
    }

    /// Append menu rows while accumulating their retained-layout signature.
    ///
    /// Args:
    ///     new_items: Menu rows in display order.
    ///
    /// Returns:
    ///     The updated menu.
    pub fn items(mut self, new_items: impl IntoIterator<Item = MoonMenuItem>) -> Self {
        let items = std::rc::Rc::make_mut(&mut self.items);
        for item in new_items {
            self.layout.push(item.kind);
            items.push(item);
        }
        self
    }

    /// Install an already shared menu level without cloning or rescanning its rows.
    ///
    /// Args:
    ///     level: Shared rows and their ordered layout signature.
    ///
    /// Returns:
    ///     The updated menu.
    pub(super) fn shared_level(mut self, level: MoonMenuLevel) -> Self {
        self.items = level.items;
        self.layout = level.layout;
        self
    }

    pub fn size(mut self, size: MoonMenuSize) -> Self {
        self.size = size;
        self
    }

    /// Set a legacy rendered outer width.
    ///
    /// Args:
    ///     width: Outer width in rendered pixels.
    ///
    /// Returns:
    ///     The updated menu.
    pub fn width(mut self, width: f32) -> Self {
        self.width = MoonMenuWidth::Rendered(width);
        self
    }

    /// Set a fixed design-reference width that scales with this menu's text metrics.
    ///
    /// Labels are truncated inside the resolved row budget, including right labels and submenu
    /// glyphs.
    ///
    /// Args:
    ///     width: Outer width at the configured font reference size.
    ///
    /// Returns:
    ///     The updated menu.
    pub fn width_scaled(mut self, width: f32) -> Self {
        self.width = MoonMenuWidth::Scaled(width);
        self
    }

    /// Fit this menu level to its items between font-scaled design-reference bounds.
    ///
    /// Each submenu resolves the same policy independently from its own items.
    ///
    /// Args:
    ///     min_width: Minimum outer width at the configured font reference size.
    ///     max_width: Maximum outer width at the configured font reference size.
    ///
    /// Returns:
    ///     The updated menu.
    pub fn fit_width(mut self, min_width: f32, max_width: f32) -> Self {
        self.width = MoonMenuWidth::Fit {
            min: min_width,
            max: max_width,
        };
        self
    }

    /// Apply an already-selected width policy to a nested menu.
    ///
    /// Args:
    ///     width: Policy inherited by the submenu and resolved against its own rows.
    ///
    /// Returns:
    ///     The updated nested menu.
    fn width_policy(mut self, width: MoonMenuWidth) -> Self {
        self.width = width;
        self
    }

    /// Set a legacy maximum height in rendered pixels.
    ///
    /// Args:
    ///     max_height: Maximum rendered menu height.
    ///
    /// Returns:
    ///     The updated menu.
    pub fn max_height(mut self, max_height: f32) -> Self {
        self.max_height = Some(MoonMenuMaxHeight::Rendered(max_height));
        self
    }

    /// Set a UI-scaled design-reference maximum menu height.
    ///
    /// Args:
    ///     max_height: Maximum height at the configured UI reference scale.
    ///
    /// Returns:
    ///     The updated menu.
    pub fn max_height_ui(mut self, max_height: f32) -> Self {
        self.max_height = Some(MoonMenuMaxHeight::Ui(max_height));
        self
    }

    /// Apply an already-selected maximum-height policy to a nested menu host.
    ///
    /// Args:
    ///     max_height: Maximum-height policy inherited from a dropdown.
    ///
    /// Returns:
    ///     The updated menu.
    fn max_height_policy(mut self, max_height: MoonMenuMaxHeight) -> Self {
        self.max_height = Some(max_height);
        self
    }

    pub fn mono(mut self, mono: bool) -> Self {
        self.mono = mono;
        self
    }

    /// Attach root-dropdown behavior that visible rows resolve lazily.
    ///
    /// Args:
    ///     selection: Shared dropdown selection and dismissal behavior.
    ///
    /// Returns:
    ///     The updated root popup.
    fn dropdown_selection(mut self, selection: std::rc::Rc<MoonDropdownSelectionContext>) -> Self {
        self.dropdown_selection = Some(selection);
        self
    }

    /// Resolve retained virtual-list state for this menu level when it crosses the threshold.
    ///
    /// Args:
    ///     tokens: Theme tokens used to calculate row metrics.
    ///     window: Window that owns keyed retained state.
    ///     cx: Application context used to create or update that state.
    ///
    /// Returns:
    ///     The retained list handle for a large menu, or `None` for an eager small menu.
    fn retained_virtual_list_state(
        &self,
        tokens: &MoonThemeTokens,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<ListState> {
        menu_level_is_virtualized(self.items.len()).then(|| {
            let metrics = self.metrics().scaled(tokens);
            window
                .use_keyed_state(
                    ElementId::from(SharedString::from(format!("{}:virtual-list", self.id))),
                    cx,
                    |_, _| MoonPopupMenuVirtualState::new(self.layout, metrics),
                )
                .update(cx, |state, _| state.sync(self.layout, metrics))
        })
    }

    pub fn render(self) -> impl IntoElement {
        self
    }

    /// Render a legacy fixed-width menu with an explicit palette and default theme tokens.
    ///
    /// Measured policies require the active `App` text system and must render through
    /// [`Self::render`] instead.
    ///
    /// Args:
    ///     p: Palette used to paint the menu.
    ///
    /// Returns:
    ///     The rendered fixed-width menu.
    ///
    /// Panics:
    ///     Panics when called after [`Self::width_scaled`] or [`Self::fit_width`].
    pub fn render_with_palette(self, p: MoonPalette) -> AnyElement {
        assert!(
            matches!(self.width, MoonMenuWidth::Rendered(_)),
            "scaled or fitted menu widths require RenderOnce with an App context"
        );
        if !menu_level_is_virtualized(self.items.len()) {
            return self.render_with_theme(p, MoonThemeTokens::default(), None, None);
        }
        MoonPopupMenuResolvedTheme {
            menu: self,
            palette: p,
            tokens: MoonThemeTokens::default(),
        }
        .into_any_element()
    }

    /// Render with explicit palette/tokens and an optional text-measurement context.
    ///
    /// Args:
    ///     p: Palette used to paint the menu.
    ///     tokens: Tokens used to resolve menu geometry.
    ///     cx: Optional application context; fitted policies require it for text measurement.
    ///     virtual_list_state: Retained variable-height state for a large menu level.
    ///
    /// Returns:
    ///     The rendered menu.
    fn render_with_theme(
        self,
        p: MoonPalette,
        tokens: MoonThemeTokens,
        cx: Option<&App>,
        virtual_list_state: Option<ListState>,
    ) -> AnyElement {
        let metrics = self.metrics().scaled(&tokens);
        self.render_with_metrics(p, metrics, tokens, cx, virtual_list_state)
    }

    /// Renders the menu with precomputed layout metrics and the supplied theme tokens.
    ///
    /// Args:
    ///     p: Palette used to paint the menu.
    ///     metrics: Resolved menu row metrics.
    ///     tokens: Active theme tokens.
    ///     cx: Optional application context used by fitted policies.
    ///     virtual_list_state: Retained variable-height state, or `None` for eager rendering.
    ///
    /// Returns:
    ///     The rendered menu.
    fn render_with_metrics(
        self,
        p: MoonPalette,
        metrics: MenuMetrics,
        tokens: MoonThemeTokens,
        cx: Option<&App>,
        virtual_list_state: Option<ListState>,
    ) -> AnyElement {
        let id = self.id.clone();
        let mono = self.mono;
        let virtualized = virtual_list_state.is_some();
        let (width, truncate_labels) = if let Some(cx) = cx {
            if virtualized {
                resolve_virtual_menu_width(
                    self.width,
                    &self.items,
                    metrics,
                    self.max_height,
                    &tokens,
                    cx,
                    mono,
                )
            } else {
                resolve_menu_width(self.width, &self.items, metrics, &tokens, cx, mono)
            }
        } else {
            match self.width {
                MoonMenuWidth::Rendered(width) => (width, false),
                MoonMenuWidth::Scaled(_) | MoonMenuWidth::Fit { .. } => {
                    unreachable!("measured menu width reached a renderer without an App context")
                }
            }
        };
        let shadow = super::foundation::box_shadow(
            px(0.0),
            px(8.0),
            px(18.0),
            px(0.0),
            rgba_from(p.shadow, 0.46),
        );

        let mut menu = div()
            .id(ElementId::from(self.id.clone()))
            // Addressable from `VisualTestContext::debug_bounds` so a test can assert where the
            // menu lands after the deferred/anchored pass. Field, setter and paint-time record are
            // all `cfg`-gated to test builds, so this costs a release build nothing.
            .debug_selector(|| id.to_string())
            .relative()
            .w(px(width))
            .p(px(tokens.ui(MENU_PADDING)))
            .rounded(px(tokens.ui(5.0)))
            .border(px(1.0))
            .border_color(rgba_from(p.border, 1.0))
            .bg(rgba_from(p.shell_high, 0.98))
            .shadow(vec![shadow])
            .occlude()
            .flex()
            .flex_col()
            .gap(px(tokens.ui(MENU_GAP)));

        if virtual_list_state.is_none() {
            if let Some(max_height) = self.max_height {
                menu = menu
                    .max_h(px(max_height.resolve(&tokens)))
                    .overflow_y_scroll();
            }
        }

        for header in self.headers {
            menu = menu.child(header);
        }

        let row_context = std::rc::Rc::new(MenuLevelRenderContext {
            menu_id: id,
            mono,
            metrics,
            width_policy: self.width,
            size: self.size,
            width,
            truncate_labels,
            palette: p,
            tokens,
            dropdown_selection: self.dropdown_selection,
        });
        if let Some(list_state) = virtual_list_state {
            let (list_height, outer_max_height) =
                virtual_menu_heights(&self.items, metrics, self.max_height, &row_context.tokens);
            let item_count = self.items.len();
            let items = self.items;
            let row_context = row_context.clone();
            menu = menu.max_h(px(outer_max_height)).overflow_hidden().child(
                list(list_state, move |ix, _window, _cx| {
                    let item = items[ix].clone();
                    div()
                        .w_full()
                        .when(ix + 1 < item_count, |row| {
                            row.mb(px(row_context.tokens.ui(MENU_GAP)))
                        })
                        .child(Self::render_item(&row_context, ix, item, Some(_cx)))
                        .into_any_element()
                })
                .h(px(list_height))
                .w_full(),
            );
        } else {
            let items = std::rc::Rc::try_unwrap(self.items)
                .unwrap_or_else(|shared_items| shared_items.as_ref().clone());
            for (ix, item) in items.into_iter().enumerate() {
                menu = menu.child(Self::render_item(&row_context, ix, item, cx));
            }
        }

        menu.into_any_element()
    }

    /// Render one menu row and recursively render its selected submenu.
    ///
    /// Args:
    ///     context: Immutable level-wide identity, geometry, and theme inputs.
    ///     ix: Zero-based row index.
    ///     item: Row model.
    ///     cx: Optional application context used by measured width policies.
    ///
    /// Returns:
    ///     The rendered row element.
    fn render_item(
        context: &MenuLevelRenderContext,
        ix: usize,
        item: MoonMenuItem,
        cx: Option<&App>,
    ) -> AnyElement {
        let menu_id = &context.menu_id;
        let mono = context.mono;
        let metrics = context.metrics;
        let menu_width_policy = context.width_policy;
        let menu_size = context.size;
        let p = context.palette;
        let tokens = &context.tokens;
        let row_id = SharedString::from(format!("{}:item:{}", menu_id, ix));
        let mut item = item;
        #[cfg(test)]
        if item.label.starts_with(MENU_PALETTE_PROBE_PREFIX) {
            MENU_PALETTE_PROBE_SHELL.store(context.palette.shell as usize, Ordering::Relaxed);
        }
        if context.truncate_labels {
            fit_menu_item_label(
                &mut item,
                context.width,
                metrics,
                tokens,
                cx.expect("measured menu row requires an App context"),
                mono,
                menu_outer_chrome(tokens),
            );
        }

        match item.kind {
            MoonMenuItemKind::Separator => div()
                .id(ElementId::from(row_id.clone()))
                .debug_selector(move || row_id.to_string())
                .h(px(1.0))
                .mx(px(2.0))
                .my(px(3.0))
                .bg(rgba_from(p.border, 0.82))
                .into_any_element(),
            MoonMenuItemKind::Label => {
                let disabled = item.disabled;
                let actionable = moon_menu_item_accepts_click(
                    MoonMenuItemKind::Label,
                    disabled,
                    item.actionable,
                );
                let on_click = menu_item_click_handler(&item, context.dropdown_selection.as_ref());
                let mut row = div()
                    .id(ElementId::from(row_id.clone()))
                    .debug_selector({
                        let row_id = row_id.clone();
                        move || row_id.to_string()
                    })
                    .h(px(metrics.row_height))
                    .rounded(px(metrics.radius))
                    .px(px(metrics.pad_x))
                    .flex()
                    .items_center()
                    .when(actionable, |this| {
                        this.hover(move |this| this.bg(rgba_from(p.overlay, 0.055)))
                            .active(move |this| this.bg(rgba_from(p.overlay, 0.032)))
                    })
                    .child(
                        MoonText::new(item.label)
                            .color(p.text_muted)
                            .alpha(0.88)
                            .font_size(metrics.font_size)
                            .line_height(metrics.line_height)
                            .weight(500.0)
                            .mono(mono)
                            .uppercase(false)
                            .render(),
                    );

                if actionable {
                    if let Some(on_click) = on_click {
                        row = row
                            .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                                cx.stop_propagation();
                            })
                            .on_click(move |event, window, cx| {
                                on_click(event, window, cx);
                            });
                    }
                }

                row.into_any_element()
            }
            MoonMenuItemKind::Item => {
                let disabled = item.disabled;
                let selected = item.selected;
                let checked = item.checked;
                let on_click = menu_item_click_handler(&item, context.dropdown_selection.as_ref());
                let submenu = item.submenu;
                let has_submenu = !submenu.items.is_empty();
                let fg = if disabled {
                    p.text_muted
                } else if selected {
                    p.selected_fg()
                } else {
                    item.tone.color(p)
                };
                let alpha = if disabled { 0.45 } else { 1.0 };

                let mut row = div()
                    .id(ElementId::from(row_id.clone()))
                    .debug_selector({
                        let row_id = row_id.clone();
                        move || row_id.to_string()
                    })
                    .relative()
                    .h(px(metrics.row_height))
                    .rounded(px(metrics.radius))
                    .px(px(metrics.pad_x))
                    .flex()
                    .items_center()
                    .gap(px(metrics.gap))
                    .cursor_default()
                    .when(selected, |this| this.bg(selected_background(p)))
                    .when(!disabled, |this| {
                        this.hover(move |this| this.bg(rgba_from(p.overlay, 0.055)))
                            .active(move |this| this.bg(rgba_from(p.overlay, 0.032)))
                    })
                    .child(
                        div()
                            .w(px(menu_check_width(&tokens)))
                            .h(px(tokens.line_height(metrics.line_height)))
                            .flex()
                            .items_center()
                            .justify_center()
                            .when(checked, |this| {
                                this.child(moon_icon(
                                    MOON_ICON_CHECK,
                                    tokens.ui(11.0),
                                    p.accent,
                                    alpha,
                                ))
                            }),
                    )
                    .child(
                        MoonText::new(item.label)
                            .color(fg)
                            .alpha(alpha)
                            .font_size(metrics.font_size)
                            .line_height(metrics.line_height)
                            .weight(if selected { 600.0 } else { 400.0 })
                            .mono(mono)
                            .uppercase(false)
                            .render(),
                    )
                    .child(div().flex_1());

                if let Some(right_label) = item.right_label {
                    row = row.child(
                        MoonText::new(right_label)
                            .color(p.text_muted)
                            .alpha(alpha * 0.88)
                            .font_size(metrics.font_size - 0.5)
                            .line_height(metrics.line_height)
                            .weight(400.0)
                            .mono(mono)
                            .uppercase(false)
                            .render(),
                    );
                } else if has_submenu {
                    row = row.child(
                        MoonText::new("›")
                            .color(p.text_muted)
                            .alpha(alpha * 0.88)
                            .font_size(metrics.font_size)
                            .line_height(metrics.line_height)
                            .weight(600.0)
                            .mono(mono)
                            .uppercase(false)
                            .render(),
                    );
                }

                if moon_menu_item_accepts_click(MoonMenuItemKind::Item, disabled, false) {
                    if let Some(on_click) = on_click {
                        row = row
                            .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                                cx.stop_propagation();
                            })
                            .on_click(move |event, window, cx| {
                                on_click(event, window, cx);
                            });
                    }
                }

                if selected && has_submenu {
                    row = row.child(
                        deferred(
                            div()
                                .absolute()
                                .left_full()
                                .ml(px(tokens.ui(SUBMENU_OFFSET_X)))
                                .top(px(-tokens.ui(MENU_PADDING)))
                                .child(MoonPopupMenuResolvedTheme {
                                    menu: MoonPopupMenu::new(format!("{menu_id}:submenu:{ix}"))
                                        .shared_level(submenu)
                                        .width_policy(menu_width_policy)
                                        .size(menu_size),
                                    palette: p,
                                    tokens: tokens.clone(),
                                }),
                        )
                        .with_priority(1),
                    );
                }

                row.into_any_element()
            }
        }
    }

    /// Return unscaled row metrics for the configured menu size.
    ///
    /// Returns:
    ///     Design-reference row metrics.
    fn metrics(&self) -> MenuMetrics {
        match self.size {
            MoonMenuSize::Compact => MenuMetrics {
                row_height: 20.0,
                font_size: 9.5,
                line_height: 12.0,
                radius: 3.0,
                pad_x: 6.0,
                gap: 5.0,
            },
            MoonMenuSize::Normal => MenuMetrics {
                row_height: 24.0,
                font_size: 10.5,
                line_height: 13.0,
                radius: 4.0,
                pad_x: 7.0,
                gap: 6.0,
            },
            MoonMenuSize::Custom {
                row_height,
                font_size,
                line_height,
                radius,
                pad_x,
                gap,
            } => MenuMetrics {
                row_height,
                font_size,
                line_height,
                radius,
                pad_x,
                gap,
            },
        }
    }
}

impl RenderOnce for MoonPopupMenu {
    /// Render the menu with active theme tokens and measured fitted widths.
    ///
    /// Args:
    ///     window: Window that owns retained state for large menu levels.
    ///     cx: Application context used for active-theme resolution and text measurement.
    ///
    /// Returns:
    ///     The rendered menu.
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = MoonTheme::active_tokens(cx);
        let virtual_list_state = self.retained_virtual_list_state(&tokens, window, cx);
        self.render_with_theme(
            MoonPalette::active(cx),
            tokens,
            Some(cx),
            virtual_list_state,
        )
    }
}

impl RenderOnce for MoonPopupMenuResolvedTheme {
    /// Render a resolved-theme menu while retaining its virtual-list scroll state by menu id.
    ///
    /// Args:
    ///     window: Window that owns the keyed list state.
    ///     cx: Application context used to create or update retained state.
    ///
    /// Returns:
    ///     The popup rendered with the inherited palette and theme tokens.
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let virtual_list_state = self
            .menu
            .retained_virtual_list_state(&self.tokens, window, cx);
        self.menu
            .render_with_theme(self.palette, self.tokens, Some(cx), virtual_list_state)
    }
}

#[derive(Default)]
struct MoonDropdownState {
    open: bool,
}

#[derive(IntoElement)]
/// Moon-styled dropdown with component-owned trigger caret and width policies.
pub struct MoonDropdown {
    id: SharedString,
    bounds: Option<MoonRect>,
    label: SharedString,
    segments: Vec<MoonButtonSegment>,
    items: Vec<MoonMenuItem>,
    menu_layout: MenuLayoutFingerprint,
    trigger_variant: MoonButtonVariant,
    trigger_size: MoonButtonSize,
    trigger_leading_icon: Option<MoonButtonIconSlot>,
    trigger_width: MoonDropdownTriggerWidth,
    trigger_caret: bool,
    selected: bool,
    disabled: bool,
    default_open: bool,
    controlled_open: Option<bool>,
    menu_width: MoonMenuWidth,
    menu_offset_x: f32,
    menu_offset_y: f32,
    menu_size: MoonMenuSize,
    menu_max_height: Option<MoonMenuMaxHeight>,
    close_on_select: bool,
    on_select: Option<MoonSelectHandler>,
    on_open_change: Option<std::rc::Rc<dyn Fn(bool, &mut Window, &mut App)>>,
}

impl MoonDropdown {
    /// Create a dropdown with an intrinsic trigger and legacy rendered menu width.
    ///
    /// Args:
    ///     id: Stable element identity shared by the trigger and popup.
    ///
    /// Returns:
    ///     A default dropdown builder.
    pub fn new(id: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            bounds: None,
            label: SharedString::from(""),
            segments: Vec::new(),
            items: Vec::new(),
            menu_layout: MenuLayoutFingerprint::new(),
            trigger_variant: MoonButtonVariant::Neutral,
            trigger_size: MoonButtonSize::Toolbar,
            trigger_leading_icon: None,
            trigger_width: MoonDropdownTriggerWidth::Intrinsic,
            trigger_caret: false,
            selected: false,
            disabled: false,
            default_open: false,
            controlled_open: None,
            menu_width: MoonMenuWidth::Rendered(160.0),
            menu_offset_x: 0.0,
            menu_offset_y: 4.0,
            menu_size: MoonMenuSize::Normal,
            menu_max_height: None,
            close_on_select: true,
            on_select: None,
            on_open_change: None,
        }
    }

    pub fn bounds(mut self, bounds: MoonRect) -> Self {
        self.bounds = Some(bounds);
        self
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = label.into();
        self
    }

    pub fn segment(mut self, segment: MoonButtonSegment) -> Self {
        self.segments.push(segment);
        self
    }

    /// Append one dropdown row and update its popup-layout signature in the same pass.
    ///
    /// Args:
    ///     item: Menu row to append.
    ///
    /// Returns:
    ///     The updated dropdown.
    pub fn item(mut self, item: MoonMenuItem) -> Self {
        self.menu_layout.push(item.kind);
        self.items.push(item);
        self
    }

    /// Append dropdown rows while accumulating their popup-layout signature.
    ///
    /// Args:
    ///     items: Menu rows in display order.
    ///
    /// Returns:
    ///     The updated dropdown.
    pub fn items(mut self, items: impl IntoIterator<Item = MoonMenuItem>) -> Self {
        for item in items {
            self.menu_layout.push(item.kind);
            self.items.push(item);
        }
        self
    }

    pub fn trigger_variant(mut self, variant: MoonButtonVariant) -> Self {
        self.trigger_variant = variant;
        self
    }

    pub fn trigger_size(mut self, size: MoonButtonSize) -> Self {
        self.trigger_size = size;
        self
    }

    /// Set the trigger's leading icon from an asset path.
    ///
    /// Args:
    ///     path: Static asset path resolved by the Moon icon renderer.
    ///
    /// Returns:
    ///     The updated dropdown.
    pub fn trigger_icon(self, path: &'static str) -> Self {
        self.trigger_leading_icon(MoonButtonIconSlot::new(path))
    }

    /// Set the trigger's leading icon slot.
    ///
    /// Args:
    ///     icon: Configured Moon button icon slot rendered before trigger content.
    ///
    /// Returns:
    ///     The updated dropdown.
    pub fn trigger_leading_icon(mut self, icon: MoonButtonIconSlot) -> Self {
        self.trigger_leading_icon = Some(icon);
        self
    }

    /// Set a legacy rendered trigger width.
    ///
    /// Args:
    ///     width: Trigger width in rendered pixels.
    ///
    /// Returns:
    ///     The updated dropdown.
    pub fn trigger_width(mut self, width: f32) -> Self {
        self.trigger_width = MoonDropdownTriggerWidth::Rendered(width);
        self
    }

    /// Set a fixed design-reference trigger width scaled with the trigger's text metrics.
    ///
    /// Plain labels are truncated inside the resolved visual padding; segmented icon triggers
    /// receive only the scaled width.
    ///
    /// Args:
    ///     width: Trigger width at the configured font reference size.
    ///
    /// Returns:
    ///     The updated dropdown.
    pub fn trigger_width_scaled(mut self, width: f32) -> Self {
        self.trigger_width = MoonDropdownTriggerWidth::Scaled(width);
        self
    }

    /// Fit a plain-label trigger between font-scaled design-reference bounds.
    ///
    /// Args:
    ///     min_width: Minimum trigger width at the configured font reference size.
    ///     max_width: Maximum trigger width at the configured font reference size.
    ///
    /// Returns:
    ///     The updated dropdown.
    pub fn fit_trigger_width(mut self, min_width: f32, max_width: f32) -> Self {
        self.trigger_width = MoonDropdownTriggerWidth::Fit {
            min: min_width,
            max: max_width,
        };
        self
    }

    /// Show or hide the component-owned dropdown caret on a plain-label trigger.
    ///
    /// Args:
    ///     visible: Whether the caret is appended and reserved during fitting.
    ///
    /// Returns:
    ///     The updated dropdown.
    pub fn trigger_caret(mut self, visible: bool) -> Self {
        self.trigger_caret = visible;
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn default_open(mut self, default_open: bool) -> Self {
        self.default_open = default_open;
        self
    }

    pub fn open(mut self, open: bool) -> Self {
        self.controlled_open = Some(open);
        self
    }

    /// Set a legacy rendered menu width.
    ///
    /// Args:
    ///     width: Outer menu width in rendered pixels.
    ///
    /// Returns:
    ///     The updated dropdown.
    pub fn menu_width(mut self, width: f32) -> Self {
        self.menu_width = MoonMenuWidth::Rendered(width);
        self
    }

    /// Set a fixed design-reference menu width scaled with the selected menu size.
    ///
    /// Args:
    ///     width: Outer width at the configured font reference size.
    ///
    /// Returns:
    ///     The updated dropdown.
    pub fn menu_width_scaled(mut self, width: f32) -> Self {
        self.menu_width = MoonMenuWidth::Scaled(width);
        self
    }

    /// Fit each menu level to its own items between font-scaled design-reference bounds.
    ///
    /// Args:
    ///     min_width: Minimum outer width at the configured font reference size.
    ///     max_width: Maximum outer width at the configured font reference size.
    ///
    /// Returns:
    ///     The updated dropdown.
    pub fn fit_menu_width(mut self, min_width: f32, max_width: f32) -> Self {
        self.menu_width = MoonMenuWidth::Fit {
            min: min_width,
            max: max_width,
        };
        self
    }

    pub fn menu_offset(mut self, x: f32, y: f32) -> Self {
        self.menu_offset_x = x;
        self.menu_offset_y = y;
        self
    }

    pub fn menu_size(mut self, size: MoonMenuSize) -> Self {
        self.menu_size = size;
        self
    }

    /// Set a legacy maximum menu height in rendered pixels.
    ///
    /// Args:
    ///     max_height: Maximum rendered menu height.
    ///
    /// Returns:
    ///     The updated dropdown.
    pub fn menu_max_height(mut self, max_height: f32) -> Self {
        self.menu_max_height = Some(MoonMenuMaxHeight::Rendered(max_height));
        self
    }

    /// Set a UI-scaled design-reference maximum menu height.
    ///
    /// Args:
    ///     max_height: Maximum height at the configured UI reference scale.
    ///
    /// Returns:
    ///     The updated dropdown.
    pub fn menu_max_height_ui(mut self, max_height: f32) -> Self {
        self.menu_max_height = Some(MoonMenuMaxHeight::Ui(max_height));
        self
    }

    pub fn close_on_select(mut self, close_on_select: bool) -> Self {
        self.close_on_select = close_on_select;
        self
    }

    pub fn on_select(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select = Some(std::rc::Rc::new(handler));
        self
    }

    pub fn on_open_change(
        mut self,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_open_change = Some(std::rc::Rc::new(handler));
        self
    }

    /// Fit an external button label with the same caret, padding, and text metrics as a dropdown.
    ///
    /// This supports custom popover triggers that must align with a neighboring MoonDropdown
    /// without reproducing its private geometry. The external [`MoonButton`] must use
    /// `mono(true)` to match this measurement contract.
    ///
    /// Args:
    ///     cx: Application context used for active-theme text measurement.
    ///     label: Complete external trigger label without a caret.
    ///     size: MoonButton size used by the external trigger.
    ///     min_width: Minimum width at the configured font reference size.
    ///     max_width: Maximum width at the configured font reference size.
    ///
    /// Returns:
    ///     The fitted label with a component-owned caret and its rendered trigger width.
    pub fn fitted_trigger_label(
        cx: &App,
        label: &str,
        size: MoonButtonSize,
        min_width: f32,
        max_width: f32,
    ) -> (SharedString, f32) {
        let tokens = MoonTheme::active_tokens(cx);
        let (font_size, _, _) = button_text_metrics(size);
        let (label, width) = fit_dropdown_trigger_label(
            label,
            DROPDOWN_CARET,
            MoonDropdownTriggerWidth::Fit {
                min: min_width,
                max: max_width,
            },
            &tokens,
            font_size,
            0.0,
            |text| measure_text_width(cx, &tokens, text, font_size, 400.0, DROPDOWN_TRIGGER_MONO),
        );
        (
            label,
            width.expect("fitted trigger width always resolves to a rendered value"),
        )
    }

    /// Render the trigger after resolving its label and scaled width.
    ///
    /// Args:
    ///     cx: Application context used for active-theme text measurement.
    ///
    /// Returns:
    ///     The rendered MoonButton trigger.
    fn render_trigger(&self, cx: &App) -> impl IntoElement {
        let trigger_id = SharedString::from(format!("{}:trigger", self.id));
        let mut trigger = MoonButton::new(trigger_id)
            .variant(self.trigger_variant)
            .size(self.trigger_size)
            .selected(self.selected)
            .disabled(self.disabled);
        if self.segments.is_empty() {
            trigger = trigger.mono(DROPDOWN_TRIGGER_MONO);
        }

        let (font_size, _, _) = button_text_metrics(self.trigger_size);
        let tokens = MoonTheme::active_tokens(cx);
        let reserved_content_width = self.trigger_leading_icon.map_or(0.0, |_| {
            button_leading_icon_reservation(self.trigger_size, &tokens)
        });
        let suffix = if self.trigger_caret && self.segments.is_empty() {
            DROPDOWN_CARET
        } else {
            ""
        };
        let (label, resolved_width) = if self.segments.is_empty() {
            fit_dropdown_trigger_label(
                self.label.as_ref(),
                suffix,
                self.trigger_width,
                &tokens,
                font_size,
                reserved_content_width,
                |text| {
                    measure_text_width(cx, &tokens, text, font_size, 400.0, DROPDOWN_TRIGGER_MONO)
                },
            )
        } else {
            let width = match self.trigger_width {
                MoonDropdownTriggerWidth::Intrinsic => None,
                MoonDropdownTriggerWidth::Rendered(width) => Some(width),
                MoonDropdownTriggerWidth::Scaled(width) => {
                    Some(width * tokens.font(font_size) / font_size.max(1.0))
                }
                MoonDropdownTriggerWidth::Fit { min, .. } => {
                    Some(min * tokens.font(font_size) / font_size.max(1.0))
                }
            };
            (self.label.clone(), width)
        };

        if let Some(bounds) = self.bounds {
            trigger = trigger.bounds(MoonRect::new(0.0, 0.0, bounds.w, bounds.h));
        } else if let Some(width) = resolved_width {
            trigger = trigger.width(width);
        }

        if let Some(icon) = self.trigger_leading_icon {
            trigger = trigger.leading_icon(icon);
        }

        if self.segments.is_empty() {
            // Leave a truly empty icon trigger childless so MoonButton uses its square icon-only
            // layout instead of reserving text padding and a phantom content gap.
            if !label.is_empty() || self.trigger_leading_icon.is_none() {
                trigger = trigger.label(label);
            }
        } else {
            for segment in self.segments.clone() {
                trigger = trigger.segment(segment);
            }
        }

        trigger.render()
    }
}

impl RenderOnce for MoonDropdown {
    /// Renders the trigger and, while open, its deferred anchored menu.
    ///
    /// Args:
    ///     window: Window that owns keyed dropdown state and the deferred menu.
    ///     cx: Application context used for theme resolution and text measurement.
    ///
    /// Returns:
    ///     The rendered dropdown.
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state_id = ElementId::from(SharedString::from(format!("{}:moon-state", self.id)));
        let state = window.use_keyed_state(state_id, cx, |_, _| MoonDropdownState {
            open: self.default_open,
        });

        let controlled_open = self.controlled_open;
        let open = controlled_open.unwrap_or_else(|| state.read(cx).open);
        let on_open_change = self.on_open_change.clone();
        let parent_view = window.current_view();
        // How much the open menu must clear before its own gap. It depends on how the trigger is
        // laid out, so it is decided here rather than at the `.mt(..)` that consumes it.
        //
        // In flow (no caller-supplied bounds) the answer is ZERO: the anchor `CorePopover` hands
        // the menu already sits on the trigger's BOTTOM edge, because `ElementExt::on_prepaint`
        // measures an absolutely positioned canvas appended after the trigger, and in a block
        // container such a child takes its static position — below the preceding in-flow sibling.
        // Adding the height here too would push the menu down by a second trigger height.
        //
        // With caller-supplied bounds `MoonButton::bounds` renders the trigger ABSOLUTELY, which
        // leaves the popover host without a single in-flow child: its auto height collapses to
        // zero, the `size_full` canvas collapses with it, and the capture lands on the trigger's
        // TOP edge. Only there does the supplied height have to be added back.
        let trigger_height = self.bounds.map_or(0.0, |bounds| bounds.h);
        let trigger = self.render_trigger(cx).into_any_element();

        let mut root = div()
            .id(ElementId::from(SharedString::from(format!(
                "{}:root",
                self.id
            ))))
            .relative();

        if let Some(bounds) = self.bounds {
            root = root
                .absolute()
                .left(px(bounds.x))
                .top(px(bounds.y))
                .w(px(bounds.w))
                .h(px(bounds.h));
        }

        if self.disabled {
            return root.child(trigger).into_any_element();
        }

        let menu_id = SharedString::from(format!("{}:menu", self.id));
        let items = self.items;
        let menu_layout = self.menu_layout;
        let menu_size = self.menu_size;
        let menu_width = self.menu_width;
        let menu_max_height = self.menu_max_height;
        let menu_offset_x = self.menu_offset_x;
        let menu_offset_y = self.menu_offset_y;
        let popup_level = MoonMenuLevel::from_parts(std::rc::Rc::new(items), menu_layout);
        let dropdown_selection = std::rc::Rc::new(MoonDropdownSelectionContext {
            close_on_select: self.close_on_select,
            on_select: self.on_select,
            state: state.clone(),
            controlled_open,
            on_open_change: self.on_open_change.clone(),
            parent_view,
        });

        let mut popover = CorePopover::new(ElementId::from(self.id.clone()))
            .appearance(false)
            .anchor(Anchor::TopLeft)
            .deferred_priority(30_000)
            .open(open)
            .trigger_any(trigger)
            .content(move |_, _window, cx| {
                let tokens = MoonTheme::active_tokens(cx);
                let mut menu = MoonPopupMenu::new(menu_id.clone())
                    .shared_level(popup_level.clone())
                    .dropdown_selection(dropdown_selection.clone())
                    .size(menu_size)
                    .width_policy(menu_width)
                    .mono(true);
                if let Some(max_height) = menu_max_height {
                    menu = menu.max_height_policy(max_height);
                }

                div()
                    .mt(px(trigger_height + tokens.ui(menu_offset_y)))
                    .ml(px(tokens.ui(menu_offset_x)))
                    .child(menu)
            });

        {
            let state = state.clone();
            let on_open_change = on_open_change.clone();
            popover = popover.on_open_change(move |open, window, cx| {
                if let Some(on_open_change) = on_open_change.as_ref() {
                    on_open_change(*open, window, cx);
                }
                if controlled_open.is_none() {
                    state.update(cx, |state, _| {
                        state.open = *open;
                    });
                    cx.notify(parent_view);
                }
            });
        }

        root.child(popover).into_any_element()
    }
}

#[cfg(test)]
mod tests;
