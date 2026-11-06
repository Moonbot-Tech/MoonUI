//! Regression coverage for Moon button sizing and icon-slot rendering.

use super::super::{
    MoonDisclosureDirection, MoonFacetKind, MoonFacetTint, MoonPalette, MoonSize, MoonSkin,
    MoonThemeConfig, facet, facet_tinted, rgba_from,
};
use super::scene_probe::{Painted, Pointer, painted};
use super::{
    MoonButton, MoonButtonIconSlot, MoonButtonSize, MoonButtonVariant, MoonTheme, MoonThemeTokens,
    SOLID_LABEL_CONTRAST_FLOOR, button_text_metrics, contrast_ratio, native_variant, raised_ink,
    solid_colors, tier_button_metrics,
};
use crate::button::{ButtonVariant, ButtonVisualState};

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
            .size(MoonButtonSize::Tier(MoonSize::Sm))
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
            .size(MoonButtonSize::Tier(MoonSize::Sm))
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
            .size(MoonButtonSize::Tier(MoonSize::Sm))
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
            .size(MoonButtonSize::Tier(MoonSize::Sm))
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

/// Blend `fg` over `bg` at `alpha`, computed here independently of any widget helper.
fn blend(fg: u32, bg: u32, alpha: f32) -> u32 {
    let channel = |shift: u32| {
        let f = ((fg >> shift) & 0xFF) as f32;
        let b = ((bg >> shift) & 0xFF) as f32;
        ((f * alpha + b * (1.0 - alpha)).round() as u32) << shift
    };
    channel(16) | channel(8) | channel(0)
}

/// Catches a solid primary button whose label drops under the AA floor in any palette or state —
/// white on the light palette's bright accent measures 2.97:1 — and a fill that drifts off the
/// palette accent. Hover and pressed fills are recomposited here over the panel they sit on.
#[test]
fn solid_label_clears_the_contrast_floor_in_every_palette_and_state() {
    for palette in [
        MoonPalette::TERMINAL,
        MoonPalette::LIGHT,
        MoonPalette::GRAPHITE,
    ] {
        let solid = solid_colors(palette);
        assert_eq!(solid.fill, palette.accent);
        for alpha in [1.0, solid.hover_alpha, solid.active_alpha] {
            let fill = blend(palette.accent, palette.panel, alpha);
            let ratio = contrast_ratio(solid.label, fill);
            assert!(
                ratio >= SOLID_LABEL_CONTRAST_FLOOR,
                "solid label {:06X} on {fill:06X}: {ratio:.2}",
                solid.label
            );
        }
    }
}

/// Catches the solid label ignoring `on_accent` where it is legible: the mockup's primary button
/// is white on the accent, and the dark palettes must not trade it for a body-text ink.
#[test]
fn solid_label_prefers_on_accent_where_it_is_legible() {
    for palette in [
        MoonPalette::TERMINAL,
        MoonPalette::LIGHT,
        MoonPalette::GRAPHITE,
    ] {
        let solid = solid_colors(palette);
        let on_accent_legible =
            contrast_ratio(palette.on_accent, palette.accent) >= SOLID_LABEL_CONTRAST_FLOOR;
        assert_eq!(solid.label == palette.on_accent, on_accent_legible);
    }
}

/// Catches the new variant leaking into the existing palette-free conversion, which every
/// existing button goes through: the old variants must still map exactly as before.
#[test]
fn existing_variants_keep_their_native_mapping() {
    assert_eq!(
        ButtonVariant::from(MoonButtonVariant::Neutral),
        ButtonVariant::Default
    );
    assert_eq!(
        ButtonVariant::from(MoonButtonVariant::Blue),
        ButtonVariant::Blue
    );
    assert_eq!(
        ButtonVariant::from(MoonButtonVariant::Solid),
        ButtonVariant::Primary
    );
}

// --- Facet skin -----------------------------------------------------------------------------

/// The Obsidian theme with its dark side drawn in `skin`: the same palette and scale on both
/// skins, so two renders differ by the skin alone.
fn obsidian_with(skin: MoonSkin) -> MoonThemeConfig {
    let mut config = MoonThemeConfig::moon_obsidian();
    config.dark.skin = skin;
    config
}

