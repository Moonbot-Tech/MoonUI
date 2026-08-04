//! Guards on the scale setters: a value that cannot render must not reach the tokens.

use super::{MoonScale, MoonThemeConfig, MoonThemeTokens};
use crate::moon::tokens::{MoonPalette, contrast_ratio};

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
        ("light", MoonPalette::LIGHT),
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
