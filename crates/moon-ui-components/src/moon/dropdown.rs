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
    text::{MoonText, fit_text_with_suffix, measure_text_width},
    theme::{MoonTheme, MoonThemeTokens},
    tokens::{MoonPalette, MoonRect, MoonTone, rgba_from},
};

#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering};

const SUBMENU_OFFSET_X: f32 = 2.0;
const DROPDOWN_TRIGGER_PAD_X: f32 = 14.0;
// Keep this caret as a text suffix rather than a `MoonDisclosure` element: its text advance is part
// of the width measured by `fit_dropdown_trigger_label` and exposed through
// `MoonDropdown::fitted_trigger_label`. An element has no text advance and therefore cannot satisfy
// the fitted-label width contract.
const DROPDOWN_CARET: &str = " \u{25be}";
const DROPDOWN_TRIGGER_MONO: bool = true;
const VIRTUAL_MENU_ITEM_THRESHOLD: usize = 64;
#[cfg(test)]
const MENU_CLONE_PROBE_PREFIX: &str = "moon-menu-clone-probe-";
#[cfg(test)]
const MENU_PALETTE_PROBE_PREFIX: &str = "moon-menu-palette-probe-";
#[cfg(test)]
static MENU_ITEM_CLONE_PROBE_COUNT: AtomicUsize = AtomicUsize::new(0);
#[cfg(test)]
static MENU_PALETTE_PROBE_SHELL: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Copy, Debug, PartialEq)]
/// Width policy for a dropdown's plain-label trigger.
enum MoonDropdownTriggerWidth {
    Intrinsic,
    Rendered(f32),
    Scaled(f32),
    Fit { min: f32, max: f32 },
}

mod layout;
mod selection;

