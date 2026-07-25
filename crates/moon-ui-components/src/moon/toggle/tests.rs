//! Regression coverage for MoonToggle geometry, interaction, and light-theme colors.

use super::{MoonToggle, MoonToggleSize, moon_toggle_click_plan, toggle_colors};
use crate::moon::MoonPalette;

/// Catches changing the compact or normal track dimensions in
/// `toggle.rs:MoonToggle::metrics` away from the reviewed designer reference, which would resize
/// the rendered switch unexpectedly.
#[test]
fn toggle_metrics_match_designer_reference() {
    let compact = MoonToggle::new("compact").size(MoonToggleSize::Compact);
    assert_eq!(compact.metrics().track_width, 28.0);
    assert_eq!(compact.metrics().track_height, 16.0);
    let normal = MoonToggle::new("normal");
    assert_eq!(normal.metrics().track_width, 36.0);
    assert_eq!(normal.metrics().track_height, 20.0);
}

/// Catches removing disabled handling or controlled-state ownership from
/// `toggle.rs:moon_toggle_click_plan`, which would let disabled toggles change or mutate internal
/// state behind a controlled value.
#[test]
fn toggle_click_plan_respects_disabled_and_controlled_state() {
    assert_eq!(moon_toggle_click_plan(false, false, true), None);

    let uncontrolled = moon_toggle_click_plan(false, false, false).unwrap();
    assert!(uncontrolled.next_checked);
    assert!(uncontrolled.update_internal);

    let controlled = moon_toggle_click_plan(true, true, false).unwrap();
    assert!(!controlled.next_checked);
    assert!(!controlled.update_internal);
}

/// Catches replacing the light/off branch in `toggle.rs:toggle_colors` with generic text roles or
/// dark-theme shadow strength, which would restore a harsh knob instead of the reviewed soft blue
/// treatment.
#[test]
fn light_toggle_uses_soft_knob_when_off() {
    let p = MoonPalette::LIGHT;
    let off = toggle_colors(p, p.accent, false);
    assert_eq!(off.track, 0xEEF9FF);
    assert_eq!(off.border, 0xC5DEEC);
    assert_eq!(off.thumb, 0x6AA6C8);
    assert_ne!(off.thumb, p.text);
    assert_ne!(off.thumb, p.text_soft);
    assert!(off.shadow_alpha < 0.20);

    let on = toggle_colors(p, p.accent, true);
    assert_eq!(on.thumb, p.surface);
    assert!(on.shadow_alpha < 0.20);
}
