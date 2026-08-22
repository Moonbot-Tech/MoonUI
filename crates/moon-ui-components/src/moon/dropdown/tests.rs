//! Regression coverage for dropdown placement, sizing, selection, and submenu behavior.

use super::button_leading_icon_reservation;
use super::{
    DROPDOWN_CARET, DROPDOWN_TRIGGER_PAD_X, MENU_CLONE_PROBE_PREFIX,
    MENU_DROPDOWN_HANDLER_PROBE_PREFIX, MENU_MEASUREMENT_PROBE_PREFIX, MENU_PADDING,
    MENU_PALETTE_PROBE_PREFIX, MENU_WIDTH_SAMPLE_ROWS, MenuMetrics, MoonButtonIconSlot,
    MoonButtonSize, MoonDropdown, MoonDropdownSelectPlan, MoonDropdownTriggerWidth, MoonMenuItem,
    MoonMenuItemKind, MoonMenuMaxHeight, MoonMenuSize, MoonMenuWidth, MoonPalette, MoonPopupMenu,
    MoonRect, MoonThemeTokens, SUBMENU_OFFSET_X, capped_menu_items_height, clamp_header_budget,
    fit_dropdown_trigger_label, fit_menu_item_labels, menu_content_max,
    menu_measurement_probe_count, moon_dropdown_select_plan, moon_menu_item_accepts_click,
    natural_menu_width, resolve_menu_width, resolve_virtual_menu_width,
    take_dropdown_handler_probe_count, take_menu_item_clone_probe_count, take_palette_probe_shell,
};
use crate::moon::{MoonScale, MoonTheme, ThemeMode};
use gpui::{ParentElement as _, Styled as _};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

/// Stable element id used by dropdown geometry probes.
const DROPDOWN_ID: &str = "geometry-probe";
/// `MoonDropdown::render` names its menu `{id}:menu`, and `MoonPopupMenu` registers that id as its
/// debug selector.
const MENU_SELECTOR: &str = "geometry-probe:menu";
/// Debug selector for the fitted dropdown opened against both viewport edges.
const FITTED_VIEWPORT_MENU_SELECTOR: &str = "fitted-viewport:menu";

/// Catches removing the incremental `layout.push` call from
/// `dropdown.rs:MoonPopupMenu::{item,items}`. Without the ordered signature, retained virtual
/// state cannot invalidate cached variable heights when a same-length menu changes row roles.
#[test]
fn menu_layout_fingerprint_tracks_count_and_row_order() {
    let item_then_label = MoonPopupMenu::new("layout-a")
        .item(MoonMenuItem::new("item"))
        .item(MoonMenuItem::label("label"));
    let label_then_item = MoonPopupMenu::new("layout-b")
        .items([MoonMenuItem::label("label"), MoonMenuItem::new("item")]);

    assert_eq!(item_then_label.layout.item_count, 2);
    assert_eq!(label_then_item.layout.item_count, 2);
    assert_ne!(
        item_then_label.layout, label_then_item.layout,
        "same-length menus with different row order need distinct retained layouts"
    );
}

/// Catches moving the viewport cap in `dropdown.rs:capped_menu_items_height` after the iterator
/// traversal. That edit restores an O(total rows) repaint scan even though the viewport is already
/// full after two ordinary rows.
#[test]
fn virtual_menu_height_stops_scanning_once_the_viewport_is_full() {
    let tokens = MoonThemeTokens::default();
    let metrics = MenuMetrics {
        row_height: 24.0,
        font_size: 10.5,
        line_height: 13.0,
        radius: 4.0,
        pad_x: 7.0,
        gap: 6.0,
    };
    let inspected = Cell::new(0);
    let kinds = (0..1_000).map(|_| {
        inspected.set(inspected.get() + 1);
        MoonMenuItemKind::Item
    });
    let cap = metrics.row_height * 2.0 + tokens.ui(super::MENU_GAP);

    assert_eq!(capped_menu_items_height(kinds, metrics, &tokens, cap), cap);
    assert_eq!(
        inspected.get(),
        2,
        "height calculation must stop after filling the bounded viewport"
    );
}

/// Catches changing `MoonMenuItem::submenu` back to an owned `Vec`. Deep-cloning a menu row would
/// then copy every nested descendant whenever the dropdown popover or one visible virtual row is
/// rebuilt.
#[test]
fn cloned_menu_items_share_immutable_submenu_storage() {
    let item = MoonMenuItem::new("More")
        .submenu([MoonMenuItem::new("Nested 1"), MoonMenuItem::new("Nested 2")]);
    let cloned = item.clone();

    assert!(
        Rc::ptr_eq(&item.submenu.items, &cloned.submenu.items),
        "cloning one row must retain the immutable submenu allocation"
    );
}

/// Root view holding one open `MoonDropdown` and recording the laid-out trigger bounds.
///
/// The menu is not among the wrapper's direct children: it renders into a
/// `deferred(anchored(..))` layer and is reached through `VisualTestContext::debug_bounds`.
/// `trigger_rect` selects the parent's in-flow path or the caller-supplied absolute-bounds path.
struct DropdownHarness {
    trigger_bounds: Rc<RefCell<Vec<gpui::Bounds<gpui::Pixels>>>>,
    trigger_rect: Option<MoonRect>,
}

/// Root view holding a long fitted dropdown at the viewport's bottom-right corner.
struct FittedViewportDropdownHarness;

impl gpui::Render for FittedViewportDropdownHarness {
    /// Render enough long rows to require both width fitting and a viewport-height cap.
    ///
    /// Args:
    ///     window: Test window whose current viewport supplies the edge placement.
    ///     _cx: View context unused by the stateless harness.
    ///
    /// Returns:
    ///     An absolutely positioned open dropdown.
    fn render(
        &mut self,
        window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        let viewport = window.viewport_size();
        let items = (0..48).map(|index| {
            MoonMenuItem::new(format!(
                "Only current market orders with a deliberately long localized label {index}"
            ))
        });
        MoonDropdown::new("fitted-viewport")
            .bounds(MoonRect::new(
                f32::from(viewport.width) - 24.0,
                f32::from(viewport.height) - 24.0,
                20.0,
                20.0,
            ))
            .default_open(true)
            .fit_menu_width(220.0, 560.0)
            .items(items)
    }
}

impl gpui::Render for DropdownHarness {
    /// Render the dropdown and record the trigger bounds after layout.
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        use gpui::{ParentElement as _, Styled as _};

        let sink = self.trigger_bounds.clone();
        let mut dropdown = MoonDropdown::new(DROPDOWN_ID)
            .label("trigger")
            // Use a fixed non-default size so the measured trigger box is known.
            .trigger_size(MoonButtonSize::Action)
            .default_open(true)
            .item(MoonMenuItem::new("only item"));
        if let Some(rect) = self.trigger_rect {
            dropdown = dropdown.bounds(rect);
        }
        // Start-align both axes so the dropdown shrink-wraps its trigger instead of stretching.
        gpui::div()
            .flex()
            .flex_row()
            .items_start()
            .justify_start()
            .on_children_prepainted(move |bounds, _, _| *sink.borrow_mut() = bounds)
            .child(dropdown)
    }
}

/// Open a dropdown in a fresh test window and return its trigger and menu boxes.
fn open_and_measure(
    cx: &mut gpui::TestAppContext,
    trigger_rect: Option<MoonRect>,
) -> (gpui::Bounds<gpui::Pixels>, gpui::Bounds<gpui::Pixels>) {
    cx.update(crate::init);
    let bounds = Rc::new(RefCell::new(Vec::new()));
    let sink = bounds.clone();
    let window = cx.add_window(move |_, _| DropdownHarness {
        trigger_bounds: sink,
        trigger_rect,
    });
    let mut cx = gpui::VisualTestContext::from_window(window.into(), cx);

    // Popover captures its trigger bounds on the first layout and requests a fresh frame.
    let menu = (0..8)
        .find_map(|_| {
            cx.update(|window, _| window.refresh());
            cx.run_until_parked();
            cx.debug_bounds(MENU_SELECTOR)
        })
        .expect(
            "open dropdown must render its menu; if `MoonDropdown::render` no longer names it \
             `{id}:menu`, MENU_SELECTOR is what went stale",
        );

    let recorded = bounds.borrow();
    assert_eq!(
        recorded.len(),
        1,
        "expected exactly the dropdown as a child"
    );
    (recorded[0], menu)
}

