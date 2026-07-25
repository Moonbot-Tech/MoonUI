//! Guards on the scale setters: a value that cannot render must not reach the tokens.

use super::{MoonScale, MoonThemeConfig};

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
