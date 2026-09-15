//! Regression coverage for Moon button sizing and icon-slot rendering.

use super::super::{MoonDisclosureDirection, MoonPalette, MoonSize};
use super::{
    MoonButton, MoonButtonIconSlot, MoonButtonSize, MoonTheme, MoonThemeTokens,
    button_text_metrics, tier_button_metrics,
};

/// Catches ignoring `button.rs:MoonButton::width` or `full_width` during rendering, which would
/// make fixed controls use content width or prevent full-row actions from filling their parent.
#[gpui::test]
fn moon_button_width_builders_preserve_layout_intent(cx: &mut gpui::TestAppContext) {
    let fixed = laid_out_bounds(cx, || MoonButton::new("fixed").label("Fixed").width(42.0));
    let full = laid_out_bounds(cx, || MoonButton::new("full").label("Full").full_width());

    assert_eq!(fixed.size.width, gpui::px(42.0));
    assert_eq!(full.size.width, gpui::px(200.0));
}

/// Catches removing the scaled `Button::px` refinement from
/// `button.rs:MoonButton::render`, which would put localized Action-button text flush against its
/// outline again. The paired default/padded geometry independently proves the seven-unit inset on
/// both sides at two UI scales and in both supported themes.
#[gpui::test]
fn explicit_horizontal_padding_adds_scaled_insets_in_both_themes(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init);
    for palette in [MoonPalette::TERMINAL, MoonPalette::LIGHT] {
        for (ui_scale, expected_delta) in [(1.0, 14.0), (2.0, 28.0)] {
            cx.update(|cx| {
                let theme = MoonTheme::global_mut(cx);
                theme.palette = palette;
                theme.scale.ui = ui_scale;
            });
            // `padding_x` OVERRIDES the tier's own native pad_x rather than adding to it (see
            // its doc comment), so the baseline must be an explicit override too - Tier(Sm) now
            // carries a non-zero native pad_x, and comparing against the unadorned default would
            // measure that tier's padding instead of the override's own scaling.
            let narrow = laid_out_bounds(cx, || {
                MoonButton::new("action-narrow")
                    .label("Action")
                    .size(MoonSize::Sm)
                    .padding_x(0.0)
            });
            let padded = laid_out_bounds(cx, || {
                MoonButton::new("action-padded")
                    .label("Action")
                    .size(MoonSize::Sm)
                    .padding_x(7.0)
            });

            assert_eq!(
                padded.size.width - narrow.size.width,
                gpui::px(expected_delta),
                "horizontal padding delta is wrong for ui_scale={ui_scale}"
            );
        }
    }
}

/// Root view that renders one button and records the laid-out bounds of the wrapper's direct
/// child.
///
/// A `MoonButton` cannot be drawn as a bare element: `Button::render` calls `use_keyed_state`,
/// which needs a real rendering view on the stack.
struct ButtonHarness {
    build: Box<dyn Fn() -> MoonButton>,
    bounds: std::rc::Rc<std::cell::RefCell<Vec<gpui::Bounds<gpui::Pixels>>>>,
}

impl gpui::Render for ButtonHarness {
    /// Render the configured button and record its final child bounds.
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        use gpui::{ParentElement as _, Styled as _};

        let sink = self.bounds.clone();
        // Give full-width buttons an independent 200-pixel parent while keeping content-sized
        // children start-aligned so the root itself does not stretch them.
        gpui::div()
            .w(gpui::px(200.0))
            .flex()
            .flex_row()
            .items_start()
            .justify_start()
            .on_children_prepainted(move |bounds, _, _| *sink.borrow_mut() = bounds)
            .child((self.build)().render())
    }
}

/// Lay the built button out in a real window and return its box.
fn laid_out_bounds(
    cx: &mut gpui::TestAppContext,
    build: impl Fn() -> MoonButton + 'static,
) -> gpui::Bounds<gpui::Pixels> {
    use gpui::AppContext as _;

    cx.update(crate::init);
    let bounds = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let sink = bounds.clone();
    let window = cx.add_window(move |_, _| ButtonHarness {
        build: Box::new(build),
        bounds: sink,
    });
    cx.update_window(window.into(), |_, window, cx| {
        window.draw(cx).clear();
    })
    .unwrap();

    let out = bounds.borrow();
    assert_eq!(out.len(), 1, "expected exactly the button as a child");
    out[0]
}