/// Root view that renders one closed dropdown and records its laid-out trigger bounds.
struct ClosedDropdownHarness {
    build: Box<dyn Fn() -> MoonDropdown>,
    bounds: Rc<RefCell<Vec<gpui::Bounds<gpui::Pixels>>>>,
}

impl gpui::Render for ClosedDropdownHarness {
    /// Render the configured dropdown and record its final child bounds.
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        use gpui::{ParentElement as _, Styled as _};

        let sink = self.bounds.clone();
        gpui::div()
            .w(gpui::px(200.0))
            .flex()
            .flex_row()
            .items_start()
            .justify_start()
            .on_children_prepainted(move |bounds, _, _| *sink.borrow_mut() = bounds)
            .child((self.build)())
    }
}

/// Lay a closed dropdown out in a real window and return its trigger box.
fn laid_out_trigger_bounds(
    cx: &mut gpui::TestAppContext,
    build: impl Fn() -> MoonDropdown + 'static,
) -> gpui::Bounds<gpui::Pixels> {
    use gpui::AppContext as _;

    cx.update(crate::init);
    let bounds = Rc::new(RefCell::new(Vec::new()));
    let sink = bounds.clone();
    let window = cx.add_window(move |_, _| ClosedDropdownHarness {
        build: Box::new(build),
        bounds: sink,
    });
    cx.update_window(window.into(), |_, window, cx| {
        window.draw(cx).clear();
    })
    .unwrap();

    let out = bounds.borrow();
    assert_eq!(out.len(), 1, "expected exactly the dropdown as a child");
    out[0]
}

/// Catches removing the `MoonButton::leading_icon` forwarding call from
/// `dropdown.rs:MoonDropdown::render_trigger`. That edit would silently hide configured trigger
/// icons while leaving labelled dropdowns otherwise functional.
#[gpui::test]
fn labelled_trigger_icon_adds_width_in_both_palettes(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init);
    for palette in [MoonPalette::TERMINAL, MoonPalette::LIGHT] {
        cx.update(|cx| MoonTheme::global_mut(cx).palette = palette);
        let plain = laid_out_trigger_bounds(cx, || {
            MoonDropdown::new("plain-labelled-trigger")
                .label("Settings")
                .trigger_size(MoonButtonSize::Action)
        });
        let with_icon = laid_out_trigger_bounds(cx, || {
            MoonDropdown::new("icon-labelled-trigger")
                .label("Settings")
                .trigger_size(MoonButtonSize::Action)
                .trigger_leading_icon(MoonButtonIconSlot::new("icons/settings.svg"))
        });

        assert!(
            with_icon.size.width > plain.size.width,
            "configured icon did not widen the labelled trigger in {palette:?}: plain {:?}, icon {:?}",
            plain.size,
            with_icon.size
        );
    }
}

/// Catches removing `reserved_content_width` from
/// `dropdown.rs:fit_dropdown_trigger_label`. That edit would fit a long translated label into the
/// icon's space and let the non-shrinking trigger content paint beyond its configured width.
#[gpui::test]
fn fitted_labelled_trigger_reserves_rendered_icon_chrome_in_both_palettes(
    cx: &mut gpui::TestAppContext,
) {
    cx.update(crate::init);
    for palette in [MoonPalette::TERMINAL, MoonPalette::LIGHT] {
        cx.update(|cx| MoonTheme::global_mut(cx).palette = palette);
        let plain = laid_out_trigger_bounds(cx, || {
            MoonDropdown::new("plain-icon-reservation-probe")
                .label("Settings")
                .trigger_size(MoonButtonSize::Action)
        });
        let with_icon = laid_out_trigger_bounds(cx, || {
            MoonDropdown::new("icon-reservation-probe")
                .label("Settings")
                .trigger_size(MoonButtonSize::Action)
                .trigger_icon("icons/settings.svg")
        });
        let rendered_icon_chrome = with_icon.size.width - plain.size.width;
        let tokens = MoonThemeTokens {
            palette,
            ..MoonThemeTokens::default()
        };
        let font_size = 10.5;
        let measure = |text: &str| text.chars().count() as f32 * 8.0;
        let reservation = button_leading_icon_reservation(MoonButtonSize::Action, &tokens);
        let (label, width) = fit_dropdown_trigger_label(
            "a deliberately long translated settings label",
            DROPDOWN_CARET,
            MoonDropdownTriggerWidth::Fit {
                min: 100.0,
                max: 100.0,
            },
            &tokens,
            font_size,
            reservation,
            measure,
        );
        let width = width.expect("fitted trigger must resolve a rendered width");
        let fitted_content_width =
            gpui::px(measure(label.as_ref()) + tokens.ui(DROPDOWN_TRIGGER_PAD_X))
                + rendered_icon_chrome;

        assert!(
            gpui::px(reservation + 0.01) >= rendered_icon_chrome,
            "reserved icon chrome is narrower than real layout in {palette:?}"
        );
        assert!(
            fitted_content_width <= gpui::px(width + 0.01),
            "fitted icon trigger content exceeds its width in {palette:?}: content {fitted_content_width:?}, width {width}"
        );
    }
}

/// Catches restoring the unconditional empty `MoonButton::label` call in
/// `dropdown.rs:MoonDropdown::render_trigger`. That edit would add text padding and a phantom gap,
/// turning toolbar icon dropdowns into off-center rectangular controls.
#[gpui::test]
fn icon_only_trigger_stays_square_in_both_palettes(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init);
    for palette in [MoonPalette::TERMINAL, MoonPalette::LIGHT] {
        cx.update(|cx| MoonTheme::global_mut(cx).palette = palette);
        let bounds = laid_out_trigger_bounds(cx, || {
            MoonDropdown::new("icon-only-trigger")
                .trigger_size(MoonButtonSize::Action)
                .trigger_icon("icons/settings.svg")
        });

        assert_eq!(
            bounds.size.width, bounds.size.height,
            "icon-only dropdown trigger is not square in {palette:?}: {:?}",
            bounds.size
        );
    }
}

/// Assert that the menu hangs in the narrow band immediately below its trigger.
fn assert_menu_hugs_trigger(trigger: gpui::Bounds<gpui::Pixels>, menu: gpui::Bounds<gpui::Pixels>) {
    let gap = menu.origin.y - trigger.bottom();
    assert!(
        gap >= gpui::px(0.0) && gap < trigger.size.height * 0.5,
        "menu must hang just below the trigger: gap {gap:?}, trigger height {:?}",
        trigger.size.height
    );
    assert_eq!(
        menu.origin.x, trigger.origin.x,
        "menu must stay left-aligned with its trigger"
    );
}

/// Catches adding a second trigger-height offset in `dropdown.rs:MoonDropdown::render`, which
/// would leave an in-flow menu visibly detached from its trigger.
///
/// This also guards the compensation for `ElementExt::on_prepaint`: if capture starts reporting
/// the host's true origin, the anchor moves to the trigger top and this test points at `.mt(...)`.
#[gpui::test]
fn open_menu_hangs_just_below_its_trigger(cx: &mut gpui::TestAppContext) {
    let (trigger, menu) = open_and_measure(cx, None);
    assert_menu_hugs_trigger(trigger, menu);
}

/// Catches removing conditional height compensation from
/// `dropdown.rs:MoonDropdown::render`, which would place a bounds-driven menu over or too far
/// below its trigger.
#[gpui::test]
fn supplied_bounds_menu_also_hangs_just_below_its_trigger(cx: &mut gpui::TestAppContext) {
    let (trigger, menu) = open_and_measure(cx, Some(MoonRect::new(40.0, 24.0, 120.0, 26.0)));
    assert_menu_hugs_trigger(trigger, menu);
}

