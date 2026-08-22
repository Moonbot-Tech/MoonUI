//! Regression coverage for dropdown selection and lazy handler dispatch.

use super::super::{MoonDropdown, MoonMenuItem, MoonMenuItemKind};
use super::{
    MENU_DROPDOWN_HANDLER_PROBE_PREFIX, MoonDropdownSelectPlan, moon_dropdown_select_plan,
    moon_menu_item_accepts_click, take_dropdown_handler_probe_count,
};
use crate::moon::{MoonPalette, MoonTheme};
use std::{cell::RefCell, rc::Rc};

/// `dropdown.rs:MoonMenuItem::action_label` must retain the label visual role while explicitly
/// opting into clicks. Returning a normal item would reintroduce item typography and its check
/// gutter; treating every enabled label as actionable would make static section headings
/// interactive after `.disabled(false)`.
#[test]
fn menu_item_clickability_respects_kind_and_disabled_state() {
    let action_label = MoonMenuItem::action_label("action", "Action label");
    assert_eq!(action_label.kind, MoonMenuItemKind::Label);
    assert!(!action_label.disabled);
    assert!(action_label.actionable);

    assert!(moon_menu_item_accepts_click(
        MoonMenuItemKind::Item,
        false,
        false
    ));
    assert!(!moon_menu_item_accepts_click(
        MoonMenuItemKind::Item,
        true,
        true
    ));
    assert!(moon_menu_item_accepts_click(
        MoonMenuItemKind::Label,
        false,
        true
    ));
    assert!(!moon_menu_item_accepts_click(
        MoonMenuItemKind::Label,
        false,
        false
    ));
    assert!(!moon_menu_item_accepts_click(
        MoonMenuItemKind::Label,
        true,
        true
    ));
    assert!(!moon_menu_item_accepts_click(
        MoonMenuItemKind::Separator,
        false,
        true
    ));
}

/// Open dropdown containing enabled and static label rows for native interaction checks.
struct LabelInteractionHarness {
    action_clicks: Rc<RefCell<usize>>,
    static_clicks: Rc<RefCell<usize>>,
    selection_keys: Rc<RefCell<Vec<String>>>,
}

impl gpui::Render for LabelInteractionHarness {
    /// Render one action label and one static label in an open dropdown.
    ///
    /// Args:
    ///     _window: Test window receiving pointer events.
    ///     _cx: Test view context.
    ///
    /// Returns:
    ///     The rendered open dropdown.
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        let action_clicks = self.action_clicks.clone();
        let static_clicks = self.static_clicks.clone();
        let selection_keys = self.selection_keys.clone();
        MoonDropdown::new("label-interaction")
            .label("trigger")
            .default_open(true)
            .close_on_select(false)
            .item(
                MoonMenuItem::action_label("action", "Action label").on_click(move |_, _, _| {
                    *action_clicks.borrow_mut() += 1;
                }),
            )
            .item(
                MoonMenuItem::label("Static label")
                    .disabled(false)
                    .on_click(move |_, _, _| {
                        *static_clicks.borrow_mut() += 1;
                    }),
            )
            .on_select(move |key, _, _| {
                selection_keys.borrow_mut().push(key.to_string());
            })
    }
}

/// Render and click both label row states under one palette.
///
/// Args:
///     cx: GPUI test application context.
///     palette: Palette used to render the dropdown.
///
/// Returns:
///     Nothing; callback counts are asserted after simulated clicks.
fn assert_rendered_label_interactions(cx: &mut gpui::TestAppContext, palette: MoonPalette) {
    cx.update(|cx| {
        MoonTheme::global_mut(cx).palette = palette;
    });
    let action_clicks = Rc::new(RefCell::new(0));
    let static_clicks = Rc::new(RefCell::new(0));
    let selection_keys = Rc::new(RefCell::new(Vec::new()));
    let window = cx.add_window({
        let action_clicks = action_clicks.clone();
        let static_clicks = static_clicks.clone();
        let selection_keys = selection_keys.clone();
        move |_, _| LabelInteractionHarness {
            action_clicks,
            static_clicks,
            selection_keys,
        }
    });
    let mut cx = gpui::VisualTestContext::from_window(window.into(), cx);
    let (action, static_label) = (0..8)
        .find_map(|_| {
            cx.update(|window, _| window.refresh());
            cx.run_until_parked();
            Some((
                cx.debug_bounds("label-interaction:menu:item:0")?,
                cx.debug_bounds("label-interaction:menu:item:1")?,
            ))
        })
        .expect("both label rows must register rendered bounds");

    cx.simulate_click(action.center(), gpui::Modifiers::default());
    assert_eq!(
        *action_clicks.borrow(),
        1,
        "enabled action label must dispatch its callback"
    );
    assert_eq!(
        selection_keys.borrow().as_slice(),
        ["action"],
        "visible action label must dispatch the dropdown-level selection key"
    );

    cx.simulate_click(static_label.center(), gpui::Modifiers::default());
    assert_eq!(
        *static_clicks.borrow(),
        0,
        "ordinary label must remain inert even when a handler is attached"
    );
    assert_eq!(
        selection_keys.borrow().as_slice(),
        ["action"],
        "static labels must not dispatch dropdown-level selection"
    );
}