/// Catches emitting an empty segment container in `button.rs:MoonButton::render`, which would add
/// a phantom gap and shift an icon-only glyph away from the center of its square button.
#[gpui::test]
fn icon_only_button_lays_out_square(cx: &mut gpui::TestAppContext) {
    let bounds = laid_out_bounds(cx, || {
        MoonButton::new("icon-only")
            .size(MoonButtonSize::ToolbarCompact)
            .leading_icon(MoonButtonIconSlot::new("icons/settings.svg"))
    });

    assert_eq!(
        bounds.size.width, bounds.size.height,
        "icon-only button is {:?} - not square, so the empty segment container and its \
         phantom gap are still there",
        bounds.size
    );
}

/// Catches attaching a lone trailing icon as a child in `button.rs:MoonButton::render`, which
/// would bypass the icon-only path and render a non-square control.
#[gpui::test]
fn trailing_only_icon_button_lays_out_square(cx: &mut gpui::TestAppContext) {
    let bounds = laid_out_bounds(cx, || {
        MoonButton::new("trailing-only")
            .size(MoonButtonSize::ToolbarCompact)
            .trailing_icon(MoonButtonIconSlot::new("icons/settings.svg"))
    });

    assert_eq!(
        bounds.size.width, bounds.size.height,
        "trailing-only icon button is {:?} - not square, so the icon is still being \
         attached as a child instead of filling the icon slot",
        bounds.size
    );
}

/// Catches promoting a two-icon button into the icon-only branch in
/// `button.rs:MoonButton::render`, which would collapse its width and hide one icon slot.
#[gpui::test]
fn leading_and_trailing_icons_keep_both_slots(cx: &mut gpui::TestAppContext) {
    let bounds = laid_out_bounds(cx, || {
        MoonButton::new("two-icons")
            .size(MoonButtonSize::ToolbarCompact)
            .leading_icon(MoonButtonIconSlot::new("icons/settings.svg"))
            .trailing_icon(MoonButtonIconSlot::new("icons/settings.svg"))
    });

    assert!(
        bounds.size.width > bounds.size.height,
        "two-icon button collapsed to {:?}",
        bounds.size
    );
}

/// Catches promoting a labeled button into the icon-only branch in
/// `button.rs:MoonButton::render`, which would collapse the Settings action and hide its label.
#[gpui::test]
fn icon_with_label_button_stays_wide(cx: &mut gpui::TestAppContext) {
    let bounds = laid_out_bounds(cx, || {
        MoonButton::new("icon-and-label")
            .size(MoonButtonSize::ToolbarCompact)
            .leading_icon(MoonButtonIconSlot::new("icons/settings.svg"))
            .text_segment("Settings", 0xFFFFFF, 500.0)
    });

    assert!(
        bounds.size.width > bounds.size.height,
        "labelled button collapsed to {:?}",
        bounds.size
    );
}

/// Catches `button.rs:MoonButtonIconSlot::rotation` dropping its `rem_euclid(1.0)` fold, which
/// would carry a negative turn value straight to `percentage()` and trip that call's
/// `0.0..=1.0` debug assertion instead of drawing the pose the caller named.
#[test]
fn rotation_folds_a_negative_turn_into_its_positive_pose() {
    let slot = MoonButtonIconSlot::new("icons/settings.svg").rotation(-0.25);

    assert_eq!(
        slot.rotation, 0.75,
        "-0.25 turns is three quarters clockwise, folded positive"
    );
}

/// Catches `button.rs:MoonButtonIconSlot::rotation` dropping its `is_finite` guard, which would
/// leave `f32::NAN` stored. Rendering would then trip `percentage()`'s debug assertion or, in a
/// release build, pass the invalid value into the SVG transformation matrix.
#[test]
fn rotation_drops_a_non_finite_value_to_zero() {
    let slot = MoonButtonIconSlot::new("icons/settings.svg").rotation(f32::NAN);

    assert_eq!(
        slot.rotation, 0.0,
        "a non-finite turn cannot name a pose and must be dropped"
    );
}

