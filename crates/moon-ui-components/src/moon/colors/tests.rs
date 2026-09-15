//! Regression coverage for the colour modes and the legacy-palette role mapping.

use gpui::Hsla;

use super::MoonColors;
use crate::moon::{
    theme::MoonThemeTokens,
    tokens::{MoonPalette, MoonTone, contrast_ratio, rgba_from},
};

/// WCAG floor for normal-size text, the bar `docs/PALETTE_SPEC.md` sets for body ink.
const TEXT_CONTRAST_FLOOR: f32 = 4.5;

/// The bundled legacy palettes, by theme name.
const LEGACY_PALETTES: [(&str, MoonPalette); 3] = [
    ("terminal", MoonPalette::TERMINAL),
    ("graphite", MoonPalette::GRAPHITE),
    ("light", MoonPalette::LIGHT),
];

/// Catches a mode's text roles landing on the wrong side of a neutral ramp — `DARK.text_secondary`
/// written as the light mode's step, say. Body, secondary and tertiary text must stay readable on
/// every main background of their own mode.
#[test]
fn text_roles_clear_the_contrast_floor_on_the_main_backgrounds_in_both_modes() {
    for (mode, colors) in [("dark", MoonColors::DARK), ("light", MoonColors::LIGHT)] {
        for (ink_name, ink) in [
            ("text_primary", colors.text_primary),
            ("text_secondary", colors.text_secondary),
            ("text_tertiary", colors.text_tertiary),
        ] {
            for (bg_name, bg) in [
                ("bg_primary", colors.bg_primary),
                ("bg_secondary", colors.bg_secondary),
                ("bg_tertiary", colors.bg_tertiary),
            ] {
                let ratio = contrast_ratio(ink.rgb_hex(), bg.rgb_hex());
                assert!(
                    ratio >= TEXT_CONTRAST_FLOOR,
                    "{mode} {ink_name} #{:06X} on {bg_name} #{:06X} is {ratio:.2}:1, below {TEXT_CONTRAST_FLOOR}:1",
                    ink.rgb_hex(),
                    bg.rgb_hex(),
                );
            }
        }
    }
}