/// Catches removing the viewport width/height caps from the fitted dropdown render path. The
/// settings menu would still compile, but long translated rows or a large independent scale would
/// reproduce the screenshot's clipping and let the row stack escape the window bottom.
#[gpui::test]
fn fitted_dropdown_stays_inside_both_viewport_edges_at_independent_scales(
    cx: &mut gpui::TestAppContext,
) {
    cx.update(crate::init);
    for (palette, scale) in [
        (
            MoonPalette::TERMINAL,
            MoonScale {
                ui: 0.9,
                font: 1.35,
                font_delta: 2.0,
            },
        ),
        (
            MoonPalette::LIGHT,
            MoonScale {
                ui: 1.35,
                font: 0.9,
                font_delta: 2.0,
            },
        ),
    ] {
        cx.update(|cx| {
            let theme = MoonTheme::global_mut(cx);
            theme.palette = palette;
            theme.scale = scale;
        });
        let window = cx.add_window(|_, _| FittedViewportDropdownHarness);
        let mut visual = gpui::VisualTestContext::from_window(window.into(), cx);
        let menu = (0..8)
            .find_map(|_| {
                visual.update(|window, _| window.refresh());
                visual.run_until_parked();
                visual.debug_bounds(FITTED_VIEWPORT_MENU_SELECTOR)
            })
            .expect("fitted viewport dropdown must render its deferred menu");
        let viewport = visual.update(|window, _| window.viewport_size());

        assert!(
            menu.size.width > gpui::px(220.0),
            "long fitted dropdown stayed at its old fixed width in {palette:?} at {scale:?}"
        );
        assert!(
            menu.origin.x >= gpui::px(0.0) && menu.right() <= viewport.width,
            "fitted dropdown escaped the horizontal viewport in {palette:?} at {scale:?}: {menu:?}"
        );
        assert!(
            menu.origin.y >= gpui::px(0.0) && menu.bottom() <= viewport.height,
            "fitted dropdown escaped the vertical viewport in {palette:?} at {scale:?}: {menu:?}"
        );
    }
}

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
/// `dropdown.rs:moon_dropdown_select_plan`, which would dismiss persistent menus or desynchronize
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
/// policy in `dropdown.rs:moon_dropdown_select_plan`, and `update_internal_open` must follow the
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

/// The manual `Clone for MoonMenuItem` impl in `dropdown.rs` is written field by field, so a
/// future field addition can silently drop `closes_menu` from the copy without a compiler
/// complaint. That would lose a row's close-policy override every time the menu level is cloned
/// during a repaint, which is every frame.
#[test]
fn menu_item_clone_preserves_closes_menu_override() {
    let forces_close = MoonMenuItem::new("dialog-row").closes_menu(true);
    assert_eq!(
        forces_close.clone().closes_menu,
        Some(true),
        "cloning a row that forces the menu closed must preserve that override"
    );

    let forces_keep_open = MoonMenuItem::new("checkbox-row").closes_menu(false);
    assert_eq!(
        forces_keep_open.clone().closes_menu,
        Some(false),
        "cloning a row that forces the menu to stay open must preserve that override"
    );
}

/// `dropdown.rs:menu_width_sample` must bound a virtual menu's WIDTH measurement to a sample large
/// enough to cover any menu of ordinary size, independent of the height-only prefix that decides
/// which rows render first. Before the fix, `resolve_virtual_menu_width` measured only that
/// height-bounded prefix (roughly fifteen rows at a typical row height), so the fitted width
/// depended on WHAT SAT AT THE TOP of the menu: adding a few rows above a long label pushed it out
/// of the measured window and narrowed the whole menu, truncating a name that fitted a moment
/// earlier. Shrinking the sample back down to the height-only prefix
/// (`height_rows.min(items.len())`, dropping the `.max(MENU_WIDTH_SAMPLE_ROWS)` floor) reproduces
/// exactly that defect.
#[gpui::test]
fn menu_width_sample_ignores_where_a_long_label_sits(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init);
    cx.update(|cx| {
        let tokens = MoonThemeTokens::default();
        let metrics = MoonPopupMenu::new("width-sample-order")
            .size(MoonMenuSize::Compact)
            .metrics()
            .scaled(&tokens);
        // Small height budget: the height-only prefix covers only a handful of rows, far short of
        // both the 80-row list and the 128-row sample floor.
        let content_max = 100.0;
        let long_label = "a deliberately long menu label that only fits a wide menu";
        let width_policy = MoonMenuWidth::Fit {
            min: 50.0,
            max: 2000.0,
        };

        let mut items_at_top = vec![MoonMenuItem::new(long_label)];
        items_at_top.extend((0..79).map(|ix| MoonMenuItem::new(format!("Short {ix}"))));

        let mut items_buried = (0..50)
            .map(|ix| MoonMenuItem::new(format!("Short {ix}")))
            .collect::<Vec<_>>();
        items_buried.push(MoonMenuItem::new(long_label));
        items_buried.extend((50..79).map(|ix| MoonMenuItem::new(format!("Short {ix}"))));
        assert_eq!(items_at_top.len(), items_buried.len());

        let (width_at_top, _) = resolve_virtual_menu_width(
            width_policy, &items_at_top, metrics, &tokens, cx, false, content_max,
        );
        let (width_buried, _) = resolve_virtual_menu_width(
            width_policy, &items_buried, metrics, &tokens, cx, false, content_max,
        );

        assert_eq!(
            width_at_top, width_buried,
            "moving the long label past the height-only window must not narrow the fitted menu width"
        );
    });
}

/// [`MENU_WIDTH_SAMPLE_ROWS`] must stay a genuine BOUND, or the fix for width-order-dependence
/// (`dropdown.rs:menu_width_sample`) turns into unbounded per-frame text measurement on a
/// pathologically long menu. Catches replacing the bounded sample with the full item list
/// (`let take = items.len();`), using the same measurement-probe counter
/// `fitted_large_menu_measures_only_its_visible_window` relies on.
#[gpui::test]
fn menu_width_sample_stays_bounded_for_a_pathological_menu(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init);
    cx.update(|cx| {
        let tokens = MoonThemeTokens::default();
        let metrics = MoonPopupMenu::new("width-sample-bound")
            .size(MoonMenuSize::Compact)
            .metrics()
            .scaled(&tokens);
        let items: Vec<_> = (0..5_000)
            .map(|ix| MoonMenuItem::new(format!("{MENU_MEASUREMENT_PROBE_PREFIX}{ix}")))
            .collect();

        let before = menu_measurement_probe_count();
        let _ = resolve_virtual_menu_width(
            MoonMenuWidth::Fit {
                min: 50.0,
                max: 2000.0,
            },
            &items,
            metrics,
            &tokens,
            cx,
            false,
            100.0,
        );
        let measured = menu_measurement_probe_count() - before;

        // The probe counter is a shared global, so a small amount of cross-test contamination
        // from concurrently running tests is expected (the same reason
        // `fitted_large_menu_measures_only_its_visible_window` uses a `< 500` margin rather than
        // an exact count). 1,000 stays an order of magnitude below the 5,000-row source while
        // comfortably clearing that noise.
        assert!(
            measured < 1_000,
            "virtual menu width measured {measured} rows for a 5,000-row source, expected a bounded sample near {MENU_WIDTH_SAMPLE_ROWS}"
        );
    });
}

/// `dropdown.rs:resolve_virtual_menu_width`'s truncation flag must read the SAMPLE it actually
/// measured (`measured_rows.len() < items.len()`), not the height-only prefix it derives that
/// sample from. The height prefix is virtually always shorter than the full list for any menu
/// bigger than one screen, so reverting to it (`initial_rows.len() < items.len()`, the pre-fix
/// expression) reports "must truncate" even for an ordinary-size menu whose whole list was already
/// measured for width, forcing labels to truncate that already fit.
#[gpui::test]
fn virtual_menu_width_truncation_follows_the_measured_sample(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init);
    cx.update(|cx| {
        let tokens = MoonThemeTokens::default();
        let metrics = MoonPopupMenu::new("width-sample-truncation")
            .size(MoonMenuSize::Compact)
            .metrics()
            .scaled(&tokens);
        // Over the 64-row virtualization threshold but comfortably under the 128-row sample, so
        // the whole list is measured; a small height budget keeps the height-only prefix (a
        // handful of rows) far shorter than the full 100-row list.
        let items: Vec<_> = (0..100)
            .map(|ix| MoonMenuItem::new(format!("Short {ix}")))
            .collect();

        let (_, truncate) = resolve_virtual_menu_width(
            MoonMenuWidth::Fit {
                min: 50.0,
                max: 2000.0,
            },
            &items,
            metrics,
            &tokens,
            cx,
            false,
            100.0,
        );

        assert!(
            !truncate,
            "the whole 100-row list was measured (sample cap is 128), so no row-count truncation was needed"
        );
    });
}