/// Catches `button.rs:MoonButtonIconSlot::caret` hard-coding its turns (or inverting `expanded`)
/// instead of delegating to `disclosure::moon_disclosure_rotation_turns`, which would point a
/// `MoonButton` collapse control the opposite way from the `MoonDisclosure` carets beside it in
/// the same window. Expected turns are the pose mapping's own literals, not a call back into the
/// function under test.
#[test]
fn caret_matches_the_shared_disclosure_pose_mapping() {
    let cases = [
        (MoonDisclosureDirection::RightDown, false, 0.75),
        (MoonDisclosureDirection::RightDown, true, 0.0),
        (MoonDisclosureDirection::DownUp, false, 0.0),
        (MoonDisclosureDirection::DownUp, true, 0.5),
    ];

    for (direction, expanded, expected_turns) in cases {
        let slot = MoonButtonIconSlot::caret(direction, expanded);
        assert_eq!(
            slot.rotation, expected_turns,
            "caret({direction:?}, {expanded}) drifted from the shared disclosure pose mapping"
        );
    }
}

// --- MoonSize tier contract (plan-button.md ss7) -------------------------------------------
//
// AUTHOR MODE: `MoonButtonSize::Tier`, `MoonSize::control_metrics`/`nearest`,
// `tier_button_metrics`, and `MoonThemeTokens::scale.tier` do not exist in this tree yet (the
// implementation lands after this Task). These tests are written to the plan's frozen contract
// and will not compile until then - see the AUTHOR report for the full unverified-symbol list.

/// The plan's frozen per-tier table (design-reference px), independent of any function under
/// test: `crates/moon-ui-components/src/moon/foundation.rs:255-272` (`MoonSize::control_metrics`)
/// is the production source this golden data is checked against, not the other way around.
/// `(tier, height, font_size, line_height, radius, pad_x, gap, icon)`.
const TIER_GOLDEN: [(MoonSize, f32, f32, f32, f32, f32, f32, f32); 4] = [
    (MoonSize::Xs, 20.0, 12.0, 16.0, 4.0, 6.0, 4.0, 12.0),
    (MoonSize::Sm, 24.0, 14.0, 20.0, 4.0, 8.0, 8.0, 14.0),
    (MoonSize::Md, 32.0, 16.0, 24.0, 6.0, 12.0, 12.0, 16.0),
    (MoonSize::Lg, 40.0, 18.0, 28.0, 8.0, 16.0, 12.0, 18.0),
];

/// [Breakage 1, band A] Catches `moon/button.rs::tier_button_metrics` drifting from a tier's
/// `control_metrics()` numbers (e.g. shaving Md's radius from 6 to 8), which would draw a wrong
/// corner on every Md button in the app. Checked at ui scale 1.0 and 2.0, against the plan's
/// golden table rather than `control_metrics()` itself, so the comparison is not circular.
#[test]
fn tier_button_metrics_are_control_metrics_scaled_by_ui_only() {
    for ui_scale in [1.0f32, 2.0f32] {
        let mut tokens = MoonThemeTokens::default();
        tokens.scale.ui = ui_scale;
        for (tier, height, font_size, line_height, radius, pad_x, gap, icon) in TIER_GOLDEN {
            let got = tier_button_metrics(tier, &tokens);
            assert_eq!(
                got.height,
                gpui::px(tokens.ui(height)),
                "{tier:?} height drifted at ui={ui_scale}"
            );
            assert_eq!(
                got.font_size,
                gpui::px(tokens.ui(font_size)),
                "{tier:?} font_size drifted at ui={ui_scale}"
            );
            assert_eq!(
                got.line_height,
                gpui::px(tokens.ui(line_height)),
                "{tier:?} line_height drifted at ui={ui_scale}"
            );
            assert_eq!(
                got.radius,
                gpui::px(tokens.ui(radius)),
                "{tier:?} radius drifted at ui={ui_scale}"
            );
            assert_eq!(
                got.pad_x,
                gpui::px(tokens.ui(pad_x)),
                "{tier:?} pad_x drifted at ui={ui_scale}"
            );
            assert_eq!(
                got.gap,
                gpui::px(tokens.ui(gap)),
                "{tier:?} gap drifted at ui={ui_scale}"
            );
            assert_eq!(
                got.icon_size,
                gpui::px(tokens.ui(icon)),
                "{tier:?} icon_size (D1: == font_size) drifted at ui={ui_scale}"
            );
        }
    }
}