/// `dropdown.rs:MoonPopupMenu::render_item` must wire enabled label rows into GPUI's native click
/// path. Removing the label click branch leaves visually correct exchange headings that no longer
/// toggle their grouped menu items.
#[gpui::test]
fn rendered_action_label_dispatches_while_static_label_stays_inert(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init);
    for palette in [MoonPalette::TERMINAL, MoonPalette::LIGHT] {
        assert_rendered_label_interactions(cx, palette);
    }
}

/// Catches closing keep-open rows or mutating controlled state in
/// `selection.rs:moon_dropdown_select_plan`, which would dismiss persistent menus or desynchronize
/// a controlled dropdown from its owner.
#[test]
fn dropdown_select_plan_respects_close_and_controlled_state() {
    assert_eq!(
        moon_dropdown_select_plan(true, None, None),
        MoonDropdownSelectPlan {
            close_menu: true,
            update_internal_open: true,
        }
    );
    assert_eq!(
        moon_dropdown_select_plan(true, None, Some(true)),
        MoonDropdownSelectPlan {
            close_menu: true,
            update_internal_open: false,
        }
    );
    assert_eq!(
        moon_dropdown_select_plan(false, None, None),
        MoonDropdownSelectPlan {
            close_menu: false,
            update_internal_open: false,
        }
    );
}

/// A row's own `closes_menu` override must win over the dropdown's whole-menu `close_on_select`
/// policy in `selection.rs:moon_dropdown_select_plan`, and `update_internal_open` must follow the
/// RESOLVED `close_menu` rather than the dropdown's raw policy. Folding the override back down to
/// `close_on_select` leaves a dialog-opening row unable to force its host menu shut, so the menu
/// paints over the modal it just opened; deriving `update_internal_open` from `close_on_select`
/// instead of `close_menu` leaves an uncontrolled dropdown's internal `open` flag out of sync with
/// a row-forced close, so the menu re-opens on the next repaint.
#[test]
fn dropdown_select_plan_row_override_wins_over_dropdown_policy() {
    // Row forces a close on a dropdown configured to stay open (checkbox-menu policy).
    assert_eq!(
        moon_dropdown_select_plan(false, Some(true), None),
        MoonDropdownSelectPlan {
            close_menu: true,
            update_internal_open: true,
        },
        "closes_menu(true) must close a close_on_select(false) dropdown and clear its internal open state"
    );
    // Row forces staying open on a dropdown configured to close.
    assert_eq!(
        moon_dropdown_select_plan(true, Some(false), None),
        MoonDropdownSelectPlan {
            close_menu: false,
            update_internal_open: false,
        },
        "closes_menu(false) must keep a close_on_select(true) dropdown open and leave its internal open state untouched"
    );
}

/// Open a large dropdown whose per-row dropdown handlers are counted when resolved.
struct LargeHandlerProbeMenuHarness;

impl gpui::Render for LargeHandlerProbeMenuHarness {
    /// Render enough handler-probe rows to require virtualization.
    ///
    /// Args:
    ///     _window: Test window.
    ///     _cx: Test view context.
    ///
    /// Returns:
    ///     An open dropdown with a root selection handler.
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        MoonDropdown::new("large-handler-probe")
            .label("trigger")
            .default_open(true)
            .menu_max_height(180.0)
            .items((0..1_000).map(|ix| {
                MoonMenuItem::new(format!("{MENU_DROPDOWN_HANDLER_PROBE_PREFIX}{ix:04}"))
            }))
            .on_select(|_, _, _| {})
    }
}

/// Catches restoring eager root-row wiring in `dropdown.rs:MoonDropdown::render`. That edit
/// allocates one dropdown click closure per dynamic row on every repaint before virtualization.
#[gpui::test]
fn large_dropdown_resolves_handlers_only_for_visible_rows(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init);
    let window = cx.add_window(|_, _| LargeHandlerProbeMenuHarness);
    let mut cx = gpui::VisualTestContext::from_window(window.into(), cx);

    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
    _ = take_dropdown_handler_probe_count();

    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
    let repaint_handlers = take_dropdown_handler_probe_count();

    assert!(
        repaint_handlers < 500,
        "virtual repaint resolved {repaint_handlers} handlers for a 1,000-row root level"
    );
}
