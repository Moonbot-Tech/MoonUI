//! Guards scale validity and density propagation through theme loading and mode selection.

use super::{MoonScale, MoonTheme, MoonThemeConfig, MoonThemeTokens};
use crate::moon::{
    colors::MoonColors,
    tokens::{MoonPalette, contrast_ratio},
};

/// WCAG floor for normal-size text.
const INK_CONTRAST_FLOOR: f32 = 4.5;

/// Read a theme colour back as `0xRRGGBB` so it can be measured against a palette entry.
fn rgb_of(color: gpui::Hsla) -> u32 {
    let rgba = color.to_rgb();
    let channel = |v: f32| ((v.clamp(0.0, 1.0) * 255.0).round() as u32) & 0xFF;
    (channel(rgba.r) << 16) | (channel(rgba.g) << 8) | channel(rgba.b)
}

/// Catches collapsing the two selection inks in `theme.rs:theme_colors` back into one value.
///
/// They answer different questions. `primary_foreground` / `accent_foreground` are printed on a
/// surface *filled* with the accent — the selected calendar day, a pill tab, a checked stepper —
/// where the dark palette's amber is light and the ink must be dark. `sidebar_accent_foreground`
/// is printed on `selected_background`, an 11% accent tint over the panel, where the ink must stay
/// light. Both directions have shipped broken once; this holds both ends at the same time.
#[test]
fn selection_inks_are_readable_on_the_surface_each_one_lands_on() {
    for (name, palette) in [
        ("dark", MoonPalette::TERMINAL),
        ("graphite", MoonPalette::GRAPHITE),
        ("light", MoonPalette::LIGHT),
        ("dark colour mode", MoonColors::DARK.to_palette()),
        ("light colour mode", MoonColors::LIGHT.to_palette()),
    ] {
        let tokens = MoonThemeTokens {
            palette,
            ..Default::default()
        };
        let colors = tokens.theme_colors();

        for (field, ink) in [
            ("primary_foreground", colors.primary_foreground),
            ("accent_foreground", colors.accent_foreground),
            (
                "button_primary_foreground",
                colors.button_primary_foreground,
            ),
        ] {
            let ratio = contrast_ratio(rgb_of(ink), palette.accent);
            assert!(
                ratio >= INK_CONTRAST_FLOOR,
                "{name}: {field} #{:06X} on the filled accent #{:06X} is {ratio:.2}:1",
                rgb_of(ink),
                palette.accent
            );
        }

        let tinted = rgb_of(colors.sidebar_accent_foreground);
        let ratio = contrast_ratio(tinted, palette.panel);
        assert!(
            ratio >= INK_CONTRAST_FLOOR,
            "{name}: sidebar_accent_foreground #{tinted:06X} on the panel #{:06X} it is tinted over is {ratio:.2}:1",
            palette.panel
        );
    }
}

/// Catches allowing non-positive or non-finite values in
/// `theme.rs:MoonThemeConfig::set_ui_scale`, which would shrink the interface and its hit
/// rectangles until the application appears frozen.
#[test]
fn an_impossible_ui_scale_is_replaced_rather_than_stored() {
    for impossible in [0.0_f32, -1.0, f32::NAN, f32::INFINITY] {
        let cfg = MoonThemeConfig::moon_terminal().with_ui_scale(impossible);

        assert_eq!(
            cfg.dark.scale.ui,
            MoonScale::default().ui,
            "a ui scale of {impossible} cannot be rendered; it must not be stored"
        );
        assert_eq!(
            cfg.light.scale.ui,
            MoonScale::default().ui,
            "both themes must be guarded, not just the dark one"
        );
    }
}