/// [Breakage 2, band A] Catches `MoonButton`'s unsized default falling back to a fixed tier
/// instead of `tokens.tier()`, which would make the theme's density setting stop moving buttons
/// at all - the goal's headline behaviour, silently dead.
#[gpui::test]
fn unsized_button_tracks_the_theme_density_tier(cx: &mut gpui::TestAppContext) {
    for (tier, expected_height) in [
        (MoonSize::Xs, 20.0),
        (MoonSize::Sm, 24.0),
        (MoonSize::Md, 32.0),
        (MoonSize::Xl, 40.0), // Xl has no tier of its own; snaps to Lg.
    ] {
        cx.update(|cx| {
            MoonTheme::global_mut(cx).scale.tier = tier;
        });
        let bounds = laid_out_bounds(cx, || MoonButton::new("density").label("Density"));
        assert_eq!(
            bounds.size.height,
            gpui::px(expected_height),
            "density tier {tier:?} did not drive the unsized button's height"
        );
    }
}

/// [Breakage 3, band A] Catches routing a tier's text through `tokens.font()`/`tokens.text()`
/// instead of `tokens.ui()`, which would grow a tier button's text past its own box at the
/// terminal's shipped `font_delta` while `Custom` correctly keeps following it. Height is a
/// direct signal (must stay `ui(24)` either way); text growth is read off the button's own
/// content width, since a wider label is the only way font growth becomes observable here.
#[gpui::test]
fn tier_text_ignores_font_delta_while_custom_still_follows_it(cx: &mut gpui::TestAppContext) {
    let build_tier = || {
        MoonButton::new("tier")
            .label("Sized Label")
            .size(MoonSize::Sm)
    };
    let build_custom = || {
        MoonButton::new("custom")
            .label("Sized Label")
            .size(MoonButtonSize::Custom {
                height: 24.0,
                radius: 4.0,
                font_size: 14.0,
                line_height: 20.0,
                gap: 8.0,
            })
    };

    cx.update(|cx| MoonTheme::global_mut(cx).scale.font_delta = 0.0);
    let tier_before = laid_out_bounds(cx, build_tier);
    let custom_before = laid_out_bounds(cx, build_custom);

    cx.update(|cx| MoonTheme::global_mut(cx).scale.font_delta = 6.0);
    let tier_after = laid_out_bounds(cx, build_tier);
    let custom_after = laid_out_bounds(cx, build_custom);

    assert_eq!(
        tier_after.size.height, tier_before.size.height,
        "a Tier(Sm) button's height must stay ui(24) regardless of font_delta"
    );
    assert_eq!(
        tier_after.size.width, tier_before.size.width,
        "a Tier(Sm) button's text must ignore font_delta, so its content width may not move"
    );
    assert!(
        custom_after.size.width > custom_before.size.width,
        "a Custom button's text must still grow with font_delta, so its content width must move \
         (before={:?}, after={:?})",
        custom_before.size,
        custom_after.size
    );
}

/// [Breakage 4, band B] Catches dropping `Lg` from `MoonButtonSize::SUPPORTED` or breaking
/// `MoonSize::nearest`'s snap, which would silently give large-density users an Md button
/// instead of Lg.
#[test]
fn xl_and_xxl_tiers_snap_down_to_the_largest_supported_tier() {
    assert_eq!(
        MoonButtonSize::SUPPORTED,
        &[MoonSize::Xs, MoonSize::Sm, MoonSize::Md, MoonSize::Lg]
    );
    assert_eq!(
        MoonSize::Xl.nearest(MoonButtonSize::SUPPORTED),
        MoonSize::Lg
    );
    assert_eq!(
        MoonSize::Xxl.nearest(MoonButtonSize::SUPPORTED),
        MoonSize::Lg
    );
    assert_eq!(
        MoonButtonSize::Tier(MoonSize::Xl).snapped(),
        MoonButtonSize::Tier(MoonSize::Lg)
    );
    assert_eq!(
        MoonButtonSize::Tier(MoonSize::Xxl).snapped(),
        MoonButtonSize::Tier(MoonSize::Lg)
    );
}