/// Catches `button.rs:solid_variant` dropping its facet tone, which would leave the primary
/// button a flat accent slab among the lit controls of the faceted theme, and catches that
/// tone being honoured on a flat theme, where the solid button must not change.
#[gpui::test]
fn solid_button_is_a_solid_accent_facet_on_the_faceted_theme_only(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init);

    cx.update(|cx| MoonTheme::install_config(MoonThemeConfig::moon_obsidian(), cx));
    cx.update(|cx| {
        let tokens = MoonTheme::active_tokens(cx);
        let variant = native_variant(MoonButtonVariant::Solid, cx);
        assert_eq!(
            variant.resolve_facet(&tokens, false, ButtonVisualState::Normal),
            Some(facet_tinted(
                tokens.palette.accent,
                MoonFacetTint::Solid,
                &tokens
            ))
        );
    });

    for flat in [
        MoonThemeConfig::moon_terminal(),
        MoonThemeConfig::moon_light(),
        obsidian_with(MoonSkin::Flat),
    ] {
        cx.update(|cx| MoonTheme::install_config(flat, cx));
        cx.update(|cx| {
            let tokens = MoonTheme::active_tokens(cx);
            let variant = native_variant(MoonButtonVariant::Solid, cx);
            assert_eq!(
                variant.resolve_facet(&tokens, false, ButtonVisualState::Normal),
                None
            );
        });
    }
}

/// Catches the solid label, which is the same ink under both skins, dropping under the AA
/// floor on the faceted fill. That fill runs from the accent lifted a quarter of the way to
/// white down to the accent sunk by 6%; both ends are recomposited here.
#[test]
fn solid_label_clears_the_contrast_floor_on_both_ends_of_the_faceted_fill() {
    let palette = MoonPalette::OBSIDIAN;
    let solid = solid_colors(palette);
    let top = blend(0xFFFFFF, palette.accent, 0.25);
    let bottom = blend(0x000000, palette.accent, 0.06);

    for fill in [top, bottom] {
        let ratio = contrast_ratio(solid.label, fill);
        assert!(
            ratio >= SOLID_LABEL_CONTRAST_FLOOR,
            "solid label {:06X} on faceted {fill:06X}: {ratio:.2}",
            solid.label
        );
    }
}

/// Catches `button.rs:raised_ink` keeping muted ink on a raised facet, where it falls under
/// the AA floor on the lit top of the control, and catches it rewriting any other colour or
/// touching a flat theme, where callers must get exactly the colour they passed.
#[test]
fn muted_ink_on_a_raised_facet_is_lifted_to_the_soft_ink() {
    let palette = MoonPalette::OBSIDIAN;
    let tokens = |skin: MoonSkin| MoonThemeTokens {
        skin,
        palette,
        ..MoonThemeTokens::default()
    };
    let faceted = tokens(MoonSkin::Facet);

    let lifted = raised_ink(palette.text_muted, &faceted);
    assert_eq!(lifted, palette.text_soft);
    // The reason for the rule, measured on the lit top of a raised facet.
    assert!(contrast_ratio(palette.text_muted, palette.panel_high) < 4.5);
    assert!(contrast_ratio(lifted, palette.panel_high) >= 4.5);

    for color in [palette.text, palette.text_soft, palette.accent, palette.red] {
        assert_eq!(raised_ink(color, &faceted), color);
    }
    assert_eq!(
        raised_ink(palette.text_muted, &tokens(MoonSkin::Flat)),
        palette.text_muted
    );
}

/// Builds one button for a probe or a layout harness.
type Build = fn() -> MoonButton;

/// The neutral button every skin test starts from.
fn neutral() -> MoonButton {
    MoonButton::new("b").label("Order")
}

