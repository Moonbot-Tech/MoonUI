//! Popup-menu width, height, and text-measurement policy.

use gpui::{App, SharedString};

use super::super::{
    text::{fit_text_to_width, measure_text_width},
    theme::MoonThemeTokens,
};
use super::{
    MoonMenuItem, MoonMenuItemKind, MoonMenuLevel, MoonMenuSize, VIRTUAL_MENU_ITEM_THRESHOLD,
};

#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering};

pub(super) const MENU_PADDING: f32 = 4.0;
const MENU_BORDER: f32 = 1.0;
pub(super) const MENU_GAP: f32 = 2.0;
pub(crate) const MENU_CHECK_WIDTH: f32 = 12.0;
/// Font-size step-down of trailing text against its row label.
pub(super) const MENU_TRAILING_FONT_DELTA: f32 = 0.5;
/// Font weight shared by trailing-text measurement, fitting, and rendering.
pub(super) const MENU_TRAILING_WEIGHT: f32 = 400.0;
const DEFAULT_VIRTUAL_MENU_MAX_HEIGHT: f32 = 320.0;
#[cfg(test)]
pub(super) const MENU_MEASUREMENT_PROBE_PREFIX: &str = "moon-menu-measurement-probe-";
#[cfg(test)]
static MENU_MEASUREMENT_PROBE_COUNT: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Copy, Debug, PartialEq)]
/// Width policy for one popup-menu level.
pub(in crate::moon) enum MoonMenuWidth {
    Rendered(f32),
    Scaled(f32),
    Fit { min: f32, max: f32 },
}

