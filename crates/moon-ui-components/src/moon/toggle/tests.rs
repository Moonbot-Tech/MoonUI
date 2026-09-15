//! Regression coverage for MoonToggle geometry, interaction, and light-theme colors.

use super::{MoonToggle, MoonToggleSize, moon_toggle_click_plan, toggle_colors};
use crate::moon::{MoonPalette, MoonScale, MoonSize, MoonThemeConfig, MoonThemeTokens};

/// Catches changing a tier's unscaled reference geometry in `MoonToggleSize::reference_metrics`
/// away from the reviewed designer reference (Sm 28x16 track/12 thumb/14px-20 text/8 gap, Md
/// 36x20/16/16-24/12), which would resize the rendered switch and its label unexpectedly.
#[test]
fn toggle_metrics_match_designer_reference() {
    let sm = MoonToggleSize::Tier(MoonSize::Sm).reference_metrics();
    assert_eq!(sm.track_width, 28.0);
    assert_eq!(sm.track_height, 16.0);
    assert_eq!(sm.thumb_size, 12.0);
    assert_eq!(sm.font_size, 14.0);
    assert_eq!(sm.line_height, 20.0);
    assert_eq!(sm.gap, 8.0);

    let md = MoonToggleSize::Tier(MoonSize::Md).reference_metrics();
    assert_eq!(md.track_width, 36.0);
    assert_eq!(md.track_height, 20.0);
    assert_eq!(md.thumb_size, 16.0);
    assert_eq!(md.font_size, 16.0);
    assert_eq!(md.line_height, 24.0);
    assert_eq!(md.gap, 12.0);
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

/// Breakage 1 (band B) -- `MoonToggleSize::resolve` / `MoonToggleMetrics::zoomed`: a Tier's font
/// size must follow `tokens.ui()` only. A plausible future edit routes it through `tokens.font()`
/// (or reinstates the old `MoonText` path) so tier labels "honour the font setting" too; at the
/// terminal's Standard density (`font_delta = 3`) that would desync every tier toggle's label from
/// the checkbox and the toolbar's measured cluster widths by ~3px.
#[test]
fn tier_font_follows_ui_zoom_not_font_scale() {
    let tokens = MoonThemeConfig::moon_terminal().with_font_delta(3.0).dark;

    let sm = MoonToggleSize::Tier(MoonSize::Sm).resolve(&tokens);
    assert_eq!(sm.font_size, tokens.ui(14.0));
    assert_ne!(sm.font_size, tokens.font(14.0));

    let md = MoonToggleSize::Tier(MoonSize::Md).resolve(&tokens);
    assert_eq!(md.font_size, tokens.ui(16.0));
    assert_ne!(md.font_size, tokens.font(16.0));
}

/// Breakage 2 (band B) -- `size: None` must resolve to
/// `Tier(tokens.tier().nearest(&MoonToggleSize::SUPPORTED_TIERS))`, snapping every unsupported tier
/// down to `Sm` or up to `Md`. A plausible future edit hardcodes the default as a constant
/// `Tier(Md)` in `new()`; that reads identically at `Md` but freezes the app's density setting
/// everywhere else -- Large would silently render Sm, and the terminal's Compact density (tier
/// `Xs`) would reach an unsupported tier.
///
/// `tier` lives on `MoonThemeTokens.scale.tier` (`MoonThemeTokens::tier()` reads it back); only
/// `tokens.tier()` and the snap need to hold, not this exact construction.
#[test]
fn density_default_snaps_every_tier_to_a_supported_one() {
    for (tier, want) in [
        (MoonSize::Xs, MoonSize::Sm),
        (MoonSize::Sm, MoonSize::Sm),
        (MoonSize::Md, MoonSize::Md),
        (MoonSize::Lg, MoonSize::Md),
        (MoonSize::Xl, MoonSize::Md),
        (MoonSize::Xxl, MoonSize::Md),
    ] {
        let tokens = MoonThemeTokens {
            scale: MoonScale {
                tier,
                ..Default::default()
            },
            ..Default::default()
        };
        assert_eq!(
            MoonToggleSize::density_default(&tokens),
            MoonToggleSize::Tier(want),
        );
    }
}

/// Breakage 3 (band B) -- `Custom` must keep following the theme's text scaling
/// (`tokens.font()` / `tokens.line_height()`) exactly as before, while `Tier` must not: unifying
/// both onto `zoomed` for simplicity would silently stop every existing `Custom` caller's text
/// from following the user's font setting.
#[test]
fn custom_keeps_text_scaling_while_tier_does_not() {
    let tokens = MoonThemeConfig::moon_terminal().with_font_delta(3.0).dark;
    let custom = MoonToggleSize::Custom {
        track_width: 36.0,
        track_height: 20.0,
        thumb_size: 16.0,
        font_size: 14.0,
        line_height: 20.0,
        gap: 8.0,
    };

    let resolved = custom.resolve(&tokens);
    assert_eq!(resolved.font_size, tokens.font(14.0));
    assert_eq!(resolved.line_height, tokens.line_height(20.0));
    assert_ne!(resolved.font_size, tokens.ui(14.0));

    let tier = MoonToggleSize::Tier(MoonSize::Sm).resolve(&tokens);
    assert_eq!(tier.line_height, tokens.ui(20.0));
    assert_ne!(tier.line_height, tokens.line_height(20.0));
}

/// Breakage 4 (band C) -- the focus ring is an absolute overlay that must never move layout; its
/// rounded radius follows the track radius, offset outward by `focus_ring_distance`. A plausible
/// future edit drops `.absolute()` or moves the overlay out of the relative switch div while
/// tidying the render, which would reflow the header strip and the toolbar row on focus.
#[test]
fn focus_ring_geometry_tracks_the_switch_radius() {
    let sm = MoonToggleSize::Tier(MoonSize::Sm).reference_metrics();
    assert_eq!(sm.focus_ring_distance, 4.0);
    assert_eq!(sm.focus_ring_width, 2.0);
    assert_eq!(sm.track_height * 0.5 + sm.focus_ring_distance, 12.0); // 16 / 2 + 4

    let md = MoonToggleSize::Tier(MoonSize::Md).reference_metrics();
    assert_eq!(md.track_height * 0.5 + md.focus_ring_distance, 14.0); // 20 / 2 + 4
}

struct ToggleHarness;

impl gpui::Render for ToggleHarness {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        _: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        use gpui::ParentElement as _;

        gpui::div().child(MoonToggle::new("probe"))
    }
}