/// Catches clamping positive values in `theme.rs:MoonThemeConfig::set_ui_scale`, which would
/// overwrite a user's deliberate custom scale when the setting is persisted again.
#[test]
fn an_unusual_but_positive_ui_scale_is_stored_verbatim() {
    for kept in [0.25_f32, 0.4, 6.0, 10.0] {
        let cfg = MoonThemeConfig::moon_terminal().with_ui_scale(kept);

        assert_eq!(
            cfg.dark.scale.ui, kept,
            "a positive scale of {kept} is a legitimate choice; the guard is not a clamp"
        );
    }
}

/// Catches accepting non-finite values or rejecting zero in
/// `theme.rs:MoonThemeConfig::set_font_delta`, which would corrupt text metrics or discard the
/// valid "no adjustment" setting.
#[test]
fn a_non_finite_font_delta_is_replaced_while_zero_is_kept() {
    for impossible in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let cfg = MoonThemeConfig::moon_terminal().with_font_delta(impossible);

        assert_eq!(
            cfg.dark.scale.font_delta,
            MoonScale::default().font_delta,
            "a font delta of {impossible} reaches text metrics; it must not be stored"
        );
    }

    let cfg = MoonThemeConfig::moon_terminal().with_font_delta(0.0);
    assert_eq!(
        cfg.dark.scale.font_delta, 0.0,
        "zero font delta is 'no adjustment', a real setting - it must be kept"
    );
}

/// Catches `theme.rs:MoonThemeConfig::moon_graphite` drifting from its bundled TOML palette.
/// A const and its bundled theme file must agree, or Graphite paints a different palette than planned.
#[test]
fn graphite_theme_config_matches_the_bundled_dark_and_light_palettes() {
    let config = MoonThemeConfig::moon_graphite();
    assert_eq!(config.dark.palette, MoonPalette::GRAPHITE);
    assert_eq!(config.light.palette, MoonPalette::LIGHT);
    assert_eq!(config.mode, crate::moon::foundation::ThemeMode::Dark);
}

/// Catches dropping the light-side tier assignment: changing theme must preserve density.
#[test]
fn density_survives_theme_selection_and_scaling() {
    use super::{MoonSize, MoonTheme, ThemeMode};
    for mode in [ThemeMode::Dark, ThemeMode::Light] {
        let mut config = MoonThemeConfig::moon_terminal()
            .with_tier(MoonSize::Sm)
            .with_ui_scale(1.25)
            .with_font_delta(3.0);
        config.mode = mode;
        let theme = MoonTheme::from_config(config);
        assert_eq!(theme.tokens().tier(), MoonSize::Sm);
        assert_eq!(theme.tokens().ui(20.0), 25.0);
        assert_eq!(theme.tokens().font(10.0), 13.0);
    }
}

/// Catches removing serde defaults for the new tier, which would reject existing theme files.
#[test]
fn legacy_theme_toml_uses_medium_density() {
    use super::MoonSize;
    let config: MoonThemeConfig = toml::from_str(
        "[dark.scale]
ui = 1.2
[light.scale]
font = 1.1
",
    )
    .unwrap();
    assert_eq!(config.dark.tier(), MoonSize::Md);
    assert_eq!(config.light.tier(), MoonSize::Md);
    assert_eq!(MoonThemeConfig::default().dark.tier(), MoonSize::Md);
}

/// Catches `MoonThemeTokens::fit_band` changing the legacy Sm/Md expression, which moves
/// Standard or Large chrome even though the compact-density change must leave both unchanged.
#[test]
fn fit_band_keeps_the_existing_expression_outside_xsmall() {
    use super::MoonSize;

    let triples = [
        (32.0, 13.0, 9.5),
        (32.0, 14.0, 9.0),
        (25.0, 14.0, 5.5),
        (26.0, 11.0, 7.5),
        (28.0, 13.0, 7.5),
        (26.0, 14.0, 6.0),
        (20.0, 12.0, 4.0),
    ];
    for (tier, delta) in [(MoonSize::Sm, 3.0), (MoonSize::Md, 6.0)] {
        let tokens = MoonThemeConfig::moon_terminal()
            .with_tier(tier)
            .with_font_delta(delta)
            .dark;
        for (base, line, pad) in triples {
            assert_eq!(
                tokens.fit_band(base, line),
                tokens.fit_height(base, line, pad)
            );
        }
    }
}

