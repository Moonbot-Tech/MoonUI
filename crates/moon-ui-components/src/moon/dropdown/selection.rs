//! Private selection and click-dispatch policy for dropdown menu rows.

use crate::moon::foundation::{MoonClickHandler, MoonSelectHandler};
use gpui::{App, Entity, EntityId, Window};

use super::{MoonDropdownState, MoonMenuItem, MoonMenuItemKind};

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

/// Planned menu-close effects for one resolved selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MoonDropdownSelectPlan {
    close_menu: bool,
    update_internal_open: bool,
}

/// Shared dropdown behavior applied lazily only when a menu row is rendered.
pub(super) struct MoonDropdownSelectionContext {
    close_on_select: bool,
    on_select: Option<MoonSelectHandler>,
    state: Entity<MoonDropdownState>,
    controlled_open: Option<bool>,
    on_open_change: Option<std::rc::Rc<dyn Fn(bool, &mut Window, &mut App)>>,
    parent_view: EntityId,
}

impl MoonDropdownSelectionContext {
    /// Retain dropdown-level selection behavior for lazily rendered rows.
    ///
    /// Args:
    ///     close_on_select: Whole-menu close policy.
    ///     on_select: Optional callback receiving the selected row key.
    ///     state: Keyed internal open state for an uncontrolled dropdown.
    ///     controlled_open: Caller-owned open state, when present.
    ///     on_open_change: Optional callback receiving requested open-state changes.
    ///     parent_view: View notified after internal state changes.
    ///
    /// Returns:
    ///     A retained selection context shared by visible row handlers.
    pub(super) fn new(
        close_on_select: bool,
        on_select: Option<MoonSelectHandler>,
        state: Entity<MoonDropdownState>,
        controlled_open: Option<bool>,
        on_open_change: Option<std::rc::Rc<dyn Fn(bool, &mut Window, &mut App)>>,
        parent_view: EntityId,
    ) -> Self {
        Self {
            close_on_select,
            on_select,
            state,
            controlled_open,
            on_open_change,
            parent_view,
        }
    }
}

/// Decide what one row's click does to the menu around it.
///
/// `close_on_select` is a whole-menu policy, but a single menu can legitimately hold both kinds of
/// row: checkbox rows that must leave a multi-select menu standing, and a row that opens a dialog
/// and must also take the menu down during that click — an open popup is deferred ABOVE the dialog
/// layer, so a menu left standing paints over the modal it just opened. `item_closes_menu` is that
/// row's own answer, and it wins; `None` follows the menu.
///
/// Args:
///     close_on_select: The dropdown's whole-menu policy.
///     item_closes_menu: The clicked row's override, if it declared one.
///     controlled_open: `Some` while the consumer owns the open state.
///
/// Returns:
///     Whether to close, and whether this dropdown owns the state that records it.
fn moon_dropdown_select_plan(
    close_on_select: bool,
    item_closes_menu: Option<bool>,
    controlled_open: Option<bool>,
) -> MoonDropdownSelectPlan {
    let close_menu = item_closes_menu.unwrap_or(close_on_select);
    MoonDropdownSelectPlan {
        close_menu,
        update_internal_open: close_menu && controlled_open.is_none(),
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
        let plan = moon_dropdown_select_plan(
            dropdown.close_on_select,
            closes_menu,
            dropdown.controlled_open,
        );
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