/// `dropdown.rs:fit_dropdown_trigger_label` must preserve the caret and clamp against a
/// font-scaled ceiling independently of UI padding. Appending the caret downstream or scaling the
/// width with UI geometry makes long translated labels overflow at non-default font scale.
#[test]
fn fitted_trigger_preserves_caret_at_independent_scale_extremes() {
    for palette in [MoonPalette::TERMINAL, MoonPalette::LIGHT] {
        for (ui, font, font_delta) in [(0.5, 1.75, 0.0), (2.5, 0.75, 4.0)] {
            let mut tokens = MoonThemeTokens {
                palette,
                ..MoonThemeTokens::default()
            };
            tokens.scale = MoonScale {
                ui,
                font,
                font_delta,
            };
            let font_size = 10.5;
            let text_scale = tokens.font(font_size) / font_size;
            let measure = |text: &str| text.chars().count() as f32 * 8.0 * text_scale;
            let (label, width) = fit_dropdown_trigger_label(
                "a deliberately long translated selector label",
                DROPDOWN_CARET,
                MoonDropdownTriggerWidth::Fit {
                    min: 80.0,
                    max: 120.0,
                },
                &tokens,
                font_size,
                0.0,
                measure,
            );
            let width = width.expect("fitted trigger must resolve a rendered width");

            assert_eq!(width, 120.0 * text_scale);
            assert!(label.ends_with(DROPDOWN_CARET));
            assert!(label.contains('\u{2026}'));
            assert!(
                measure(label.as_ref()) + tokens.ui(DROPDOWN_TRIGGER_PAD_X) <= width,
                "fitted trigger overflowed at ui={ui}, font={font}, delta={font_delta}"
            );
        }
    }
}

/// `dropdown.rs:MoonDropdownTriggerWidth::Scaled` must use font scaling while retaining enough
/// UI-scaled padding for the ellipsis and component-owned caret. Replacing either scale with the
/// other clips fixed Terminal selectors at independent scale extremes.
#[test]
fn scaled_trigger_uses_font_width_without_clipping_component_chrome() {
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
            let font_size = 10.5;
            let text_scale = tokens.font(font_size) / font_size;
            let measure = |text: &str| text.chars().count() as f32 * 8.0 * text_scale;
            let (label, width) = fit_dropdown_trigger_label(
                "a deliberately long fixed selector label",
                DROPDOWN_CARET,
                MoonDropdownTriggerWidth::Scaled(120.0),
                &tokens,
                font_size,
                0.0,
                measure,
            );
            let width = width.expect("scaled trigger must resolve a rendered width");
            let minimum =
                tokens.ui(DROPDOWN_TRIGGER_PAD_X) + measure(&format!("\u{2026}{DROPDOWN_CARET}"));

            assert_eq!(width, (120.0 * text_scale).max(minimum));
            assert!(label.ends_with(DROPDOWN_CARET));
            assert!(
                measure(label.as_ref()) + tokens.ui(DROPDOWN_TRIGGER_PAD_X) <= width,
                "scaled trigger overflowed at ui={ui}, font={font}, delta={font_delta}"
            );
        }
    }
}

/// `dropdown.rs:MoonMenuWidth::Scaled` must follow font width without shrinking below UI-scaled
/// row chrome. Using only either scale makes a fixed Terminal menu overflow in the
/// high-UI/low-font cross-product.
#[gpui::test]
fn scaled_menu_width_retains_fitted_rows_at_independent_scale_extremes(
    cx: &mut gpui::TestAppContext,
) {
    cx.update(crate::init);
    for palette in [MoonPalette::TERMINAL, MoonPalette::LIGHT] {
        for (ui, font, font_delta) in [(0.5, 1.75, 0.0), (2.5, 0.75, 4.0), (2.5, 0.25, 0.0)] {
            cx.update(|cx| {
                let mut tokens = MoonThemeTokens {
                    palette,
                    ..MoonThemeTokens::default()
                };
                tokens.scale = MoonScale {
                    ui,
                    font,
                    font_delta,
                };
                let metrics = MoonPopupMenu::new("scaled-menu-test")
                    .size(MoonMenuSize::Compact)
                    .metrics()
                    .scaled(&tokens);
                let mut items = vec![
                    MoonMenuItem::new("a deliberately long menu label for truncation")
                        .right_label("12:34:56"),
                ];
                let text_scale = tokens.font(metrics.font_size) / metrics.font_size;
                let requested = 160.0 * text_scale;
                let (width, truncate) = resolve_menu_width(
                    MoonMenuWidth::Scaled(160.0),
                    &items,
                    metrics,
                    &tokens,
                    cx,
                    true,
                );

                assert!(truncate);
                assert!(width >= requested);
                fit_menu_item_labels(&mut items, width, metrics, &tokens, cx, true);
                let fitted_natural =
                    natural_menu_width(&items, metrics, &tokens, |text, size, weight| {
                        super::measure_text_width(cx, &tokens, text, size, weight, true)
                    });
                assert!(
                    fitted_natural <= width,
                    "scaled menu row overflowed at ui={ui}, font={font}, delta={font_delta}"
                );
            });
        }
    }
}

/// `dropdown.rs:MoonMenuMaxHeight::Ui` must scale with UI geometry while the legacy rendered
/// policy remains raw. Routing either through font scaling leaves menu scroll bounds detached from
/// their row heights.
#[test]
fn menu_max_height_distinguishes_ui_scaled_and_rendered_values() {
    let mut tokens = MoonThemeTokens::default();
    tokens.scale = MoonScale {
        ui: 2.5,
        font: 0.25,
        font_delta: 0.0,
    };

    assert_eq!(
        MoonMenuMaxHeight::Ui(300.0).resolve(&tokens),
        tokens.ui(300.0)
    );
    assert_eq!(MoonMenuMaxHeight::Rendered(300.0).resolve(&tokens), 300.0);
}

/// Open a deliberately large mixed menu containing separators, labels, and ordinary items.
struct LargeMixedMenuHarness;

impl gpui::Render for LargeMixedMenuHarness {
    /// Render a large open menu whose complete eager element tree would be observable by selectors.
    ///
    /// Args:
    ///     _window: Test window.
    ///     _cx: Test view context.
    ///
    /// Returns:
    ///     A height-bounded dropdown containing 1,000 mixed rows.
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        let items = (0..1_000).map(|ix| match ix % 3 {
            0 => MoonMenuItem::separator(),
            1 => MoonMenuItem::label(format!("Exchange {}", ix / 3)),
            _ => MoonMenuItem::new(format!("Core {ix}")),
        });
        MoonDropdown::new("large-mixed")
            .label("trigger")
            .default_open(true)
            .menu_max_height(180.0)
            .items(items)
    }
}

/// Catches replacing `dropdown.rs:menu_level_is_virtualized` with an eager path for mixed menus.
/// That edit constructs all 1,000 row elements before clipping, freezing core and log selectors
/// as their dynamic sources grow.
#[gpui::test]
fn large_mixed_menu_constructs_only_a_bounded_visible_window(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init);
    let window = cx.add_window(|_, _| LargeMixedMenuHarness);
    let mut cx = gpui::VisualTestContext::from_window(window.into(), cx);

    (0..8)
        .find(|_| {
            cx.update(|window, _| window.refresh());
            cx.run_until_parked();
            cx.debug_bounds("large-mixed:menu:item:2").is_some()
        })
        .expect("large mixed menu must render its first ordinary row");

    let separator = cx
        .debug_bounds("large-mixed:menu:item:0")
        .expect("the mixed virtual list must render separators");
    let label = cx
        .debug_bounds("large-mixed:menu:item:1")
        .expect("the mixed virtual list must render labels");

    assert!(
        cx.debug_bounds("large-mixed:menu:item:100").is_none(),
        "a row beyond the bounded viewport and overdraw must not be constructed"
    );
    assert!(
        cx.debug_bounds("large-mixed:menu:item:999").is_none(),
        "an off-screen tail row must not be constructed before scrolling"
    );
    assert!(
        separator.size.height < label.size.height,
        "mixed virtualization must retain compact separator geometry"
    );
}