/// Catches `MoonThemeTokens::tier_band_base` deriving non-Xs bases from a tier metric, which
/// moves reviewed Standard or Large bands instead of confining the new geometry to Compact.
#[test]
fn tier_band_bases_are_compact_only() {
    use super::MoonSize;

    let xs = MoonThemeConfig::moon_terminal()
        .with_tier(MoonSize::Xs)
        .dark;
    let sm = MoonThemeConfig::moon_terminal()
        .with_tier(MoonSize::Sm)
        .with_font_delta(3.0)
        .dark;
    for (standard, compact) in [
        (25.0, 21.0),
        (26.0, 22.0),
        (32.0, 28.0),
        (28.0, 24.0),
        (20.0, 16.0),
    ] {
        assert_eq!(xs.tier_band_base(standard, |m| m.line_height), compact);
        assert_eq!(sm.tier_band_base(standard, |m| m.line_height), standard);
    }
}

/// Catches allowing non-positive or non-finite values in `theme.rs:MoonThemeConfig::set_zoom`,
/// which would render every window at nothing, the settings screen that could repair it included.
#[test]
fn an_impossible_zoom_is_replaced_rather_than_stored() {
    for impossible in [0.0_f32, -1.0, f32::NAN, f32::INFINITY] {
        let cfg = MoonThemeConfig::moon_terminal().with_zoom(impossible);

        assert_eq!(
            cfg.dark.scale.zoom,
            MoonScale::default().zoom,
            "a zoom of {impossible} cannot be rendered; it must not be stored"
        );
        assert_eq!(
            cfg.light.scale.zoom,
            MoonScale::default().zoom,
            "both themes must be guarded, not just the dark one"
        );
    }
}

/// Catches clamping positive values in `theme.rs:MoonThemeConfig::set_zoom`, which would
/// overwrite a zoom a consumer persisted outside the range of its own slider.
#[test]
fn an_unusual_but_positive_zoom_is_stored_verbatim() {
    for kept in [0.5_f32, 0.8, 1.75, 3.0] {
        let cfg = MoonThemeConfig::moon_terminal().with_zoom(kept);

        assert_eq!(
            cfg.dark.scale.zoom, kept,
            "a positive zoom of {kept} is a legitimate choice; the guard is not a clamp"
        );
        assert_eq!(cfg.light.scale.zoom, kept, "both themes take the zoom");
    }
}

/// Catches a `MoonScale::default` zoom other than one, or a dropped serde default on the new
/// field: every theme file written before it existed would open its windows zoomed.
#[test]
fn legacy_theme_toml_defaults_zoom_to_one() {
    let config: MoonThemeConfig = toml::from_str(
        "[dark.scale]
ui = 1.2
[light.scale]
font = 1.1
",
    )
    .unwrap();
    assert_eq!(config.dark.zoom(), 1.0);
    assert_eq!(config.light.zoom(), 1.0);
    assert_eq!(MoonThemeTokens::default().zoom(), 1.0);
}

/// Catches `theme.rs:MoonTheme::from_config` installing a theme file's impossible zoom: a
/// hand-edited `zoom = 0` bypasses `set_zoom`, every window would refuse it, and the tokens would
/// still report 0 to consumers.
#[test]
fn an_impossible_zoom_in_a_theme_file_is_normalized_on_install() {
    let config: MoonThemeConfig = toml::from_str(
        "[dark.scale]
zoom = 0.0
[light.scale]
zoom = -2.0
",
    )
    .unwrap();
    let theme = MoonTheme::from_config(config);
    assert_eq!(theme.scale.zoom, 1.0);
    assert_eq!(theme.config.light.zoom(), 1.0);
}
