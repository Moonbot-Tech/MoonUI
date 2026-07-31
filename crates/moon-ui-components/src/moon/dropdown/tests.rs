//! Regression coverage for dropdown placement, sizing, selection, and submenu behavior.

use super::{
    DROPDOWN_CARET, DROPDOWN_TRIGGER_PAD_X, MENU_CLONE_PROBE_PREFIX,
    MENU_DROPDOWN_HANDLER_PROBE_PREFIX, MENU_MEASUREMENT_PROBE_PREFIX, MENU_PADDING,
    MENU_PALETTE_PROBE_PREFIX, MenuMetrics, MoonButtonSize, MoonDropdown, MoonDropdownSelectPlan,
    MoonDropdownTriggerWidth, MoonMenuItem, MoonMenuItemKind, MoonMenuMaxHeight, MoonMenuSize,
    MoonMenuWidth, MoonPalette, MoonPopupMenu, MoonRect, MoonThemeTokens, SUBMENU_OFFSET_X,
    capped_menu_items_height, fit_dropdown_trigger_label, fit_menu_item_labels,
    menu_measurement_probe_count, moon_dropdown_select_plan, moon_menu_item_accepts_click,
    natural_menu_width, resolve_menu_width, take_dropdown_handler_probe_count,
    take_menu_item_clone_probe_count, take_palette_probe_shell,
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
        moon_dropdown_select_plan(true, None),
        MoonDropdownSelectPlan {
            close_menu: true,
            update_internal_open: true,
        }
    );
    assert_eq!(
        moon_dropdown_select_plan(true, Some(true)),
        MoonDropdownSelectPlan {
            close_menu: true,
            update_internal_open: false,
        }
    );
    assert_eq!(
        moon_dropdown_select_plan(false, None),
        MoonDropdownSelectPlan {
            close_menu: false,
            update_internal_open: false,
        }
    );
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
