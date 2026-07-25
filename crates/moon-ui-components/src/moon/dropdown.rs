use crate::popover::Popover as CorePopover;
use gpui::prelude::FluentBuilder;
use gpui::*;

use super::{
    button::{
        MoonButton, MoonButtonSegment, MoonButtonSize, MoonButtonVariant, button_text_metrics,
    },
    foundation::{MoonClickHandler, MoonSelectHandler, selected_background},
    icons::{MOON_ICON_CHECK, moon_icon},
    text::{MoonText, fit_text_to_width, fit_text_with_suffix, measure_text_width},
    theme::{MoonTheme, MoonThemeTokens},
    tokens::{MoonPalette, MoonRect, MoonTone, rgba_from},
};

const MENU_PADDING: f32 = 4.0;
const MENU_BORDER: f32 = 1.0;
const MENU_CHECK_WIDTH: f32 = 12.0;
const SUBMENU_OFFSET_X: f32 = 2.0;
const DROPDOWN_TRIGGER_PAD_X: f32 = 14.0;
const DROPDOWN_CARET: &str = " \u{25be}";
const DROPDOWN_TRIGGER_MONO: bool = true;

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
    let visual_padding = tokens.ui(DROPDOWN_TRIGGER_PAD_X);
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

#[derive(Clone, Copy, Debug)]
struct MenuMetrics {
    row_height: f32,
    font_size: f32,
    line_height: f32,
    radius: f32,
    pad_x: f32,
    gap: f32,
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
        match item.kind {
            MoonMenuItemKind::Separator => {}
            MoonMenuItemKind::Label => {
                let budget = (width - outer - metrics.pad_x * 2.0).max(0.0);
                let fitted = fit_text_to_width(item.label.as_ref(), budget, |text| {
                    measure_text_width(cx, tokens, text, metrics.font_size, 500.0, mono)
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
                        measure_text_width(cx, tokens, "\u{2026}", metrics.font_size, 600.0, mono);
                    let right_budget = (text_budget - main_ellipsis).max(0.0);
                    let (right_label, right_width) =
                        fit_text_to_width(right_label.as_ref(), right_budget, |text| {
                            measure_text_width(
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
                        - measure_text_width(
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
                    measure_text_width(cx, tokens, text, metrics.font_size, 600.0, mono)
                })
                .0;
                item.label = SharedString::from(fitted);
            }
        }
    }
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
        measure_text_width(cx, tokens, text, size, weight, mono)
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

fn moon_menu_item_accepts_click(kind: MoonMenuItemKind, disabled: bool) -> bool {
    matches!(kind, MoonMenuItemKind::Item) && !disabled
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MoonDropdownSelectPlan {
    close_menu: bool,
    update_internal_open: bool,
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

#[derive(Clone)]
pub struct MoonMenuItem {
    key: SharedString,
    label: SharedString,
    kind: MoonMenuItemKind,
    right_label: Option<SharedString>,
    tone: MoonTone,
    selected: bool,
    checked: bool,
    disabled: bool,
    submenu: Vec<MoonMenuItem>,
    on_click: Option<MoonClickHandler>,
}

impl MoonMenuItem {
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
            submenu: Vec::new(),
            on_click: None,
        }
    }

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
            submenu: Vec::new(),
            on_click: None,
        }
    }

    pub fn label(label: impl Into<SharedString>) -> Self {
        let mut item = Self::new(label);
        item.kind = MoonMenuItemKind::Label;
        item.disabled = true;
        item
    }

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
            submenu: Vec::new(),
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

    pub fn submenu(mut self, items: impl IntoIterator<Item = MoonMenuItem>) -> Self {
        self.submenu = items.into_iter().collect();
        self
    }

    pub fn has_submenu(&self) -> bool {
        !self.submenu.is_empty()
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
    items: Vec<MoonMenuItem>,
    size: MoonMenuSize,
    width: MoonMenuWidth,
    max_height: Option<MoonMenuMaxHeight>,
    mono: bool,
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
            items: Vec::new(),
            size: MoonMenuSize::Normal,
            width: MoonMenuWidth::Rendered(160.0),
            max_height: None,
            mono: true,
        }
    }

    pub fn item(mut self, item: MoonMenuItem) -> Self {
        self.items.push(item);
        self
    }

    pub fn header(mut self, header: impl IntoElement) -> Self {
        self.headers.push(header.into_any_element());
        self
    }

    pub fn items(mut self, items: impl IntoIterator<Item = MoonMenuItem>) -> Self {
        self.items.extend(items);
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
        self.render_with_theme(p, MoonThemeTokens::default(), None)
    }

    /// Render with explicit palette/tokens and an optional text-measurement context.
    ///
    /// Args:
    ///     p: Palette used to paint the menu.
    ///     tokens: Tokens used to resolve menu geometry.
    ///     cx: Optional application context; fitted policies require it for text measurement.
    ///
    /// Returns:
    ///     The rendered menu.
    fn render_with_theme(
        self,
        p: MoonPalette,
        tokens: MoonThemeTokens,
        cx: Option<&App>,
    ) -> AnyElement {
        let metrics = self.metrics().scaled(&tokens);
        self.render_with_metrics(p, metrics, tokens, cx)
    }

    /// Renders the menu with precomputed layout metrics and the supplied theme tokens.
    ///
    /// Args:
    ///     p: Palette used to paint the menu.
    ///     metrics: Resolved menu row metrics.
    ///     tokens: Active theme tokens.
    ///     cx: Optional application context used by fitted policies.
    ///
    /// Returns:
    ///     The rendered menu.
    fn render_with_metrics(
        mut self,
        p: MoonPalette,
        metrics: MenuMetrics,
        tokens: MoonThemeTokens,
        cx: Option<&App>,
    ) -> AnyElement {
        let id = self.id.clone();
        let mono = self.mono;
        let (width, truncate_labels) = if let Some(cx) = cx {
            resolve_menu_width(self.width, &self.items, metrics, &tokens, cx, mono)
        } else {
            match self.width {
                MoonMenuWidth::Rendered(width) => (width, false),
                MoonMenuWidth::Scaled(_) | MoonMenuWidth::Fit { .. } => {
                    unreachable!("measured menu width reached a renderer without an App context")
                }
            }
        };
        if truncate_labels {
            fit_menu_item_labels(
                &mut self.items,
                width,
                metrics,
                &tokens,
                cx.expect("measured menu layout requires an App context"),
                mono,
            );
        }
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
            .gap(px(tokens.ui(2.0)));

        if let Some(max_height) = self.max_height {
            menu = menu
                .max_h(px(max_height.resolve(&tokens)))
                .overflow_y_scroll();
        }

        for header in self.headers {
            menu = menu.child(header);
        }

        for (ix, item) in self.items.into_iter().enumerate() {
            menu = menu.child(Self::render_item(
                &id,
                mono,
                ix,
                item,
                metrics,
                self.width,
                p,
                tokens.clone(),
                cx,
            ));
        }

        menu.into_any_element()
    }

    /// Render one menu row and recursively render its selected submenu.
    ///
    /// Args:
    ///     menu_id: Parent menu identity used for stable row and submenu ids.
    ///     mono: Whether row text uses the configured monospaced family.
    ///     ix: Zero-based row index.
    ///     item: Row model.
    ///     metrics: Resolved row metrics.
    ///     menu_width_policy: Policy each submenu resolves against its own rows.
    ///     p: Active palette.
    ///     tokens: Active theme tokens.
    ///     cx: Optional application context used by fitted submenus.
    ///
    /// Returns:
    ///     The rendered row element.
    fn render_item(
        menu_id: &SharedString,
        mono: bool,
        ix: usize,
        item: MoonMenuItem,
        metrics: MenuMetrics,
        menu_width_policy: MoonMenuWidth,
        p: MoonPalette,
        tokens: MoonThemeTokens,
        cx: Option<&App>,
    ) -> AnyElement {
        let row_id = SharedString::from(format!("{}:item:{}", menu_id, ix));

        match item.kind {
            MoonMenuItemKind::Separator => div()
                .id(ElementId::from(row_id))
                .h(px(1.0))
                .mx(px(2.0))
                .my(px(3.0))
                .bg(rgba_from(p.border, 0.82))
                .into_any_element(),
            MoonMenuItemKind::Label => div()
                .id(ElementId::from(row_id))
                .h(px(metrics.row_height))
                .px(px(metrics.pad_x))
                .flex()
                .items_center()
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
                )
                .into_any_element(),
            MoonMenuItemKind::Item => {
                let disabled = item.disabled;
                let selected = item.selected;
                let checked = item.checked;
                let submenu = item.submenu;
                let has_submenu = !submenu.is_empty();
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

                if moon_menu_item_accepts_click(MoonMenuItemKind::Item, disabled) {
                    if let Some(on_click) = item.on_click {
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
                        div()
                            .absolute()
                            .left_full()
                            .ml(px(tokens.ui(SUBMENU_OFFSET_X)))
                            .top(px(-tokens.ui(MENU_PADDING)))
                            .child(
                                MoonPopupMenu::new(format!("{menu_id}:submenu:{ix}"))
                                    .items(submenu)
                                    .width_policy(menu_width_policy)
                                    .render_with_metrics(p, metrics, tokens.clone(), cx),
                            ),
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
    ///     cx: Application context used for active-theme resolution and text measurement.
    ///
    /// Returns:
    ///     The rendered menu.
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = MoonTheme::active_tokens(cx);
        self.render_with_theme(MoonPalette::active(cx), tokens, Some(cx))
    }
}

fn wire_dropdown_items(
    items: Vec<MoonMenuItem>,
    close_on_select: bool,
    on_select: Option<MoonSelectHandler>,
    state: Entity<MoonDropdownState>,
    controlled_open: Option<bool>,
    on_open_change: Option<std::rc::Rc<dyn Fn(bool, &mut Window, &mut App)>>,
    parent_view: EntityId,
) -> Vec<MoonMenuItem> {
    items
        .into_iter()
        .map(|mut item| {
            if moon_menu_item_accepts_click(item.kind, item.disabled) {
                let key = item.key.clone();
                let existing_handler = item.on_click.clone();
                let on_select = on_select.clone();
                let state = state.clone();
                let on_open_change = on_open_change.clone();
                item.on_click = Some(std::rc::Rc::new(move |event, window, cx| {
                    let plan = moon_dropdown_select_plan(close_on_select, controlled_open);
                    if let Some(existing_handler) = existing_handler.as_ref() {
                        existing_handler(event, window, cx);
                    }
                    if let Some(on_select) = on_select.as_ref() {
                        on_select(&key, window, cx);
                    }
                    if plan.close_menu {
                        if let Some(on_open_change) = on_open_change.as_ref() {
                            on_open_change(false, window, cx);
                        }
                        if plan.update_internal_open {
                            state.update(cx, |state, _| {
                                state.open = false;
                            });
                            cx.notify(parent_view);
                        }
                    }
                }));
            }
            item
        })
        .collect()
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
    trigger_variant: MoonButtonVariant,
    trigger_size: MoonButtonSize,
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
            trigger_variant: MoonButtonVariant::Neutral,
            trigger_size: MoonButtonSize::Toolbar,
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

    pub fn item(mut self, item: MoonMenuItem) -> Self {
        self.items.push(item);
        self
    }

    pub fn items(mut self, items: impl IntoIterator<Item = MoonMenuItem>) -> Self {
        self.items.extend(items);
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

        if self.segments.is_empty() {
            trigger = trigger.label(label);
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
        let menu_size = self.menu_size;
        let menu_width = self.menu_width;
        let menu_max_height = self.menu_max_height;
        let menu_offset_x = self.menu_offset_x;
        let menu_offset_y = self.menu_offset_y;
        let close_on_select = self.close_on_select;
        let on_select = self.on_select.clone();
        let popup_items = wire_dropdown_items(
            items,
            close_on_select,
            on_select,
            state.clone(),
            controlled_open,
            self.on_open_change.clone(),
            parent_view,
        );

        let mut popover = CorePopover::new(ElementId::from(self.id.clone()))
            .appearance(false)
            .anchor(Anchor::TopLeft)
            .deferred_priority(30_000)
            .open(open)
            .trigger_any(trigger)
            .content(move |_, _window, cx| {
                let p = MoonPalette::active(cx);
                let tokens = MoonTheme::active_tokens(cx);
                let mut menu = MoonPopupMenu::new(menu_id.clone())
                    .items(popup_items.clone())
                    .size(menu_size)
                    .width_policy(menu_width)
                    .mono(true);
                if let Some(max_height) = menu_max_height {
                    menu = menu.max_height_policy(max_height);
                }

                div()
                    .mt(px(trigger_height + tokens.ui(menu_offset_y)))
                    .ml(px(tokens.ui(menu_offset_x)))
                    .child(menu.render_with_theme(p, tokens, Some(cx)))
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