/// Open a large dropdown whose rows are counted only when their models are cloned.
struct LargeCloneProbeMenuHarness;

impl gpui::Render for LargeCloneProbeMenuHarness {
    /// Render enough clone-probe rows to require virtualization.
    ///
    /// Args:
    ///     _window: Test window.
    ///     _cx: Test view context.
    ///
    /// Returns:
    ///     An open height-bounded dropdown.
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        MoonDropdown::new("large-clone-probe")
            .label("trigger")
            .default_open(true)
            .menu_max_height(180.0)
            .items(
                (0..1_000).map(|ix| MoonMenuItem::new(format!("{MENU_CLONE_PROBE_PREFIX}{ix:04}"))),
            )
    }
}

/// Catches restoring `popup_items.clone()` in `dropdown.rs:MoonDropdown::render`. That edit clones
/// every dynamic root row whenever the open popover content repaints, even though the virtual list
/// only constructs its bounded visible window.
#[gpui::test]
fn large_dropdown_repaint_clones_only_its_visible_window(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init);
    let window = cx.add_window(|_, _| LargeCloneProbeMenuHarness);
    let mut cx = gpui::VisualTestContext::from_window(window.into(), cx);

    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
    _ = take_menu_item_clone_probe_count();

    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
    let repaint_clones = take_menu_item_clone_probe_count();

    assert!(
        repaint_clones < 500,
        "virtual repaint cloned {repaint_clones} rows from a 1,000-row root level"
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

/// Open a palette-only popup menu through the public direct-render API.
struct LargePaletteMenuHarness;

impl gpui::Render for LargePaletteMenuHarness {
    /// Render enough direct popup rows to require virtualization.
    ///
    /// Args:
    ///     _window: Test window.
    ///     _cx: Test view context.
    ///
    /// Returns:
    ///     A palette-only menu with a bounded viewport.
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        MoonPopupMenu::new("large-palette")
            .items((0..1_000).map(|ix| MoonMenuItem::new(format!("Direct row {ix}"))))
            .max_height(180.0)
            .render_with_palette(MoonPalette::TERMINAL)
    }
}

/// Catches passing `None` from `dropdown.rs:MoonPopupMenu::render_with_palette` into the shared
/// renderer. That mutation eagerly constructs all 1,000 direct popup rows and restores the public
/// non-virtualized escape path.
#[gpui::test]
fn palette_only_large_menu_constructs_a_bounded_visible_window(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init);
    let window = cx.add_window(|_, _| LargePaletteMenuHarness);
    let mut cx = gpui::VisualTestContext::from_window(window.into(), cx);

    cx.update(|window, _| window.refresh());
    cx.run_until_parked();

    assert!(
        cx.debug_bounds("large-palette:item:0").is_some(),
        "direct popup must render its first visible row"
    );
    assert!(
        cx.debug_bounds("large-palette:item:999").is_none(),
        "direct popup must not construct its off-screen tail"
    );
}

/// Catches constructing a fresh `ListState` inside
/// `dropdown.rs:MoonPopupMenu::render_with_palette`. That edit returns a scrolled direct popup to
/// its first row on the next ordinary view refresh.
#[gpui::test]
fn palette_only_large_menu_retains_scroll_across_repaint(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init);
    let window = cx.add_window(|_, _| LargePaletteMenuHarness);
    let mut cx = gpui::VisualTestContext::from_window(window.into(), cx);

    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
    let first_row = cx
        .debug_bounds("large-palette:item:0")
        .expect("direct popup must initially render its first row");

    cx.simulate_event(gpui::ScrollWheelEvent {
        position: first_row.center(),
        delta: gpui::ScrollDelta::Pixels(gpui::point(gpui::px(0.0), gpui::px(-240.0))),
        ..Default::default()
    });
    cx.run_until_parked();
    assert!(
        cx.debug_bounds("large-palette:item:0").is_none(),
        "scrolling must move the first row outside the virtualized window"
    );

    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
    assert!(
        cx.debug_bounds("large-palette:item:0").is_none(),
        "an ordinary repaint must retain the direct popup's scroll position"
    );
}

/// Open a large fitted menu whose labels are recognized by the measurement probe.
struct LargeFittedMenuHarness;

impl gpui::Render for LargeFittedMenuHarness {
    /// Render a fitted menu with many off-screen labels.
    ///
    /// Args:
    ///     _window: Test window.
    ///     _cx: Test view context.
    ///
    /// Returns:
    ///     A height-bounded fitted popup.
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        MoonPopupMenu::new("large-fitted")
            .items(
                (0..1_000)
                    .map(|ix| MoonMenuItem::new(format!("{MENU_MEASUREMENT_PROBE_PREFIX}{ix:04}"))),
            )
            .fit_width(100.0, 240.0)
            .max_height(180.0)
    }
}

/// Catches routing a virtual fitted level through `dropdown.rs:resolve_menu_width` or fitting all
/// labels before list construction. Either edit performs at least one sentinel measurement per
/// off-screen row and makes first-open latency grow with the complete dynamic source.
#[gpui::test]
fn fitted_large_menu_measures_only_its_visible_window(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init);
    let before = menu_measurement_probe_count();
    let window = cx.add_window(|_, _| LargeFittedMenuHarness);
    let mut cx = gpui::VisualTestContext::from_window(window.into(), cx);

    cx.update(|window, _| window.refresh());
    cx.run_until_parked();

    let measured = menu_measurement_probe_count() - before;
    assert!(measured > 0, "visible fitted labels must still be measured");
    assert!(
        measured < 500,
        "virtual fitted menu measured {measured} sentinel strings for a 1,000-row source"
    );
    assert!(
        cx.debug_bounds("large-fitted:item:999").is_none(),
        "the fitted menu tail must remain outside the constructed window"
    );
}

/// Render equal short-label fitted menus immediately below and at the virtualization threshold.
struct FitThresholdHarness;

impl gpui::Render for FitThresholdHarness {
    /// Render two fitted menu levels that differ only by one repeated short row.
    ///
    /// Args:
    ///     _window: Test window.
    ///     _cx: Test view context.
    ///
    /// Returns:
    ///     A vertical pair of eager and virtualized fitted menus.
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        gpui::div()
            .flex()
            .flex_col()
            .child(
                MoonPopupMenu::new("fit-threshold-eager")
                    .items((0..63).map(|_| MoonMenuItem::new("Short")))
                    .fit_width(100.0, 240.0),
            )
            .child(
                MoonPopupMenu::new("fit-threshold-virtual")
                    .items((0..64).map(|_| MoonMenuItem::new("Short")))
                    .fit_width(100.0, 240.0),
            )
    }
}

/// Catches resolving virtual `Fit` widths directly to their declared maximum. That edit makes a
/// short-label menu jump wider when its row count crosses the virtualization threshold.
#[gpui::test]
fn fitted_width_does_not_jump_at_virtualization_threshold(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init);
    let window = cx.add_window(|_, _| FitThresholdHarness);
    let mut cx = gpui::VisualTestContext::from_window(window.into(), cx);

    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
    let eager = cx
        .debug_bounds("fit-threshold-eager")
        .expect("eager fitted menu must render");
    let virtualized = cx
        .debug_bounds("fit-threshold-virtual")
        .expect("virtual fitted menu must render");

    assert_eq!(
        eager.size.width, virtualized.size.width,
        "repeating one short row must not change the fitted menu width at the threshold"
    );
}

/// Open a large parent level with a selected submenu and record submenu activation.
struct LargeSubmenuHarness {
    activations: Rc<Cell<usize>>,
}