/// Catches `colors.rs:MoonColors::from_palette` resolving a role differently from the colour
/// components paint for the same purpose today — `text_primary_on_brand` taking `accent_fg`, the
/// theme-shaped answer that lands at 1.24:1 on the dark theme's amber button, or the hover and
/// overlay tints picking another field or alpha. Either would make moving a call site from a
/// palette field to its role a visible change. The base-theme bridge, `MoonTone` and `rgba_from`
/// are the reference.
#[test]
fn legacy_roles_match_what_components_paint_today() {
    for (theme, p) in LEGACY_PALETTES {
        let roles = MoonColors::from_palette(p);
        let base = MoonThemeTokens {
            palette: p,
            ..MoonThemeTokens::default()
        }
        .theme_colors();

        let mut pairs: Vec<(String, Hsla, Hsla)> = vec![
            (
                "text_primary".into(),
                roles.text_primary.into(),
                base.foreground,
            ),
            (
                "text_primary_on_brand".into(),
                roles.text_primary_on_brand.into(),
                base.button_primary_foreground,
            ),
            (
                "text_secondary".into(),
                roles.text_secondary.into(),
                base.secondary_foreground,
            ),
            (
                "text_tertiary".into(),
                roles.text_tertiary.into(),
                base.muted_foreground,
            ),
            (
                "text_brand_primary".into(),
                roles.text_brand_primary.into(),
                rgba_from(MoonTone::Accent.color(p), 1.0),
            ),
            (
                "text_error_primary".into(),
                roles.text_error_primary.into(),
                rgba_from(MoonTone::Danger.color(p), 1.0),
            ),
            (
                "text_warning_primary".into(),
                roles.text_warning_primary.into(),
                rgba_from(MoonTone::Warning.color(p), 1.0),
            ),
            (
                "text_success_primary".into(),
                roles.text_success_primary.into(),
                rgba_from(MoonTone::Positive.color(p), 1.0),
            ),
            (
                "border_primary".into(),
                roles.border_primary.into(),
                base.border,
            ),
            (
                "border_brand".into(),
                roles.border_brand.into(),
                base.drag_border,
            ),
            (
                "bg_secondary".into(),
                roles.bg_secondary.into(),
                base.secondary,
            ),
            (
                "bg_secondary_hover".into(),
                roles.bg_secondary_hover.into(),
                base.list_hover,
            ),
            ("bg_tertiary".into(), roles.bg_tertiary.into(), base.muted),
            (
                "bg_quaternary".into(),
                roles.bg_quaternary.into(),
                base.slider_bar,
            ),
            (
                "bg_brand_primary".into(),
                roles.bg_brand_primary.into(),
                base.list_active,
            ),
            (
                "bg_brand_secondary".into(),
                roles.bg_brand_secondary.into(),
                base.drop_target,
            ),
            (
                "bg_brand_solid".into(),
                roles.bg_brand_solid.into(),
                base.button_primary,
            ),
            (
                "bg_brand_solid_hover".into(),
                roles.bg_brand_solid_hover.into(),
                base.button_primary_hover,
            ),
            (
                "bg_error_solid".into(),
                roles.bg_error_solid.into(),
                base.danger,
            ),
            (
                "bg_error_solid_hover".into(),
                roles.bg_error_solid_hover.into(),
                base.danger_hover,
            ),
            (
                "bg_warning_solid".into(),
                roles.bg_warning_solid.into(),
                base.warning,
            ),
            (
                "bg_success_solid".into(),
                roles.bg_success_solid.into(),
                base.success,
            ),
            ("focus_ring".into(), roles.focus_ring.into(), base.ring),
            (
                "slider_handle_bg".into(),
                roles.slider_handle_bg.into(),
                base.slider_thumb,
            ),
        ];
        let overlay_steps = [
            (roles.alpha_white_10, roles.alpha_black_10),
            (roles.alpha_white_20, roles.alpha_black_20),
            (roles.alpha_white_30, roles.alpha_black_30),
            (roles.alpha_white_40, roles.alpha_black_40),
            (roles.alpha_white_50, roles.alpha_black_50),
            (roles.alpha_white_60, roles.alpha_black_60),
            (roles.alpha_white_70, roles.alpha_black_70),
            (roles.alpha_white_80, roles.alpha_black_80),
            (roles.alpha_white_90, roles.alpha_black_90),
            (roles.alpha_white_100, roles.alpha_black_100),
        ];
        for (index, (white, black)) in overlay_steps.into_iter().enumerate() {
            let percent = (index + 1) * 10;
            let alpha = percent as f32 / 100.0;
            pairs.push((
                format!("alpha_white_{percent}"),
                white.into(),
                rgba_from(p.shell, alpha),
            ));
            pairs.push((
                format!("alpha_black_{percent}"),
                black.into(),
                rgba_from(p.overlay, alpha),
            ));
        }

        for (role, resolved, today) in pairs {
            assert_eq!(
                resolved, today,
                "{theme} palette: role {role} does not match the colour painted for it today"
            );
        }
    }
}

/// Catches `colors.rs:MoonColors::from_palette` borrowing hues the legacy palettes lack from the
/// wrong mode. A dark palette given the light mode's utility ramp paints near-white badge fills on
/// a near-black theme, and a light palette given the dark ramp paints near-black ones.
#[test]
fn legacy_palettes_borrow_missing_hues_from_the_mode_of_their_own_side() {
    for (theme, p) in LEGACY_PALETTES {
        let (side_name, side) = if p.is_light() {
            ("light", MoonColors::LIGHT)
        } else {
            ("dark", MoonColors::DARK)
        };
        let roles = MoonColors::from_palette(p);
        for (role, resolved, expected) in [
            (
                "utility_pink_50",
                roles.utility_pink_50,
                side.utility_pink_50,
            ),
            (
                "utility_sky_700",
                roles.utility_sky_700,
                side.utility_sky_700,
            ),
            (
                "utility_slate_100",
                roles.utility_slate_100,
                side.utility_slate_100,
            ),
        ] {
            assert_eq!(
                resolved, expected,
                "{theme} palette: {role} should come from the {side_name} mode"
            );
        }
    }
}