/// [Breakage 5, band C] Catches repointing a deprecated alias (e.g. `Action` -> `Tier(Md)`),
/// which would jump every un-migrated external caller a tier the instant this ships.
#[test]
#[allow(deprecated)]
fn deprecated_size_aliases_map_to_their_owner_decided_tier() {
    assert_eq!(MoonButtonSize::Micro, MoonButtonSize::Tier(MoonSize::Xs));
    assert_eq!(
        MoonButtonSize::ToolbarCompact,
        MoonButtonSize::Tier(MoonSize::Sm)
    );
    assert_eq!(MoonButtonSize::Action, MoonButtonSize::Tier(MoonSize::Sm));
    assert_eq!(MoonButtonSize::Toolbar, MoonButtonSize::Tier(MoonSize::Md));
    assert_eq!(MoonButtonSize::Pill, MoonButtonSize::Tier(MoonSize::Md));
}

/// [Breakage 7, band B - part 1/2] Catches an icon-only button losing its square aspect at any
/// supported tier.
#[gpui::test]
fn icon_only_button_stays_square_at_every_tier(cx: &mut gpui::TestAppContext) {
    for tier in MoonButtonSize::SUPPORTED.iter().copied() {
        let bounds = laid_out_bounds(cx, move || {
            MoonButton::new("icon-only")
                .size(tier)
                .leading_icon(MoonButtonIconSlot::new("icons/settings.svg"))
        });
        assert_eq!(
            bounds.size.width, bounds.size.height,
            "icon-only button at tier {tier:?} is {:?} - not square",
            bounds.size
        );
    }
}

/// [Breakage 7, band B - part 2/2] Catches a trailing icon slot's explicit `.size()` losing to
/// the tier default, which the contract says must still win for a trailing slot.
#[gpui::test]
fn trailing_icon_explicit_size_still_wins_over_the_tier_default(cx: &mut gpui::TestAppContext) {
    let default_trailing = laid_out_bounds(cx, || {
        MoonButton::new("trailing-default")
            .size(MoonSize::Md)
            .label("Go")
            .trailing_icon(MoonButtonIconSlot::new("icons/settings.svg"))
    });
    let oversized_trailing = laid_out_bounds(cx, || {
        MoonButton::new("trailing-oversized")
            .size(MoonSize::Md)
            .label("Go")
            .trailing_icon(MoonButtonIconSlot::new("icons/settings.svg").size(40.0))
    });

    assert!(
        oversized_trailing.size.width > default_trailing.size.width,
        "an explicit trailing .size(40.0) must widen the button past the tier default \
         (default={:?}, oversized={:?})",
        default_trailing.size,
        oversized_trailing.size
    );
}

/// [Breakage 8, band C] Catches `button_text_metrics` or the post-snap
/// `From<MoonButtonSize> for crate::Size` mapping drifting from the tier table, which would
/// desync select/combobox compat sizing from what the button itself draws. `Tier(Xl)` must snap
/// before crossing into `crate::Size`, or `select.rs`/`combobox` see an out-of-range size.
#[test]
fn tier_compat_surfaces_return_the_tiers_own_numbers() {
    const GOLDEN_SIZE: [(MoonSize, f32, f32, f32, crate::Size); 4] = [
        (MoonSize::Xs, 12.0, 16.0, 4.0, crate::Size::XSmall),
        (MoonSize::Sm, 14.0, 20.0, 8.0, crate::Size::Small),
        (MoonSize::Md, 16.0, 24.0, 12.0, crate::Size::Medium),
        (MoonSize::Lg, 18.0, 28.0, 12.0, crate::Size::Large),
    ];
    for (tier, font_size, line_height, gap, expected_size) in GOLDEN_SIZE {
        assert_eq!(
            button_text_metrics(MoonButtonSize::Tier(tier)),
            (font_size, line_height, gap),
            "button_text_metrics drifted from the tier table at {tier:?}"
        );
        assert_eq!(
            crate::Size::from(MoonButtonSize::Tier(tier)),
            expected_size,
            "From<MoonButtonSize> for Size drifted at {tier:?}"
        );
    }
    assert_eq!(
        crate::Size::from(MoonButtonSize::Tier(MoonSize::Xl)),
        crate::Size::Large,
        "Tier(Xl) must snap before crossing into crate::Size"
    );
}