impl gpui::Render for LargeSubmenuHarness {
    /// Render a virtualized parent whose first row opens a submenu beyond the list mask.
    ///
    /// Args:
    ///     _window: Test window.
    ///     _cx: Test view context.
    ///
    /// Returns:
    ///     A large parent menu with one immediately visible submenu action.
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        let activations = self.activations.clone();
        let first = MoonMenuItem::new("More")
            .selected(true)
            .submenu([
                MoonMenuItem::new("Run nested action").on_click(move |_, _, _| {
                    activations.set(activations.get() + 1);
                }),
            ]);
        MoonPopupMenu::new("virtual-submenu-parent")
            .item(first)
            .items((1..1_000).map(|ix| MoonMenuItem::new(format!("Parent row {ix}"))))
            .max_height(180.0)
    }
}

/// Catches removing `deferred` around the selected submenu in
/// `dropdown.rs:MoonPopupMenu::render_item`. Without that escape, GPUI clips the submenu hitbox to
/// the virtual parent's list mask, so the visible nested action cannot be clicked.
#[gpui::test]
fn virtualized_parent_submenu_escapes_the_list_mask(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init);
    let activations = Rc::new(Cell::new(0));
    let sink = activations.clone();
    let window = cx.add_window(move |_, _| LargeSubmenuHarness { activations: sink });
    let mut cx = gpui::VisualTestContext::from_window(window.into(), cx);

    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
    let parent = cx
        .debug_bounds("virtual-submenu-parent")
        .expect("virtual parent menu must render");
    let action = cx
        .debug_bounds("virtual-submenu-parent:submenu:0:item:0")
        .expect("selected submenu action must render");

    assert!(
        action.origin.x >= parent.right(),
        "submenu action must be laid out beyond the virtual parent"
    );
    cx.simulate_click(action.center(), gpui::Modifiers::none());
    assert_eq!(
        activations.get(),
        1,
        "submenu action outside the parent list mask must remain clickable"
    );
}

/// Open a small parent whose selected submenu contains many clone-probe rows.
struct LargeNestedCloneProbeHarness;

impl gpui::Render for LargeNestedCloneProbeHarness {
    /// Render one selected parent with a virtualized nested level.
    ///
    /// Args:
    ///     _window: Test window.
    ///     _cx: Test view context.
    ///
    /// Returns:
    ///     A parent menu with a 1,000-row selected submenu.
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        let parent = MoonMenuItem::new("More").selected(true).submenu(
            (0..1_000)
                .map(|ix| MoonMenuItem::new(format!("{MENU_CLONE_PROBE_PREFIX}nested-{ix:04}"))),
        );
        MoonPopupMenu::new("large-nested-clone-probe").item(parent)
    }
}

/// Catches rebuilding a selected submenu with `.items(submenu.iter().cloned())`. That edit clones
/// all 1,000 nested rows on every parent repaint before the nested level can virtualize.
#[gpui::test]
fn large_selected_submenu_repaint_clones_only_visible_rows(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init);
    let window = cx.add_window(|_, _| LargeNestedCloneProbeHarness);
    let mut cx = gpui::VisualTestContext::from_window(window.into(), cx);

    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
    assert!(
        cx.debug_bounds("large-nested-clone-probe:submenu:0:item:0")
            .is_some(),
        "the nested virtual level must render its first visible row"
    );
    _ = take_menu_item_clone_probe_count();

    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
    let repaint_clones = take_menu_item_clone_probe_count();
    assert!(
        repaint_clones < 500,
        "selected submenu repaint cloned {repaint_clones} rows from a 1,000-row nested level"
    );
}

/// Open an explicit-palette root with one selected submenu palette probe.
struct PaletteSubmenuHarness;

impl gpui::Render for PaletteSubmenuHarness {
    /// Render a selected submenu through the public explicit-palette route.
    ///
    /// Args:
    ///     _window: Test window.
    ///     _cx: Test view context.
    ///
    /// Returns:
    ///     A terminal-palette root whose active application theme is deliberately different.
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        MoonPopupMenu::new("palette-submenu")
            .item(
                MoonMenuItem::new("More")
                    .selected(true)
                    .submenu([MoonMenuItem::new(format!(
                        "{MENU_PALETTE_PROBE_PREFIX}nested"
                    ))]),
            )
            .render_with_palette(MoonPalette::TERMINAL)
    }
}

/// Catches rendering a selected submenu through ordinary active-theme `RenderOnce`. That edit
/// makes an explicit terminal-palette root open a submenu painted with the active light palette.
#[gpui::test]
fn explicit_palette_is_inherited_by_selected_submenu(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init);
    cx.update(|cx| {
        MoonTheme::set_mode(ThemeMode::Light, cx);
        assert_ne!(
            MoonPalette::active(cx).shell,
            MoonPalette::TERMINAL.shell,
            "the test requires an active palette distinct from the explicit terminal palette"
        );
    });
    _ = take_palette_probe_shell();
    let window = cx.add_window(|_, _| PaletteSubmenuHarness);
    let mut cx = gpui::VisualTestContext::from_window(window.into(), cx);

    cx.update(|window, _| window.refresh());
    cx.run_until_parked();

    assert_eq!(
        take_palette_probe_shell(),
        MoonPalette::TERMINAL.shell,
        "selected submenu must inherit the explicit root palette"
    );
}

/// `dropdown.rs:MoonPopupMenu::render_with_palette` must fail loudly for widths that require an
/// `App` text system. Falling back to the fit minimum or default scale would render user menus at
/// the wrong width and clip their labels.
#[test]
fn palette_only_menu_render_rejects_measured_width_policies() {
    assert!(
        std::panic::catch_unwind(|| {
            let _ = MoonPopupMenu::new("scaled-palette-only")
                .width_scaled(160.0)
                .render_with_palette(MoonPalette::TERMINAL);
        })
        .is_err()
    );
    assert!(
        std::panic::catch_unwind(|| {
            let _ = MoonPopupMenu::new("fitted-palette-only")
                .fit_width(80.0, 240.0)
                .render_with_palette(MoonPalette::TERMINAL);
        })
        .is_err()
    );
}

/// `dropdown.rs:natural_menu_width` must reserve both the right-label glyphs and the additional
/// flex gap they introduce. Omitting either clips the clock's time column at its fitted width.
#[test]
fn fitted_menu_accounts_for_right_label_and_its_gap() {
    let tokens = MoonThemeTokens::default();
    let metrics = MenuMetrics {
        row_height: 20.0,
        font_size: 9.5,
        line_height: 12.0,
        radius: 3.0,
        pad_x: 6.0,
        gap: 5.0,
    }
    .scaled(&tokens);
    let plain = [MoonMenuItem::new("UTC+12")];
    let with_right = [MoonMenuItem::new("UTC+12").right_label("12:34:56")];
    let measure = |text: &str, _size: f32, _weight: f32| text.chars().count() as f32;

    let plain_width = natural_menu_width(&plain, metrics, &tokens, measure);
    let right_width = natural_menu_width(&with_right, metrics, &tokens, measure);

    assert_eq!(
        right_width - plain_width,
        "12:34:56".chars().count() as f32 + metrics.gap
    );
}

/// Root view containing a fitted parent menu with an immediately visible selected submenu.
struct FittedSubmenuHarness;

impl gpui::Render for FittedSubmenuHarness {
    /// Render parent and submenu from deliberately different content widths.
    ///
    /// Args:
    ///     _window: Test window.
    ///     _cx: Test view context.
    ///
    /// Returns:
    ///     A fitted parent menu with its selected submenu visible.
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        MoonPopupMenu::new("fit-parent")
            .size(MoonMenuSize::Compact)
            .fit_width(80.0, 400.0)
            .item(
                MoonMenuItem::new("More")
                    .selected(true)
                    .submenu([MoonMenuItem::new(
                        "a submenu label that is much wider than its parent",
                    )]),
            )
    }
}

