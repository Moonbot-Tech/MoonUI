//! Regression coverage for dropdown placement, sizing, selection, and submenu behavior.

use super::{
    DROPDOWN_CARET, DROPDOWN_TRIGGER_PAD_X, MENU_PADDING, MenuMetrics, MoonButtonSize,
    MoonDropdown, MoonDropdownSelectPlan, MoonDropdownTriggerWidth, MoonMenuItem, MoonMenuItemKind,
    MoonMenuMaxHeight, MoonMenuSize, MoonMenuWidth, MoonPalette, MoonPopupMenu, MoonRect,
    MoonThemeTokens, SUBMENU_OFFSET_X, fit_dropdown_trigger_label, fit_menu_item_labels,
    moon_dropdown_select_plan, moon_menu_item_accepts_click, natural_menu_width,
    resolve_menu_width,
};
use crate::moon::{MoonScale, MoonTheme};
use std::{cell::RefCell, rc::Rc};

/// Stable element id used by dropdown geometry probes.
const DROPDOWN_ID: &str = "geometry-probe";
/// `MoonDropdown::render` names its menu `{id}:menu`, and `MoonPopupMenu` registers that id as its
/// debug selector.
const MENU_SELECTOR: &str = "geometry-probe:menu";

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
    let window = cx.add_window({
        let action_clicks = action_clicks.clone();
        let static_clicks = static_clicks.clone();
        move |_, _| LabelInteractionHarness {
            action_clicks,
            static_clicks,
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

    cx.simulate_click(static_label.center(), gpui::Modifiers::default());
    assert_eq!(
        *static_clicks.borrow(),
        0,
        "ordinary label must remain inert even when a handler is attached"
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