impl MoonMenuWidth {
    /// Return whether this policy measures content against active font metrics.
    pub(super) fn is_measured(self) -> bool {
        !matches!(self, Self::Rendered(_))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
/// Maximum-height policy for a popup menu.
pub(super) enum MoonMenuMaxHeight {
    Rendered(f32),
    Ui(f32),
}

impl MoonMenuMaxHeight {
    /// Resolve the maximum menu height into rendered pixels.
    pub(super) fn resolve(self, tokens: &MoonThemeTokens) -> f32 {
        match self {
            Self::Rendered(height) => height,
            Self::Ui(height) => tokens.ui(height),
        }
    }
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

impl MenuMetrics {
    /// Scale menu geometry while retaining design-reference text inputs.
    pub(super) fn scaled(self, tokens: &MoonThemeTokens) -> Self {
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

/// Return unscaled row metrics for one menu size.
pub(super) fn unscaled_menu_metrics(size: MoonMenuSize) -> MenuMetrics {
    match size {
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

/// Return menu row metrics resolved for the active scale.
pub(crate) fn menu_row_metrics(size: MoonMenuSize, tokens: &MoonThemeTokens) -> MenuMetrics {
    unscaled_menu_metrics(size).scaled(tokens)
}

/// Return the fixed outer chrome around a menu row.
pub(super) fn menu_outer_chrome(tokens: &MoonThemeTokens) -> f32 {
    tokens.ui(MENU_PADDING) * 2.0 + MENU_BORDER * 2.0
}

/// Return whether a menu level requires bounded element construction.
pub(super) fn menu_level_is_virtualized(item_count: usize) -> bool {
    item_count >= VIRTUAL_MENU_ITEM_THRESHOLD
}

/// Compute natural mixed-row height only until the viewport is full.
pub(super) fn capped_menu_items_height(
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

/// Resolve the outer height ceiling for one menu level.
pub(super) fn resolve_menu_outer_max(
    max_height: Option<MoonMenuMaxHeight>,
    tokens: &MoonThemeTokens,
    virtualized: bool,
) -> f32 {
    max_height
        .map(|max_height| max_height.resolve(tokens))
        .unwrap_or(if virtualized {
            tokens.ui(DEFAULT_VIRTUAL_MENU_MAX_HEIGHT)
        } else {
            f32::INFINITY
        })
}

/// Resolve row-list height after paying for chrome and pinned headers.
pub(super) fn menu_content_max(
    outer_max: f32,
    tokens: &MoonThemeTokens,
    header_budget: f32,
    metrics: MenuMetrics,
) -> f32 {
    (outer_max - menu_outer_chrome(tokens) - header_budget).max(metrics.row_height)
}

/// Cap declared header height while preserving at least one menu row.
pub(super) fn clamp_header_budget(
    declared: f32,
    outer_max: f32,
    chrome: f32,
    row_height: f32,
) -> f32 {
    if declared <= 0.0 {
        return 0.0;
    }
    if !outer_max.is_finite() {
        return declared;
    }
    declared.min((outer_max - chrome - row_height).max(0.0))
}

/// Resolve a bounded virtual-list viewport height.
pub(super) fn virtual_menu_list_height(
    items: &[MoonMenuItem],
    metrics: MenuMetrics,
    tokens: &MoonThemeTokens,
    content_max: f32,
) -> f32 {
    capped_menu_items_height(
        items.iter().map(|item| item.kind),
        metrics,
        tokens,
        content_max,
    )
}

/// Measure trailing text at its natural and ellipsis-only widths.
fn trailing_label_widths(
    right_label: Option<&SharedString>,
    metrics: MenuMetrics,
    measure: &mut impl FnMut(&str, f32, f32) -> f32,
) -> (f32, f32) {
    let Some(right_label) = right_label else {
        return (0.0, 0.0);
    };
    let font_size = metrics.font_size - MENU_TRAILING_FONT_DELTA;
    let natural = measure(right_label.as_ref(), font_size, MENU_TRAILING_WEIGHT);
    let minimum = if right_label.is_empty() {
        0.0
    } else {
        measure("\u{2026}", font_size, MENU_TRAILING_WEIGHT)
    };
    (natural, minimum)
}

/// Fit trailing text first while reserving an ellipsis for the main label.
fn fit_trailing_label(
    right_label: &mut Option<SharedString>,
    budget: &mut f32,
    main_weight: f32,
    metrics: MenuMetrics,
    tokens: &MoonThemeTokens,
    cx: &App,
    mono: bool,
) {
    let Some(text) = right_label.take() else {
        return;
    };
    let main_ellipsis =
        measure_menu_text_width(cx, tokens, "\u{2026}", metrics.font_size, main_weight, mono);
    let trailing_budget = (*budget - main_ellipsis).max(0.0);
    let (fitted, consumed) = fit_text_to_width(text.as_ref(), trailing_budget, |text| {
        measure_menu_text_width(
            cx,
            tokens,
            text,
            metrics.font_size - MENU_TRAILING_FONT_DELTA,
            MENU_TRAILING_WEIGHT,
            mono,
        )
    });
    *right_label = Some(SharedString::from(fitted));
    *budget = (*budget - consumed).max(0.0);
}

/// Return the UI-scaled check-column width.
pub(super) fn menu_check_width(tokens: &MoonThemeTokens) -> f32 {
    tokens.ui(MENU_CHECK_WIDTH)
}

#[derive(Clone, Copy, Debug, PartialEq)]
/// Natural and minimum viable outer widths for one menu level.
struct MenuWidthRequirements {
    natural: f32,
    minimum: f32,
}

/// Measure natural and minimum viable widths in one pass.
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
                let (trailing_natural, trailing_minimum) =
                    trailing_label_widths(item.right_label.as_ref(), metrics, &mut measure);
                let trailing_gap = if item.right_label.is_some() {
                    metrics.gap
                } else {
                    0.0
                };
                let chrome = metrics.pad_x * 2.0 + trailing_gap;
                let natural = chrome
                    + measure(item.label.as_ref(), metrics.font_size, 500.0)
                    + trailing_natural;
                let marker = if item.label.is_empty() {
                    0.0
                } else {
                    measure("\u{2026}", metrics.font_size, 500.0)
                };
                (natural, chrome + marker + trailing_minimum)
            }
            MoonMenuItemKind::Item => {
                let (trailing_natural, trailing_minimum) = if item.right_label.is_some() {
                    trailing_label_widths(item.right_label.as_ref(), metrics, &mut measure)
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

/// Return the natural width required by the widest row.
#[cfg(test)]
pub(super) fn natural_menu_width(
    items: &[MoonMenuItem],
    metrics: MenuMetrics,
    tokens: &MoonThemeTokens,
    measure: impl FnMut(&str, f32, f32) -> f32,
) -> f32 {
    menu_width_requirements(items, metrics, tokens, measure).natural
}

/// Fit every row label to one resolved menu width.
#[cfg(test)]
pub(super) fn fit_menu_item_labels(
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

/// Fit one row label and trailing label to a resolved menu width.
pub(super) fn fit_menu_item_label(
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
            let trailing_gap = if item.right_label.is_some() {
                metrics.gap
            } else {
                0.0
            };
            let mut budget = (width - outer - metrics.pad_x * 2.0 - trailing_gap).max(0.0);
            fit_trailing_label(
                &mut item.right_label,
                &mut budget,
                500.0,
                metrics,
                tokens,
                cx,
                mono,
            );
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

            if item.right_label.is_some() {
                fit_trailing_label(
                    &mut item.right_label,
                    &mut text_budget,
                    600.0,
                    metrics,
                    tokens,
                    cx,
                    mono,
                );
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

/// Measure menu text while exposing sentinel calls to regression tests.
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

/// Return the current sentinel-only measurement count.
#[cfg(test)]
pub(super) fn menu_measurement_probe_count() -> usize {
    MENU_MEASUREMENT_PROBE_COUNT.load(Ordering::Relaxed)
}

/// Resolve a menu width and whether its labels require truncation.
pub(super) fn resolve_menu_width(
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

/// Return the prefix that can contribute to a virtual menu's initial viewport.
fn virtual_menu_initial_rows<'a>(
    items: &'a [MoonMenuItem],
    metrics: MenuMetrics,
    tokens: &MoonThemeTokens,
    content_max: f32,
) -> &'a [MoonMenuItem] {
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

/// Number of leading rows used for bounded virtual-menu width measurement.
pub(super) const MENU_WIDTH_SAMPLE_ROWS: usize = 128;

/// Return the bounded leading sample used for virtual-menu width measurement.
fn menu_width_sample(items: &[MoonMenuItem], height_rows: usize) -> &[MoonMenuItem] {
    let take = height_rows.max(MENU_WIDTH_SAMPLE_ROWS).min(items.len());
    &items[..take]
}

/// Resolve virtual-menu width from a bounded leading row sample.
pub(super) fn resolve_virtual_menu_width(
    policy: MoonMenuWidth,
    items: &[MoonMenuItem],
    metrics: MenuMetrics,
    tokens: &MoonThemeTokens,
    cx: &App,
    mono: bool,
    content_max: f32,
) -> (f32, bool) {
    if let MoonMenuWidth::Rendered(width) = policy {
        return (width, false);
    }

    let initial_rows = virtual_menu_initial_rows(items, metrics, tokens, content_max);
    let measured_rows = menu_width_sample(items, initial_rows.len());
    let text_scale = tokens.font(metrics.font_size) / metrics.font_size.max(1.0);
    let requirements =
        menu_width_requirements(measured_rows, metrics, tokens, |text, size, weight| {
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
        measured_rows.len() < items.len() || width < requirements.natural,
    )
}

/// Resolve one retained level's width under an optional rendered ceiling.
pub(in crate::moon) fn resolve_menu_level_width(
    policy: MoonMenuWidth,
    level: &MoonMenuLevel,
    size: MoonMenuSize,
    tokens: &MoonThemeTokens,
    cx: &App,
    mono: bool,
    rendered_max_width: Option<f32>,
) -> (f32, bool) {
    let metrics = menu_row_metrics(size, tokens);
    let virtualized = menu_level_is_virtualized(level.len());
    let outer_max = resolve_menu_outer_max(None, tokens, virtualized);
    let content_max = menu_content_max(outer_max, tokens, 0.0, metrics);
    let (mut width, mut truncate) = if virtualized {
        resolve_virtual_menu_width(
            policy,
            level.as_slice(),
            metrics,
            tokens,
            cx,
            mono,
            content_max,
        )
    } else {
        resolve_menu_width(policy, level.as_slice(), metrics, tokens, cx, mono)
    };
    if policy.is_measured()
        && let Some(max_width) = rendered_max_width
        && width > max_width
    {
        width = max_width.max(1.0);
        truncate = true;
    }
    (width, truncate)
}

/// Resolve one retained level's natural outer height inside a rendered cap.
pub(in crate::moon) fn menu_level_outer_height(
    level: &MoonMenuLevel,
    size: MoonMenuSize,
    tokens: &MoonThemeTokens,
    rendered_max_height: f32,
) -> f32 {
    let metrics = menu_row_metrics(size, tokens);
    let chrome = menu_outer_chrome(tokens);
    let content_max = (rendered_max_height - chrome).max(metrics.row_height);
    let content = capped_menu_items_height(
        level.as_slice().iter().map(|item| item.kind),
        metrics,
        tokens,
        content_max,
    );
    (chrome + content).min(rendered_max_height)
}