/// `dropdown.rs:MoonPopupMenu::render_item` must pass the fit policy, not the resolved parent
/// width, into a submenu. Restoring `.width(menu_width)` makes this named child clip to the narrow
/// parent even though its own label fits below the shared maximum.
#[gpui::test]
fn fitted_submenu_resolves_width_from_its_own_items(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init);
    let scale = MoonScale {
        ui: 2.5,
        font: 0.75,
        font_delta: 4.0,
    };
    cx.update(|cx| {
        MoonTheme::global_mut(cx).scale = scale;
    });
    let window = cx.add_window(|_, _| FittedSubmenuHarness);
    let mut cx = gpui::VisualTestContext::from_window(window.into(), cx);

    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
    let parent = cx
        .debug_bounds("fit-parent")
        .expect("fitted parent menu must render");
    let submenu = cx
        .debug_bounds("fit-parent:submenu:0")
        .expect("selected submenu must render");
    let row = cx
        .debug_bounds("fit-parent:item:0")
        .expect("selected submenu row must render");
    let mut tokens = MoonThemeTokens::default();
    tokens.scale = scale;

    assert!(
        submenu.size.width > parent.size.width,
        "submenu width {:?} must follow its own longer label, not parent width {:?}",
        submenu.size.width,
        parent.size.width
    );
    assert_eq!(
        submenu.origin.x - row.right(),
        gpui::px(tokens.ui(SUBMENU_OFFSET_X)),
        "submenu must position from the rendered row edge"
    );
    assert_eq!(
        submenu.origin.y - row.origin.y,
        gpui::px(-tokens.ui(MENU_PADDING)),
        "submenu top overlap must follow UI scale"
    );
}

/// `dropdown.rs:menu_width_requirements`'s `Label` arm must reserve its own `right_label` the same
/// way the `Item` arm does. Resolving `(0.0, 0.0)` for a label row's trailing text instead of
/// calling `trailing_label_widths` lets a label's trailing count spill past a Scaled/Fit menu's
/// resolved right edge.
#[test]
fn fitted_label_row_accounts_for_right_label_and_its_gap() {
    let tokens = MoonThemeTokens::default();
    let metrics = MenuMetrics {
        row_height: 20.0,
        font_size: 9.5,
        line_height: 12.0,
        radius: 3.0,
        pad_x: 6.0,
        gap: 5.0,
    }
    .scaled(&tokens);
    let plain = [MoonMenuItem::label("Exchanges")];
    let with_right = [MoonMenuItem::label("Exchanges").right_label("42")];
    let measure = |text: &str, _size: f32, _weight: f32| text.chars().count() as f32;

    let plain_width = natural_menu_width(&plain, metrics, &tokens, measure);
    let right_width = natural_menu_width(&with_right, metrics, &tokens, measure);

    assert_eq!(
        right_width - plain_width,
        "42".chars().count() as f32 + metrics.gap,
        "a label row's resolved width must grow by exactly its trailing text plus one gap"
    );
}

/// `dropdown.rs:menu_content_max` must charge the pinned header's budget straight to the row
/// list. Dropping the `- header_budget` term lets a virtualized menu with a header size its list
/// as if the header were absent, so the menu overshoots its own maximum height and the outer
/// `overflow_hidden` silently clips the last rows.
#[test]
fn menu_content_max_charges_the_header_budget_to_the_row_list() {
    let tokens = MoonThemeTokens::default();
    let metrics = MenuMetrics {
        row_height: 24.0,
        font_size: 10.5,
        line_height: 13.0,
        radius: 4.0,
        pad_x: 7.0,
        gap: 6.0,
    }
    .scaled(&tokens);
    let outer_max = 400.0;
    let header_budget = 50.0;

    let without_header = menu_content_max(outer_max, &tokens, 0.0, metrics);
    let with_header = menu_content_max(outer_max, &tokens, header_budget, metrics);

    assert_eq!(
        without_header - with_header,
        header_budget,
        "a pinned header's declared budget must come straight off the row list's viewport"
    );
}

/// `dropdown.rs:clamp_header_budget` must cap a header declared taller than the menu allows so at
/// least one row still fits beneath it. Returning `declared` unclamped leaves the row list no
/// space at all and the menu opens as an unusable strip.
#[test]
fn clamp_header_budget_leaves_room_for_at_least_one_row() {
    let chrome = 10.0;
    let row_height = 24.0;
    let outer_max = 100.0;
    // Independent of the clamp under test: the exact declared height that leaves precisely one
    // row's worth of space once chrome is paid for.
    let allowed = outer_max - chrome - row_height;

    assert_eq!(
        clamp_header_budget(allowed, outer_max, chrome, row_height),
        allowed,
        "a header declared exactly at the row-preserving limit must pass through unclamped"
    );
    assert_eq!(
        clamp_header_budget(allowed + 1.0, outer_max, chrome, row_height),
        allowed,
        "a header declared one unit past the limit must be capped back to it, not left unclamped"
    );
}

/// UI-scaled height declared for the pinned header probe below. Chosen far larger than both the
/// default row height and the probe's own 4px content, so an unenforced wrapper collapses the gap
/// to the header's real content height instead of the declared value.
const HEADER_HEIGHT_UI: f32 = 300.0;

/// Open direct popup menu with one pinned header whose own content is far shorter than its
/// declared height, so the wrapper's enforced `.h(px(height))` is the only thing separating it
/// from the first row.
struct HeaderHeightHarness;

impl gpui::Render for HeaderHeightHarness {
    /// Render the probe menu.
    ///
    /// Args:
    ///     _window: Test window.
    ///     _cx: Test view context.
    ///
    /// Returns:
    ///     A popup menu with one pinned header and one row.
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        use gpui::{InteractiveElement as _, Styled as _};

        MoonPopupMenu::new("header-height-probe")
            .header(
                HEADER_HEIGHT_UI,
                gpui::div()
                    .debug_selector(|| "header-height-probe:header".into())
                    .h(gpui::px(4.0))
                    .w(gpui::px(4.0)),
            )
            .item(MoonMenuItem::new("only item"))
    }
}

/// Catches removing `.h(px(height))` from the pinned-header wrapper in
/// `dropdown.rs:MoonPopupMenu::render_with_metrics`. Without it the wrapper shrinks to the
/// header's own natural content size instead of the height its budget reserved, so an
/// over-declared header wastes list space and an under-declared one pushes rows past the cap.
#[gpui::test]
fn pinned_header_wrapper_enforces_its_declared_height(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init);
    cx.update(|cx| {
        MoonTheme::global_mut(cx).scale = MoonScale {
            ui: 1.0,
            font: 1.0,
            font_delta: 0.0,
        };
    });
    let window = cx.add_window(|_, _| HeaderHeightHarness);
    let mut cx = gpui::VisualTestContext::from_window(window.into(), cx);

    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
    let header = cx
        .debug_bounds("header-height-probe:header")
        .expect("pinned header must render");
    let row = cx
        .debug_bounds("header-height-probe:item:0")
        .expect("first row must render");

    let observed_gap = row.origin.y - (header.origin.y + header.size.height);

    assert!(
        observed_gap >= gpui::px(HEADER_HEIGHT_UI - 4.0),
        "row started at {:?}, only {:?} below the header's own 4px content; the wrapper must be \
         pinned to its declared {HEADER_HEIGHT_UI}px height rather than shrinking to the header's \
         natural size",
        row.origin,
        observed_gap
    );
}

/// Open a direct popup menu capped well below its eight rows' natural height, with a pinned header,
/// and deliberately keep it below [`super::VIRTUAL_MENU_ITEM_THRESHOLD`] so it renders through the
/// eager branch rather than the virtualized `list()`.
struct EagerCappedHeaderHarness;

impl gpui::Render for EagerCappedHeaderHarness {
    /// Render the probe menu.
    ///
    /// Args:
    ///     _window: Test window.
    ///     _cx: Test view context.
    ///
    /// Returns:
    ///     A capped, sub-threshold popup menu with a pinned header.
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        use gpui::{InteractiveElement as _, Styled as _};

        MoonPopupMenu::new("eager-capped-header")
            .max_height(120.0)
            .header(
                28.0,
                gpui::div()
                    .debug_selector(|| "eager-capped-header:header".into())
                    .h(gpui::px(28.0))
                    .w_full(),
            )
            .items((0..8).map(|ix| MoonMenuItem::new(format!("Row {ix}"))))
    }
}

