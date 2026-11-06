//! Private selection and click-dispatch policy for dropdown menu rows.

use crate::moon::foundation::{MoonClickHandler, MoonSelectHandler};
use crate::popover::PopoverState;
use gpui::Entity;

use super::{MoonMenuItem, MoonMenuItemKind};

#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering};

/// Prefix used to count lazily resolved dropdown handlers in regression tests.
#[cfg(test)]
const MENU_DROPDOWN_HANDLER_PROBE_PREFIX: &str = "moon-menu-handler-probe-";
/// Number of probe handlers resolved since the previous reset.
#[cfg(test)]
static MENU_DROPDOWN_HANDLER_PROBE_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Return whether a menu row kind is allowed to dispatch click handlers.
///
/// Args:
///     kind: Visual role of the menu row.
///     disabled: Whether interaction is disabled for the row.
///     label_actionable: Whether a label explicitly opted into action behavior.
///
/// Returns:
///     `true` for enabled items and explicitly actionable enabled labels.
pub(super) fn moon_menu_item_accepts_click(
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

/// Shared selection callbacks and the actual popover whose focus must close before dispatch.
pub(super) struct MoonDropdownSelectionContext {
    close_on_select: bool,
    on_select: Option<MoonSelectHandler>,
    popover: Entity<PopoverState>,
}

impl MoonDropdownSelectionContext {
    /// Retain the visible menu's callbacks and lifecycle owner for lazy row handlers.
    ///
    /// `close_on_select` supplies the default when a row has no override; `on_select` receives its
    /// key after dismissal. `popover` owns focus restoration and controlled-state notifications.
    /// Returns a context shared by the rows of this visible menu.
    pub(super) fn new(
        close_on_select: bool,
        on_select: Option<MoonSelectHandler>,
        popover: Entity<PopoverState>,
    ) -> Self {
        Self {
            close_on_select,
            on_select,
            popover,
        }
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
pub(super) fn menu_item_click_handler(
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
    let closes_menu = item.closes_menu;
    Some(std::rc::Rc::new(move |event, window, cx| {
        // Restore the trigger's focus BEFORE a callback opens a modal/native dialog. Root then
        // remembers a live return target rather than the menu handle this click is removing.
        if closes_menu.unwrap_or(dropdown.close_on_select) {
            dropdown
                .popover
                .update(cx, |state, cx| state.dismiss(window, cx));
        }
        if let Some(existing_handler) = existing_handler.as_ref() {
            existing_handler(event, window, cx);
        }
        if let Some(on_select) = dropdown.on_select.as_ref() {
            on_select(&key, window, cx);
        }
    }))
}

/// Reset and return the visible-row dropdown-handler probe.
///
/// Returns:
///     Number of probe handlers resolved since the previous reset.
#[cfg(test)]
fn take_dropdown_handler_probe_count() -> usize {
    MENU_DROPDOWN_HANDLER_PROBE_COUNT.swap(0, Ordering::Relaxed)
}

#[cfg(test)]
mod tests;