pub(crate) use layout::{MENU_CHECK_WIDTH, MenuMetrics, menu_row_metrics};
use layout::{
    MENU_GAP, MENU_PADDING, MENU_TRAILING_FONT_DELTA, MENU_TRAILING_WEIGHT, MoonMenuMaxHeight,
    clamp_header_budget, fit_menu_item_label, menu_check_width, menu_content_max,
    menu_level_is_virtualized, menu_outer_chrome, resolve_menu_outer_max, resolve_menu_width,
    resolve_virtual_menu_width, unscaled_menu_metrics, virtual_menu_list_height,
};
#[cfg(test)]
use layout::{
    MENU_MEASUREMENT_PROBE_PREFIX, MENU_WIDTH_SAMPLE_ROWS, capped_menu_items_height,
    fit_menu_item_labels, menu_measurement_probe_count, natural_menu_width,
};
pub(super) use layout::{MoonMenuWidth, menu_level_outer_height, resolve_menu_level_width};
use selection::{
    MoonDropdownSelectionContext, menu_item_click_handler, moon_menu_item_accepts_click,
};

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

    /// Borrow the retained rows for sibling layout consumers.
    ///
    /// Returns:
    ///     The ordered immutable menu rows.
    pub(super) fn as_slice(&self) -> &[MoonMenuItem] {
        self.items.as_slice()
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

/// Render a row's trailing text.
///
/// The render half of the trailing-text policy; measurement and fitting use
/// the same font-size delta and font weight in the layout module.
///
/// Args:
///     right_label: Already-fitted trailing text.
///     p: Active palette.
///     metrics: Resolved row geometry.
///     mono: Whether the row uses the configured monospaced font.
///     alpha: Row alpha the trailing text is dimmed against.
///
/// Returns:
///     The rendered trailing text.
fn menu_trailing_label(
    right_label: SharedString,
    p: MoonPalette,
    metrics: MenuMetrics,
    mono: bool,
    alpha: f32,
) -> AnyElement {
    MoonText::new(right_label)
        .color(p.text_muted)
        .alpha(alpha)
        .font_size(metrics.font_size - MENU_TRAILING_FONT_DELTA)
        .line_height(metrics.line_height)
        .weight(MENU_TRAILING_WEIGHT)
        .mono(mono)
        .uppercase(false)
        .render()
        .into_any_element()
}

/// Reset and return the menu-row clone probe used by virtual repaint regressions.
///
/// Returns:
///     Number of row clones recorded since the previous reset.
#[cfg(test)]
pub(super) fn take_menu_item_clone_probe_count() -> usize {
    MENU_ITEM_CLONE_PROBE_COUNT.swap(0, Ordering::Relaxed)
}

/// Reset and return the last palette shell observed by a probe submenu row.
///
/// Returns:
///     Palette shell color, or zero when no probe row rendered.
#[cfg(test)]
fn take_palette_probe_shell() -> u32 {
    MENU_PALETTE_PROBE_SHELL.swap(0, Ordering::Relaxed) as u32
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
    /// Per-row override of the dropdown's `close_on_select`; `None` follows the dropdown.
    closes_menu: Option<bool>,
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
            closes_menu: self.closes_menu,
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
            closes_menu: None,
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
            closes_menu: None,
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
            closes_menu: None,
        }
    }

    pub fn key(&self) -> &SharedString {
        &self.key
    }

    /// Set the muted trailing text rendered at the row's right edge.
    ///
    /// Honoured by ordinary rows and by label rows alike, so a section heading or a click-only
    /// action row can carry a count without borrowing the checkbox column that would make it read
    /// as selectable state. On an ordinary row it also replaces the submenu chevron.
    ///
    /// Args:
    ///     right_label: Trailing text, typically a count or a shortcut.
    ///
    /// Returns:
    ///     The updated row.
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

    /// Override, for this row alone, whether clicking it closes the menu.
    ///
    /// `MoonDropdown::close_on_select` is a whole-menu policy, which is wrong for a menu holding
    /// two kinds of row: checkbox rows that must leave a multi-select menu standing, and a row
    /// that opens a dialog. The second kind MUST take the menu down — a popup is deferred above
    /// the dialog layer, so a menu left open paints over the modal it just opened, and the first
    /// click into that modal both dismisses the menu and pulls focus back out of the dialog.
    ///
    /// Without this, the only way to mix the two in one menu is for the consumer to take over the
    /// dropdown's open state entirely, which costs it a mirrored flag and a retained callback in
    /// every hosting view.
    ///
    /// Args:
    ///     closes_menu: Whether a click on this row closes the menu, overriding the dropdown.
    ///
    /// Returns:
    ///     The row carrying its own close policy.
    pub fn closes_menu(mut self, closes_menu: bool) -> Self {
        self.closes_menu = Some(closes_menu);
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
    headers: Vec<(f32, AnyElement)>,
    items: std::rc::Rc<Vec<MoonMenuItem>>,
    layout: MenuLayoutFingerprint,
    size: MoonMenuSize,
    width: MoonMenuWidth,
    rendered_max_width: Option<f32>,
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
            rendered_max_width: None,
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

    /// Pin an element above the rows, outside the scrolling region.
    ///
    /// Repeatable; each header carries its own height. A header does not scroll, so the space it
    /// takes comes out of the row list's budget, and the height travels with the element rather
    /// than as a separate builder call — the row list is sized in pixels before its siblings are
    /// laid out, so it cannot discover the header on its own, and a header whose height was never
    /// declared would be pinned to zero and vanish.
    ///
    /// The height is a design-reference value. It is UI-scaled at render and initially floored by
    /// the menu's row height so text survives font growth; a configured outer cap may then shrink
    /// the wrapper before reducing the list below its one-row floor.
    ///
    /// Args:
    ///     height_ui: Header height at the configured UI reference scale.
    ///     header: Element rendered above the rows.
    ///
    /// Returns:
    ///     The updated menu.
    pub fn header(mut self, height_ui: f32, header: impl IntoElement) -> Self {
        self.headers.push((height_ui, header.into_any_element()));
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
    pub(super) fn width_policy(mut self, width: MoonMenuWidth) -> Self {
        self.width = width;
        self
    }

    /// Cap measured widths to an already-rendered viewport budget.
    ///
    /// Args:
    ///     max_width: Maximum outer width in rendered pixels.
    ///
    /// Returns:
    ///     The updated menu.
    pub(super) fn rendered_max_width(mut self, max_width: f32) -> Self {
        self.rendered_max_width = Some(max_width);
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
        // The capped eager row list needs an identity distinct from the outer menu to retain its
        // scroll offset. Clone it before `self` is consumed by the branch-specific row rendering.
        let id_for_rows = self.id.clone();
        let mono = self.mono;
        let virtualized = virtual_list_state.is_some();

        // Resolve this before measurement because pinned headers reduce both the row list's height
        // and the initial-row budget used to fit a virtualized menu's width. Each header also costs
        // one gap because the outer flex column separates every child.
        let menu_gap = tokens.ui(MENU_GAP);
        let resolved_outer_max = resolve_menu_outer_max(self.max_height, &tokens, virtualized);
        let requested_heights: Vec<f32> = self
            .headers
            .iter()
            .map(|(height_ui, _)| tokens.ui(*height_ui).max(metrics.row_height))
            .collect();
        let header_gaps = menu_gap * requested_heights.len() as f32;
        let requested_total = requested_heights.iter().sum::<f32>();
        let header_budget = clamp_header_budget(
            requested_total + header_gaps,
            resolved_outer_max,
            menu_outer_chrome(&tokens),
            metrics.row_height,
        );
        // The clamp only affects layout if the elements follow it. Spending the surviving budget
        // back onto the wrappers is what makes the header yield, as `clamp_header_budget` promises;
        // pinning them to the requested heights instead would let header plus list overrun the
        // menu's maximum and the outer `overflow_hidden` would clip the rows the clamp just saved.
        let header_heights: Vec<f32> = if requested_total > 0.0 {
            let scale = ((header_budget - header_gaps).max(0.0) / requested_total).min(1.0);
            requested_heights
                .iter()
                .map(|height| height * scale)
                .collect()
        } else {
            requested_heights
        };
        let content_max = menu_content_max(resolved_outer_max, &tokens, header_budget, metrics);

        let (mut width, mut truncate_labels) = if let Some(cx) = cx {
            if virtualized {
                resolve_virtual_menu_width(
                    self.width,
                    &self.items,
                    metrics,
                    &tokens,
                    cx,
                    mono,
                    content_max,
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
        if self.width.is_measured()
            && let Some(max_width) = self.rendered_max_width
            && width > max_width
        {
            width = max_width.max(1.0);
            truncate_labels = true;
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
            .gap(px(tokens.ui(MENU_GAP)));

        // The cap belongs to the whole menu, but the scroll belongs to the rows alone: a header
        // that scrolled with them would leave the menu the moment the list is longer than the cap,
        // which is exactly the case a header exists for.
        let capped = self.max_height.is_some();
        if virtual_list_state.is_none() && capped {
            menu = menu.max_h(px(resolved_outer_max)).overflow_hidden();
        }

        for ((_, header), height) in self.headers.into_iter().zip(header_heights) {
            // Pinned to exactly the height its budget reserved, so the declaration cannot drift
            // from the layout in either direction: an over-declared header would otherwise shrink
            // the row list without using the space, and an under-declared one would push the list
            // past the menu's maximum.
            menu = menu.child(
                div()
                    .flex_none()
                    .h(px(height))
                    .overflow_hidden()
                    .child(header),
            );
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
            let list_height =
                virtual_menu_list_height(&self.items, metrics, &row_context.tokens, content_max);
            let item_count = self.items.len();
            let items = self.items;
            let row_context = row_context.clone();
            menu = menu.max_h(px(resolved_outer_max)).overflow_hidden().child(
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
            // Only a capped menu needs a scrolling container of its own; an uncapped one keeps its
            // natural height and its rows stay direct children, with no extra layout node and no
            // retained element state to key.
            if capped {
                let mut rows = div()
                    .id(ElementId::from(SharedString::from(format!(
                        "{}:rows",
                        id_for_rows
                    ))))
                    .flex()
                    .flex_col()
                    .gap(px(menu_gap))
                    .min_h_0()
                    .flex_1()
                    .overflow_y_scroll();
                for (ix, item) in items.into_iter().enumerate() {
                    rows = rows.child(Self::render_item(&row_context, ix, item, cx));
                }
                menu = menu.child(rows);
            } else {
                for (ix, item) in items.into_iter().enumerate() {
                    menu = menu.child(Self::render_item(&row_context, ix, item, cx));
                }
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
                    .justify_between()
                    .gap(px(metrics.gap))
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

                // A label row carries the same trailing count an ordinary row can. The count sits
                // at the row's own muted alpha rather than taking the ordinary row's further
                // opacity reduction: the whole row is already secondary chrome, and dimming it
                // again would push it under the rows it is meant to be read against.
                //
                // `justify_between` plus the row's own gap, rather than a flexible spacer: that
                // renders exactly the one gap the width measurement charges for this row kind.
                if let Some(right_label) = item.right_label {
                    row = row.child(menu_trailing_label(right_label, p, metrics, mono, 0.88));
                }

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
                    row = row.child(menu_trailing_label(
                        right_label,
                        p,
                        metrics,
                        mono,
                        alpha * 0.88,
                    ));
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
        unscaled_menu_metrics(self.size)
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
    menu_header: Option<std::rc::Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>>,
    menu_header_height: f32,
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
            menu_header: None,
            menu_header_height: 0.0,
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

    /// Pin an element above the menu's rows that does not scroll with them.
    ///
    /// The builder takes a closure rather than a built element because the popup's content is
    /// rebuilt on every render: a stored `AnyElement` could be consumed only once. The closure
    /// receives the window, so a header may own focusable content such as a search field.
    ///
    /// When a maximum height is configured, the header is laid out inside that limit rather than
    /// added on top of it: the row list gives up the space the header takes. The height is declared
    /// rather than measured because the row list is sized in pixels before its siblings are laid
    /// out; the virtualized width-fitting prefix reads the same budget, so an inaccurate
    /// declaration also mis-measures the menu's width.
    ///
    /// Calling this twice replaces the header, unlike [`MoonPopupMenu::header`], which appends.
    ///
    /// **The closure is retained for as long as the popup state is** — it is cloned into the
    /// popover's content closure, which outlives the frame. Capture weak handles and cheap values
    /// only: a strong view handle closes a `view -> popup state -> closure -> view` cycle and the
    /// view never drops, the same hazard the tree and virtual-list row builders carry.
    ///
    /// Args:
    ///     height_ui: Header height at the configured UI reference scale.
    ///     header: Builder invoked once per popup render.
    ///
    /// Returns:
    ///     The updated dropdown.
    pub fn header(
        mut self,
        height_ui: f32,
        header: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        self.menu_header = Some(std::rc::Rc::new(header));
        self.menu_header_height = height_ui;
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
        let menu_header = self.menu_header.clone();
        let menu_header_height = self.menu_header_height;
        let popup_level = MoonMenuLevel::from_parts(std::rc::Rc::new(items), menu_layout);
        let dropdown_selection = std::rc::Rc::new(MoonDropdownSelectionContext::new(
            self.close_on_select,
            self.on_select,
            state.clone(),
            controlled_open,
            self.on_open_change.clone(),
            parent_view,
        ));

        let mut popover = CorePopover::new(ElementId::from(self.id.clone()))
            .appearance(false)
            .anchor(Anchor::TopLeft)
            .deferred_priority(30_000)
            .open(open)
            .trigger_any(trigger)
            .content(move |_, window, cx| {
                let tokens = MoonTheme::active_tokens(cx);
                let mut menu = MoonPopupMenu::new(menu_id.clone())
                    .shared_level(popup_level.clone())
                    .dropdown_selection(dropdown_selection.clone())
                    .size(menu_size)
                    .width_policy(menu_width)
                    .mono(true);
                if menu_width.is_measured() {
                    let viewport = window.viewport_size();
                    let viewport_max_width = (f32::from(viewport.width) - 16.0).max(1.0);
                    // The deferred anchor fits the content wrapper, and this menu sits below an
                    // in-wrapper top offset. Pay for that offset here or a nominal viewport-height
                    // menu still extends past the bottom by exactly the trigger compensation plus
                    // gap used below.
                    let popover_top_offset = trigger_height + tokens.ui(menu_offset_y);
                    let viewport_max_height =
                        (f32::from(viewport.height) - 16.0 - popover_top_offset).max(80.0);
                    let resolved_max_height = menu_max_height
                        .map(|height| height.resolve(&tokens))
                        .unwrap_or(viewport_max_height)
                        .min(viewport_max_height);
                    menu = menu
                        .rendered_max_width(viewport_max_width)
                        .max_height(resolved_max_height);
                } else if let Some(max_height) = menu_max_height {
                    menu = menu.max_height_policy(max_height);
                }
                // Built here rather than stored: this closure runs on every popup render, so a
                // retained element would be consumable only once.
                if let Some(header) = menu_header.as_ref() {
                    menu = menu.header(menu_header_height, header(window, cx));
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