/// Catches moving `.flex_1()`/`.overflow_y_scroll()` from the row list to the outer menu in
/// `dropdown.rs:MoonPopupMenu::render_with_metrics`'s capped eager branch. Without its own scroll
/// region, a sub-threshold row list cannot reveal rows past the cap while keeping its header pinned.
#[gpui::test]
fn eager_capped_menu_scrolls_its_rows_independently_of_the_pinned_header(
    cx: &mut gpui::TestAppContext,
) {
    cx.update(crate::init);
    let window = cx.add_window(|_, _| EagerCappedHeaderHarness);
    let mut cx = gpui::VisualTestContext::from_window(window.into(), cx);

    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
    let header_before = cx
        .debug_bounds("eager-capped-header:header")
        .expect("pinned header must render");
    let row0_before = cx
        .debug_bounds("eager-capped-header:item:0")
        .expect("first row must render");

    cx.simulate_event(gpui::ScrollWheelEvent {
        position: row0_before.center(),
        delta: gpui::ScrollDelta::Pixels(gpui::point(gpui::px(0.0), gpui::px(-400.0))),
        ..Default::default()
    });
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();

    let header_after = cx
        .debug_bounds("eager-capped-header:header")
        .expect("pinned header must remain rendered after scrolling the row list");
    let row0_after = cx
        .debug_bounds("eager-capped-header:item:0")
        .expect("scrolled row list must keep rendering its rows");

    assert_eq!(
        header_after.origin.y, header_before.origin.y,
        "the pinned header must not move when the row list scrolls"
    );
    assert_ne!(
        row0_after.origin.y, row0_before.origin.y,
        "the eager capped branch must let its own row list scroll, not sit inert with the whole \
         menu clipped from outside"
    );
}

/// Maximum outer height for the virtual pinned-header parity probe.
const VIRTUAL_HEADER_MENU_MAX_HEIGHT: f32 = 120.0;

/// Open a virtualized popup whose pinned header must remain outside its scrolling row list.
struct VirtualCappedHeaderHarness;

impl gpui::Render for VirtualCappedHeaderHarness {
    /// Render a capped menu at the virtualization threshold with one pinned header.
    ///
    /// Args:
    ///     _window: Test window.
    ///     _cx: Test view context.
    ///
    /// Returns:
    ///     A virtualized popup menu whose rows exceed its available viewport.
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        use gpui::{InteractiveElement as _, Styled as _};

        MoonPopupMenu::new("virtual-capped-header")
            .max_height(VIRTUAL_HEADER_MENU_MAX_HEIGHT)
            .header(
                28.0,
                gpui::div()
                    .debug_selector(|| "virtual-capped-header:header".into())
                    .h(gpui::px(28.0))
                    .w_full(),
            )
            .items(
                (0..super::VIRTUAL_MENU_ITEM_THRESHOLD)
                    .map(|ix| MoonMenuItem::new(format!("Row {ix}"))),
            )
    }
}

/// Catches preserving pinned-header scrolling only in the eager branch while moving virtual
/// headers into, or omitting them from, the retained list. The menu maximum is an independent
/// outer-height oracle; the header origin must stay fixed while row zero leaves the constructed
/// virtual window after scrolling.
#[gpui::test]
fn virtual_capped_menu_scrolls_its_rows_independently_of_the_pinned_header(
    cx: &mut gpui::TestAppContext,
) {
    cx.update(crate::init);
    let window = cx.add_window(|_, _| VirtualCappedHeaderHarness);
    let mut cx = gpui::VisualTestContext::from_window(window.into(), cx);

    let (menu, header_before, row0_before) = (0..8)
        .find_map(|_| {
            cx.update(|window, _| window.refresh());
            cx.run_until_parked();
            Some((
                cx.debug_bounds("virtual-capped-header")?,
                cx.debug_bounds("virtual-capped-header:header")?,
                cx.debug_bounds("virtual-capped-header:item:0")?,
            ))
        })
        .expect("virtual capped menu, pinned header, and first row must render");

    assert!(
        menu.size.height <= gpui::px(VIRTUAL_HEADER_MENU_MAX_HEIGHT),
        "virtual menu height {:?} exceeded its independent {}px outer cap",
        menu.size.height,
        VIRTUAL_HEADER_MENU_MAX_HEIGHT
    );

    cx.simulate_event(gpui::ScrollWheelEvent {
        position: row0_before.center(),
        delta: gpui::ScrollDelta::Pixels(gpui::point(gpui::px(0.0), gpui::px(-400.0))),
        ..Default::default()
    });
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();

    let header_after = cx
        .debug_bounds("virtual-capped-header:header")
        .expect("pinned virtual header must remain rendered after row scrolling");
    assert_eq!(
        header_after.origin.y, header_before.origin.y,
        "the pinned virtual header must not move with the retained row list"
    );
    assert!(
        cx.debug_bounds("virtual-capped-header:item:0").is_none(),
        "scrolling must move row zero outside the constructed virtual window"
    );
}

/// UI-scaled height declared for the pinned header probe below, deliberately far larger than the
/// capped menu's own maximum so `clamp_header_budget`'s scale must engage (< 1.0) rather than
/// resolve to a no-op.
const OVERSIZED_HEADER_HEIGHT_UI: f32 = 200.0;
/// Outer cap declared for the same probe menu, small enough that the oversized header alone
/// already exceeds it.
const HEADER_CLAMP_MENU_MAX_HEIGHT: f32 = 100.0;

/// Open direct popup menu whose declared header height and outer cap force the header-scaling
/// clamp to actually engage, unlike an uncapped menu where the scale trivially resolves to 1.0.
struct HeaderClampScalingHarness;

impl gpui::Render for HeaderClampScalingHarness {
    /// Render the probe menu.
    ///
    /// Args:
    ///     _window: Test window.
    ///     _cx: Test view context.
    ///
    /// Returns:
    ///     A capped popup menu with one oversized pinned header and one row.
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        use gpui::{InteractiveElement as _, Styled as _};

        MoonPopupMenu::new("header-clamp-scaling-probe")
            .max_height(HEADER_CLAMP_MENU_MAX_HEIGHT)
            .header(
                OVERSIZED_HEADER_HEIGHT_UI,
                gpui::div()
                    .debug_selector(|| "header-clamp-scaling-probe:header".into())
                    .h(gpui::px(4.0))
                    .w(gpui::px(4.0)),
            )
            .item(MoonMenuItem::new("only item"))
    }
}

/// Catches replacing `dropdown.rs:MoonPopupMenu::render_with_metrics`'s header-scaling block
/// (`let header_heights: Vec<f32> = if requested_total > 0.0 { .. } else { requested_heights };`)
/// with a bare `let header_heights = requested_heights;`. `clamp_header_budget`'s surviving budget
/// then never reaches the header wrapper's own `.h(px(height))`: the wrapper keeps rendering at its
/// full, unclamped requested height while the row list is still sized as if it had been clamped,
/// so header plus list overrun the menu's own maximum and the outer `overflow_hidden` silently
/// clips the tail of the row list.
#[gpui::test]
fn pinned_header_scaling_shrinks_the_wrapper_when_the_clamp_engages(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init);
    cx.update(|cx| {
        MoonTheme::global_mut(cx).scale = MoonScale {
            ui: 1.0,
            font: 1.0,
            font_delta: 0.0,
        };
    });
    let window = cx.add_window(|_, _| HeaderClampScalingHarness);
    let mut cx = gpui::VisualTestContext::from_window(window.into(), cx);

    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
    let header = cx
        .debug_bounds("header-clamp-scaling-probe:header")
        .expect("pinned header must render");
    let row = cx
        .debug_bounds("header-clamp-scaling-probe:item:0")
        .expect("first row must render");

    // The wrapper's own height plus the outer flex column's gap to the first row: the wrapper
    // itself carries no debug selector, so this is the only way to observe how tall it actually
    // rendered.
    let header_block = row.origin.y - header.origin.y;

    assert!(
        header_block < gpui::px(OVERSIZED_HEADER_HEIGHT_UI),
        "header block measured {header_block:?}, which is not below its declared \
         {OVERSIZED_HEADER_HEIGHT_UI}px request; the surviving budget from clamp_header_budget \
         must be spent back onto the header wrapper, not left unscaled",
    );
    assert!(
        header_block <= gpui::px(HEADER_CLAMP_MENU_MAX_HEIGHT),
        "header block {header_block:?} alone exceeds the menu's own \
         {HEADER_CLAMP_MENU_MAX_HEIGHT}px maximum; an unscaled header pushes the row list past the \
         cap, and the outer overflow_hidden then clips it silently",
    );
}