/// Catches `button.rs:Button::render` painting anything new on a flat theme: a facet fill, a
/// facet edge or a shadow where the flat button has none. The Obsidian palette is drawn here
/// with the Flat skin, and every colour is the flat design's own: the dark neutral fill, the
/// palette roles and the alphas, written out rather than read back from the button.
#[test]
fn a_flat_theme_paints_every_button_state_as_it_did_before_the_facet_skin() {
    /// The neutral button's flat fill on a dark palette, which is not a palette entry.
    const NEUTRAL_DARK_FILL: u32 = 0x1F2126;
    let p = MoonPalette::OBSIDIAN;
    let solid = solid_colors(p);
    let flat = |fill: (u32, f32), border: (u32, f32)| {
        Painted::flat(rgba_from(fill.0, fill.1), rgba_from(border.0, border.1))
    };
    let cases: [(&str, Pointer, Build, Painted); 9] = [
        (
            "resting neutral",
            Pointer::Away,
            neutral,
            flat((NEUTRAL_DARK_FILL, 1.0), (p.border, 1.0)),
        ),
        (
            "hovered neutral",
            Pointer::Over,
            neutral,
            flat((p.overlay, 0.10), (p.border_hover, 1.0)),
        ),
        (
            "pressed neutral",
            Pointer::Pressed,
            neutral,
            flat((p.overlay, 0.06), (p.border_hover, 1.0)),
        ),
        (
            "selected neutral",
            Pointer::Away,
            || neutral().selected(true),
            flat((p.accent, p.accent_tint_a), (p.accent, 1.0)),
        ),
        (
            "disabled neutral",
            Pointer::Away,
            || neutral().disabled(true),
            flat((p.panel, 0.32), (p.border, 0.42)),
        ),
        (
            "resting blue",
            Pointer::Away,
            || neutral().primary(),
            flat((p.blue, 0.10), (p.blue, 0.22)),
        ),
        (
            "hovered blue",
            Pointer::Over,
            || neutral().primary(),
            flat((p.blue, 0.18), (p.blue, 0.42)),
        ),
        (
            "resting solid",
            Pointer::Away,
            || neutral().variant(MoonButtonVariant::Solid),
            flat((solid.fill, 1.0), (solid.fill, 1.0)),
        ),
        (
            "hovered solid",
            Pointer::Over,
            || neutral().variant(MoonButtonVariant::Solid),
            flat((solid.fill, solid.hover_alpha), (solid.fill, 1.0)),
        ),
    ];

    let mismatches = paint_mismatches(MoonSkin::Flat, cases);
    assert!(
        mismatches.is_empty(),
        "under the Flat skin:\n{}",
        mismatches.join("\n")
    );
}

/// Draw each case's button under the Obsidian palette with `skin` and describe every one that
/// did not paint what was expected, so one run names all the states that drifted.
fn paint_mismatches<const N: usize>(
    skin: MoonSkin,
    cases: [(&str, Pointer, Build, Painted); N],
) -> Vec<String> {
    cases
        .into_iter()
        .filter_map(|(name, pointer, build, expected)| {
            let got = painted(obsidian_with(skin), pointer, build);
            (got != expected)
                .then(|| format!("the {name} button painted {got:?}, not {expected:?}"))
        })
        .collect()
}

