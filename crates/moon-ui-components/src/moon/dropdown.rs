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
mod model;
mod popup;
mod selection;
mod trigger;

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
use model::MenuLayoutFingerprint;
pub(super) use model::MoonMenuLevel;
#[cfg(test)]
pub(super) use model::take_menu_item_clone_probe_count;
pub use model::{MoonMenuItem, MoonMenuItemKind, MoonMenuSize};
pub use popup::MoonPopupMenu;
#[cfg(test)]
use popup::take_palette_probe_shell;
use selection::{
    MoonDropdownSelectionContext, menu_item_click_handler, moon_menu_item_accepts_click,
};
pub use trigger::MoonDropdown;
use trigger::MoonDropdownState;
#[cfg(test)]
use trigger::fit_dropdown_trigger_label;

#[cfg(test)]
mod tests;