/// Breakage 5 (band C) -- clicking an enabled toggle must call `focus_handle.focus(window, cx)`
/// explicitly inside `on_mouse_down`. GPUI's auto-focus-on-press listener never fires here because
/// the same handler calls `stop_propagation` + `prevent_default` (needed so the terminal's header
/// keeps working), which is enough for the checkbox but silently swallows focus for the toggle. A
/// plausible future edit deletes the explicit call believing `track_focus` alone is enough; the
/// consequence is silent -- nothing crashes, but the ring never appears on click and keyboard users
/// lose the tab anchor.
#[gpui::test]
fn click_focuses_toggle_so_the_ring_appears(cx: &mut gpui::TestAppContext) {
    use gpui::AppContext as _;

    cx.update(crate::init);
    let window = cx.add_window(|window, cx| {
        let view = cx.new(|_| ToggleHarness);
        crate::Root::new(view, window, cx).bordered(false)
    });
    let mut cx = gpui::VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();
    assert!(cx.debug_bounds("probe:focus-ring").is_none());

    let track = cx.debug_bounds("probe:track").expect("track must render");
    cx.simulate_click(track.center(), gpui::Modifiers::none());
    cx.run_until_parked();

    assert!(
        cx.debug_bounds("probe:focus-ring").is_some(),
        "click must explicitly focus the toggle so the ring renders"
    );
}