/// Catches `button.rs:Button::render` resolving a state's facet and then not painting it, or
/// painting the facet of another state: a faceted button that stays flat at rest, does not
/// relight under the pointer, or keeps its drop shadow while pressed. Also catches a disabled
/// or ghost button picking up a surface, and a loading one losing its fade.
#[test]
fn the_faceted_theme_paints_each_button_state_with_its_own_facet() {
    let p = MoonPalette::OBSIDIAN;
    let tokens = MoonThemeTokens {
        skin: MoonSkin::Facet,
        palette: p,
        ..MoonThemeTokens::default()
    };
    let raised = facet(MoonFacetKind::Raised, &tokens);
    let top_to_bottom = |top: u32, bottom: u32| {
        gpui::linear_gradient(
            180.0,
            gpui::linear_color_stop(rgba_from(top, 1.0), 0.0),
            gpui::linear_color_stop(rgba_from(bottom, 1.0), 1.0),
        )
    };
    assert_eq!(
        raised.fill,
        top_to_bottom(p.panel_high, p.panel),
        "the resting surface the cases below start from is a gradient"
    );
    let soft_blue = facet_tinted(p.blue, MoonFacetTint::Soft, &tokens);

    let cases: [(&str, Pointer, Build, Painted); 11] = [
        (
            "resting neutral",
            Pointer::Away,
            neutral,
            Painted::facet(raised.clone()),
        ),
        (
            "hovered neutral",
            Pointer::Over,
            neutral,
            Painted::facet(facet(MoonFacetKind::RaisedHover, &tokens)),
        ),
        (
            "pressed neutral",
            Pointer::Pressed,
            neutral,
            Painted {
                fill: Some(top_to_bottom(p.panel, p.panel_high)),
                border: Some(raised.edge),
                shadows: raised
                    .shadows
                    .iter()
                    .filter(|shadow| shadow.inset)
                    .cloned()
                    .collect(),
            },
        ),
        (
            "selected neutral",
            Pointer::Away,
            || neutral().selected(true),
            Painted::facet(facet_tinted(p.accent, MoonFacetTint::Selected, &tokens)),
        ),
        (
            "disabled neutral",
            Pointer::Away,
            || neutral().disabled(true),
            Painted::flat(rgba_from(p.panel, 0.32), rgba_from(p.border, 0.42)),
        ),
        (
            "resting blue",
            Pointer::Away,
            || neutral().primary(),
            Painted::facet(soft_blue.clone()),
        ),
        (
            "hovered blue",
            Pointer::Over,
            || neutral().primary(),
            Painted::facet(facet_tinted(p.blue, MoonFacetTint::Selected, &tokens)),
        ),
        (
            "loading blue",
            Pointer::Away,
            || neutral().primary().loading(true),
            Painted {
                fill: Some(soft_blue.fill.opacity(0.8)),
                border: Some(soft_blue.edge.opacity(0.8)),
                shadows: soft_blue.shadows.clone(),
            },
        ),
        (
            "resting danger",
            Pointer::Away,
            || neutral().danger(),
            Painted::facet(facet_tinted(p.red, MoonFacetTint::Soft, &tokens)),
        ),
        (
            "resting solid",
            Pointer::Away,
            || neutral().variant(MoonButtonVariant::Solid),
            Painted::facet(facet_tinted(p.accent, MoonFacetTint::Solid, &tokens)),
        ),
        (
            "hovered ghost",
            Pointer::Over,
            || neutral().ghost(),
            Painted::flat(rgba_from(p.overlay, 0.10), rgba_from(p.border_hover, 1.0)),
        ),
    ];

    let mismatches = paint_mismatches(MoonSkin::Facet, cases);
    assert!(
        mismatches.is_empty(),
        "under the Facet skin:\n{}",
        mismatches.join("\n")
    );
}

/// Catches the Facet skin changing a button's box, by adding a border, padding or a second
/// element, and catches a render path that only the faceted theme takes panicking: the same
/// button must occupy the same pixels on a flat theme and on the faceted one.
#[gpui::test]
fn the_facet_skin_leaves_every_button_box_unchanged(cx: &mut gpui::TestAppContext) {
    let builds: [(&str, Build); 9] = [
        ("neutral", || MoonButton::new("b").label("Order")),
        ("panel", || {
            MoonButton::new("b")
                .label("Order")
                .variant(MoonButtonVariant::Panel)
        }),
        ("blue", || MoonButton::new("b").label("Order").primary()),
        ("danger", || MoonButton::new("b").label("Order").danger()),
        ("solid", || {
            MoonButton::new("b")
                .label("Order")
                .variant(MoonButtonVariant::Solid)
        }),
        ("ghost", || MoonButton::new("b").label("Order").ghost()),
        ("selected neutral", || {
            MoonButton::new("b").label("Order").selected(true)
        }),
        ("disabled neutral", || {
            MoonButton::new("b").label("Order").disabled(true)
        }),
        ("loading blue", || {
            MoonButton::new("b").label("Order").primary().loading(true)
        }),
    ];

    cx.update(crate::init);
    for (name, build) in builds {
        cx.update(|cx| MoonTheme::install_config(obsidian_with(MoonSkin::Flat), cx));
        let flat = laid_out_bounds(cx, build);
        cx.update(|cx| MoonTheme::install_config(obsidian_with(MoonSkin::Facet), cx));
        let faceted = laid_out_bounds(cx, build);

        assert_eq!(
            faceted, flat,
            "the {name} button moved under the Facet skin"
        );
    }
}
