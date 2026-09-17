//! Regression coverage for MoonToggle geometry, interaction, and role-driven colours.

use super::{
    MoonToggle, MoonToggleLabelSide, MoonToggleSize, MoonToggleVariant, THUMB_TRAVEL, ThumbTravel,
    ToggleColors, moon_toggle_click_plan,
};
use crate::checkbox::{ChoiceColors, MoonCheckboxMetrics};
use crate::moon::checkbox::tier_size;
use crate::moon::{
    MoonColors, MoonPalette, MoonScale, MoonSize, MoonThemeConfig, MoonThemeTokens, MoonTone,
    rgba_from,
};
use gpui::{px, size};

/// Catches changing a tier's unscaled reference geometry in `MoonToggleSize::reference_metrics`
/// away from the reviewed designer reference (Sm 36x20 track/16 thumb/14px-20 text/8 gap, Md
/// 44x24/20/16-24/12), which would resize the rendered switch and its label unexpectedly.
#[test]
fn toggle_metrics_match_designer_reference() {
    let sm = MoonToggleSize::Tier(MoonSize::Sm).reference_metrics();
    assert_eq!(sm.track_width, 36.0);
    assert_eq!(sm.track_height, 20.0);
    assert_eq!(sm.thumb_size, 16.0);
    assert_eq!(sm.font_size, 14.0);
    assert_eq!(sm.line_height, 20.0);
    assert_eq!(sm.gap, 8.0);

    let md = MoonToggleSize::Tier(MoonSize::Md).reference_metrics();
    assert_eq!(md.track_width, 44.0);
    assert_eq!(md.track_height, 24.0);
    assert_eq!(md.thumb_size, 20.0);
    assert_eq!(md.font_size, 16.0);
    assert_eq!(md.line_height, 24.0);
    assert_eq!(md.gap, 12.0);
}

/// Catches `moon/toggle.rs:MoonToggleSize::reference_metrics` breaking the rule its tiers are
/// drawn to, which mis-centres a toggle's thumb or lifts its track off the line of text beside it:
/// a default track is exactly one text line tall, and its thumb sits 2px in from each edge.
#[test]
fn toggle_tiers_keep_the_two_pixel_inset_on_each_side() {
    for tier in MoonToggleSize::SUPPORTED_TIERS {
        let metrics = MoonToggleSize::Tier(tier).reference_metrics();
        assert_eq!(metrics.track_height, metrics.line_height, "{tier:?}");
        assert_eq!(metrics.thumb_size, metrics.track_height - 4.0, "{tier:?}");
    }
}

/// Catches a toggle tier's text drifting from the checkbox and radio of the same tier: the label
/// and supporting text must share the checkbox's font size, line height, weights, track-to-text
/// gap and label-to-description gap, at any UI zoom, so a toggle's label lines up with a
/// checkbox's in the same form.
#[test]
fn tier_text_matches_the_checkbox_of_the_same_tier() {
    let tokens = MoonThemeConfig::moon_terminal()
        .with_font_delta(3.0)
        .with_ui_scale(1.5)
        .dark;
    for tier in MoonToggleSize::SUPPORTED_TIERS {
        let toggle = MoonToggleSize::Tier(tier)
            .resolve(MoonToggleVariant::Default, &tokens)
            .choice();
        let checkbox = MoonCheckboxMetrics::resolve(tier_size(tier), &tokens);
        assert_eq!(toggle.font_size, checkbox.font_size, "{tier:?}");
        assert_eq!(toggle.line_height, checkbox.line_height, "{tier:?}");
        assert_eq!(toggle.label_weight, checkbox.label_weight, "{tier:?}");
        assert_eq!(
            toggle.description_weight, checkbox.description_weight,
            "{tier:?}"
        );
        assert_eq!(toggle.gap, checkbox.gap, "{tier:?}");
        assert_eq!(toggle.description_gap, checkbox.description_gap, "{tier:?}");
    }
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

/// Catches a toggle drifting from the reviewed design, which reads its colours from the roles
/// rather than the palette: an unchecked track is `bg_tertiary` behind a `border_secondary`
/// outline; a checked one is `bg_brand_solid`, `bg_brand_solid_hover` under the pointer, and draws
/// no outline at all; and the thumb is `fg_white` in both states. Restoring the old hard-coded
/// light-theme blues, outlining a checked track, or painting the thumb from a text ink would each
/// undo a reviewed decision. Every palette is paired with the other side's roles, so a colour read
/// from the palette instead of the roles cannot pass by coinciding.
#[test]
fn toggle_colors_follow_the_colour_roles() {
    for (p, roles) in [
        (MoonPalette::TERMINAL, MoonColors::LIGHT),
        (MoonPalette::LIGHT, MoonColors::DARK),
    ] {
        let off = ToggleColors::resolve(p, roles, MoonToggleVariant::Default, None, false, false);
        assert_eq!(off.track, roles.bg_tertiary.into());
        assert_eq!(off.track_hover, off.track);
        assert_eq!(off.border, roles.border_secondary.into());
        assert_eq!(off.thumb, roles.fg_white.into());

        let on = ToggleColors::resolve(p, roles, MoonToggleVariant::Default, None, true, false);
        assert_eq!(on.track, roles.bg_brand_solid.into());
        assert_eq!(on.track_hover, roles.bg_brand_solid_hover.into());
        assert!(on.border.is_transparent());
        assert_eq!(on.thumb, off.thumb);
    }
}

/// Catches an explicit tone losing its hold on a checked track: `tone` must fill it with that tone
/// instead of the brand colour, and keep that fill under the pointer, while an unchecked track and
/// the thumb stay on the roles whatever the tone. A plausible future edit drops the tone when the
/// brand fill lands, which silently repaints every toned toggle in the brand colour.
#[test]
fn an_explicit_tone_fills_a_checked_track_instead_of_the_brand() {
    let p = MoonPalette::TERMINAL;
    let roles = MoonColors::DARK;
    let warning = rgba_from(MoonTone::Warning.color(p), 1.0);

    let on = ToggleColors::resolve(
        p,
        roles,
        MoonToggleVariant::Default,
        Some(MoonTone::Warning),
        true,
        false,
    );
    assert_eq!(on.track, warning);
    assert_eq!(on.track_hover, warning);
    assert!(on.border.is_transparent());
    assert_eq!(on.thumb, roles.fg_white.into());

    let off = ToggleColors::resolve(
        p,
        roles,
        MoonToggleVariant::Default,
        Some(MoonTone::Warning),
        false,
        false,
    );
    assert_eq!(off.track, roles.bg_tertiary.into());
    assert_eq!(off.border, roles.border_secondary.into());
}

/// Catches a disabled toggle going back to painting its colours translucent. GPUI fades each
/// element on its own rather than the control as a whole, so a translucent thumb shows the track
/// through it and reads as a tint of the track instead of a faded white. Every colour is mixed
/// halfway into `bg_primary` and stays opaque instead, which is what a control at half opacity
/// looks like. A colour that paints nothing must stay transparent, or an outline the variant leaves
/// off would come back as a surface-coloured ring. The label and supporting text are not part of
/// this: they carry the choice roles' own disabled colours, as a checkbox's do.
#[test]
fn a_disabled_toggle_mixes_its_colours_into_the_surface_rather_than_fading_them() {
    let p = MoonPalette::TERMINAL;
    let roles = MoonColors::DARK;
    let surface: gpui::Hsla = roles.bg_primary.into();
    let halfway = |color: gpui::Hsla| {
        let faded = gpui::Rgba::from(color).alpha(color.a * 0.5);
        gpui::Hsla::from(gpui::Rgba::from(surface).blend(faded))
    };

    for variant in [MoonToggleVariant::Default, MoonToggleVariant::Slim] {
        for checked in [false, true] {
            let enabled = ToggleColors::resolve(p, roles, variant, None, checked, false);
            let disabled = ToggleColors::resolve(p, roles, variant, None, checked, true);

            for (enabled, disabled) in [
                (enabled.track, disabled.track),
                (enabled.track_hover, disabled.track_hover),
                (enabled.border, disabled.border),
                (enabled.thumb, disabled.thumb),
                (enabled.thumb_border, disabled.thumb_border),
            ] {
                if enabled.a == 0.0 {
                    assert!(
                        disabled.is_transparent(),
                        "{variant:?} checked {checked}: a colour that paints nothing must stay so"
                    );
                } else {
                    assert_eq!(
                        disabled,
                        halfway(enabled),
                        "{variant:?} checked {checked}: every colour is mixed halfway into the surface"
                    );
                    assert_eq!(disabled.a, 1.0, "{variant:?} and stays opaque");
                }
            }
        }

        // The text comes from the shared choice colours, which dim it on their own.
        let text = ChoiceColors::resolve(p, roles, None, true, true);
        assert_ne!(
            text.label,
            ChoiceColors::resolve(p, roles, None, true, false).label
        );
    }
}

/// Breakage 1 (band B) -- `MoonToggleSize::resolve` / `MoonToggleMetrics::zoomed`: a Tier's font
/// size must follow `tokens.ui()` only. A plausible future edit routes it through `tokens.font()`
/// (or reinstates the old `MoonText` path) so tier labels "honour the font setting" too; at the
/// terminal's Standard density (`font_delta = 3`) that would desync every tier toggle's label from
/// the checkbox and the toolbar's measured cluster widths by ~3px.
#[test]
fn tier_font_follows_ui_zoom_not_font_scale() {
    let tokens = MoonThemeConfig::moon_terminal().with_font_delta(3.0).dark;

    let sm = MoonToggleSize::Tier(MoonSize::Sm).resolve(MoonToggleVariant::Default, &tokens);
    assert_eq!(sm.font_size, tokens.ui(14.0));
    assert_ne!(sm.font_size, tokens.font(14.0));

    let md = MoonToggleSize::Tier(MoonSize::Md).resolve(MoonToggleVariant::Default, &tokens);
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

    let resolved = custom.resolve(MoonToggleVariant::Default, &tokens);
    assert_eq!(resolved.font_size, tokens.font(14.0));
    assert_eq!(resolved.line_height, tokens.line_height(20.0));
    assert_ne!(resolved.font_size, tokens.ui(14.0));

    let tier = MoonToggleSize::Tier(MoonSize::Sm).resolve(MoonToggleVariant::Default, &tokens);
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
    assert_eq!(sm.track_height * 0.5 + sm.focus_ring_distance, 14.0); // 20 / 2 + 4

    let md = MoonToggleSize::Tier(MoonSize::Md).reference_metrics();
    assert_eq!(md.track_height * 0.5 + md.focus_ring_distance, 16.0); // 24 / 2 + 4
}

/// Width of the column a stretched `SizedToggleHarness` spans, wider than any of its toggles.
const STRETCHED_WIDTH: f32 = 320.;

struct SizedToggleHarness {
    size: MoonSize,
    variant: MoonToggleVariant,
    checked: bool,
    label: Option<&'static str>,
    description: Option<&'static str>,
    label_side: MoonToggleLabelSide,
    /// Stretches the toggle across a `STRETCHED_WIDTH` column instead of shrinking it to its
    /// content.
    stretched: bool,
}

impl SizedToggleHarness {
    fn bare(size: MoonSize, checked: bool) -> Self {
        Self {
            size,
            variant: MoonToggleVariant::Default,
            checked,
            label: None,
            description: None,
            label_side: MoonToggleLabelSide::Right,
            stretched: false,
        }
    }
}

impl gpui::Render for SizedToggleHarness {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        _: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        use gpui::{InteractiveElement as _, IntoElement as _, ParentElement as _, Styled as _};

        let mut toggle = MoonToggle::new("sized")
            .size(self.size)
            .variant(self.variant)
            .checked(self.checked)
            .label_side(self.label_side);
        if let Some(label) = self.label {
            toggle = toggle.label(label);
        }
        if let Some(description) = self.description {
            toggle = toggle.description(description);
        }
        if self.stretched {
            crate::v_flex()
                .w(px(STRETCHED_WIDTH))
                .child(toggle)
                .into_any_element()
        } else {
            // A flex row shrinks the probe to its content, so the probe measures the toggle.
            crate::h_flex()
                .child(
                    gpui::div()
                        .debug_selector(|| "sized-control".into())
                        .child(toggle),
                )
                .into_any_element()
        }
    }
}

fn render_toggle(
    cx: &mut gpui::TestAppContext,
    harness: SizedToggleHarness,
) -> gpui::VisualTestContext {
    let window = cx.add_window(move |_, _| harness);
    let cx = gpui::VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();
    cx
}

/// Catches the rendered track or thumb drifting from the reviewed geometry: a Sm toggle is a 36x20
/// track with a 16px thumb and a Md toggle a 44x24 track with a 20px thumb, and the thumb sits 2px
/// in from the track's outer edge on the top, the bottom and the side it rests against. Measuring
/// the thumb's insets from inside the track's 1px border would push it 1px down and towards the
/// right, off centre in both states.
#[gpui::test]
fn track_and_thumb_render_at_the_reviewed_size(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init);
    // A default toggle keeps its thumb 2px inside the track; a slim one's thumb is the track's
    // own height, so it sits flush against every edge.
    for (variant, tier, track, thumb, gap) in [
        (
            MoonToggleVariant::Default,
            MoonSize::Sm,
            (36., 20.),
            16.,
            2.,
        ),
        (
            MoonToggleVariant::Default,
            MoonSize::Md,
            (44., 24.),
            20.,
            2.,
        ),
        (MoonToggleVariant::Slim, MoonSize::Sm, (32., 16.), 16., 0.),
        (MoonToggleVariant::Slim, MoonSize::Md, (40., 20.), 20., 0.),
    ] {
        for checked in [false, true] {
            let mut cx = render_toggle(
                cx,
                SizedToggleHarness {
                    variant,
                    ..SizedToggleHarness::bare(tier, checked)
                },
            );
            let control = cx.debug_bounds("sized-control").expect("probe must render");
            let track_bounds = cx.debug_bounds("sized:track").expect("track must render");
            let thumb_bounds = cx.debug_bounds("sized:thumb").expect("thumb must render");

            assert_eq!(
                control.size,
                size(px(track.0), px(track.1)),
                "{variant:?} {tier:?}"
            );
            assert_eq!(
                track_bounds.size,
                size(px(track.0), px(track.1)),
                "{variant:?} {tier:?}"
            );
            assert_eq!(
                thumb_bounds.size,
                size(px(thumb), px(thumb)),
                "{variant:?} {tier:?}"
            );
            assert_eq!(
                thumb_bounds.top() - track_bounds.top(),
                px(gap),
                "{variant:?} {tier:?}"
            );
            assert_eq!(
                track_bounds.bottom() - thumb_bounds.bottom(),
                px(gap),
                "{variant:?} {tier:?}"
            );
            if checked {
                assert_eq!(
                    track_bounds.right() - thumb_bounds.right(),
                    px(gap),
                    "{variant:?} {tier:?}"
                );
            } else {
                assert_eq!(
                    thumb_bounds.left() - track_bounds.left(),
                    px(gap),
                    "{variant:?} {tier:?}"
                );
            }
        }
    }
}

/// Catches a toggle's text leaving the checkbox's layout: the label must sit on one text line
/// (20px Sm, 24px Md) centred on the track, the track-to-text gap must be 8px or 12px, and the
/// supporting text must stack under the label at 0px or 2px, on either side of the track. The
/// toggle is stretched across a wider column, where a text column that fills the row would push the
/// track away from a label on its left.
#[gpui::test]
fn label_and_description_lay_out_like_a_checkbox(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init);
    for (tier, line, gap, description_gap) in
        [(MoonSize::Sm, 20., 8., 0.), (MoonSize::Md, 24., 12., 2.)]
    {
        for label_side in [MoonToggleLabelSide::Right, MoonToggleLabelSide::Left] {
            let mut cx = render_toggle(
                cx,
                SizedToggleHarness {
                    label: Some("Overlay hints"),
                    description: Some("Show hints over the chart"),
                    label_side,
                    stretched: true,
                    ..SizedToggleHarness::bare(tier, false)
                },
            );
            let track = cx.debug_bounds("sized:track").expect("track must render");
            let label = cx.debug_bounds("sized:label").expect("label must render");
            let description = cx
                .debug_bounds("sized:description")
                .expect("description must render");

            assert_eq!(label.size.height, px(line), "{tier:?}");
            assert_eq!(label.center().y, track.center().y, "{tier:?}");
            assert_eq!(
                description.top() - label.bottom(),
                px(description_gap),
                "{tier:?}"
            );
            assert_eq!(description.left(), label.left(), "{tier:?}");
            if label_side == MoonToggleLabelSide::Left {
                assert!(
                    track.right() < px(STRETCHED_WIDTH),
                    "{tier:?} track must follow its label, not the end of the row"
                );
            }
            let text_gap = match label_side {
                MoonToggleLabelSide::Right => label.left() - track.right(),
                MoonToggleLabelSide::Left => track.left() - label.right().max(description.right()),
            };
            assert_eq!(text_gap, px(gap), "{tier:?} {label_side:?}");
        }
    }
}

/// Catches a toggle reserving text space for text it does not show: an empty label must render
/// no text column and no gap, so the toggle stays exactly its track.
#[gpui::test]
fn toggle_with_empty_label_renders_only_its_track(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init);
    let mut cx = render_toggle(
        cx,
        SizedToggleHarness {
            label: Some(""),
            ..SizedToggleHarness::bare(MoonSize::Sm, false)
        },
    );
    let control = cx.debug_bounds("sized-control").expect("probe must render");
    assert_eq!(control.size, size(px(36.), px(20.)));
    assert!(cx.debug_bounds("sized:label").is_none());
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

/// Catches the slim variant drifting from its reviewed reference (Sm 32x16 track under a 16px
/// thumb, Md 40x20 under 20px, both outlined a full pixel against the default's hairline), and
/// catches a new toggle defaulting to anything but `Default`. A slim thumb is exactly its track's
/// height, so it rides the track instead of sitting inside it; the two then share one silhouette at
/// the resting end, which the design accepts. A thumb any smaller would read as a shrunken default
/// toggle.
#[test]
fn slim_variant_matches_its_designer_reference() {
    assert_eq!(MoonToggleVariant::default(), MoonToggleVariant::Default);
    assert_eq!(MoonToggle::new("probe").variant, MoonToggleVariant::Default);

    for (tier, track_width, track_height, thumb_size) in [
        (MoonSize::Sm, 32.0, 16.0, 16.0),
        (MoonSize::Md, 40.0, 20.0, 20.0),
    ] {
        let slim = MoonToggleSize::Tier(tier).reference_metrics_for(MoonToggleVariant::Slim);
        assert_eq!(slim.track_width, track_width, "{tier:?}");
        assert_eq!(slim.track_height, track_height, "{tier:?}");
        assert_eq!(slim.thumb_size, thumb_size, "{tier:?}");
        assert_eq!(slim.border, 1.0, "{tier:?}");
        assert_eq!(
            slim.thumb_size, slim.track_height,
            "{tier:?} a slim thumb is the height of its track"
        );

        let default = MoonToggleSize::Tier(tier).reference_metrics();
        assert_eq!(default.border, 0.5, "{tier:?}");
        assert_eq!(
            (default.track_height - default.thumb_size) * 0.5,
            2.0,
            "{tier:?} a default thumb keeps its 2px gap"
        );
        assert!(slim.track_height < default.track_height, "{tier:?}");
        assert!(slim.track_width < default.track_width, "{tier:?}");
    }
}

/// Catches a slim toggle's two outlines drifting apart: its track and its thumb carry one colour
/// per state, so the thumb reads as part of the track rather than a disc laid over it. That colour
/// is `border_secondary` while off, unchanged under the pointer, `toggle_slim_border_pressed` once
/// on, and `toggle_slim_border_pressed_hover` when a checked one is hovered. A default toggle keeps
/// its own rule — an unchecked track outlined in `border_secondary`, a checked one left to its
/// fill, and a thumb that is never outlined. The fills are shared, so they must not differ between
/// the variants.
#[test]
fn a_slim_toggle_outlines_its_track_and_thumb_alike() {
    for (p, roles) in [
        (MoonPalette::TERMINAL, MoonColors::LIGHT),
        (MoonPalette::LIGHT, MoonColors::DARK),
    ] {
        let slim = |checked| {
            ToggleColors::resolve(p, roles, MoonToggleVariant::Slim, None, checked, false)
        };
        let default = |checked| {
            ToggleColors::resolve(p, roles, MoonToggleVariant::Default, None, checked, false)
        };

        for checked in [false, true] {
            let slim = slim(checked);
            assert_eq!(
                slim.thumb_border, slim.border,
                "the thumb must match the track it rides, checked {checked}"
            );
            assert_eq!(
                slim.thumb_border_hover, slim.border_hover,
                "and must keep matching it under the pointer, checked {checked}"
            );
        }

        let off = slim(false);
        assert_eq!(off.border, roles.border_secondary.into());
        assert_eq!(off.border_hover, off.border);

        let on = slim(true);
        assert_eq!(on.border, roles.toggle_slim_border_pressed.into());
        assert_eq!(
            on.border_hover,
            roles.toggle_slim_border_pressed_hover.into()
        );

        assert!(default(true).border.is_transparent());
        assert_eq!(default(false).border, roles.border_secondary.into());
        assert_eq!(default(false).border_hover, default(false).border);
        for checked in [false, true] {
            assert!(default(checked).thumb_border.is_transparent());
            assert!(default(checked).thumb_border_hover.is_transparent());
            assert_eq!(slim(checked).track, default(checked).track);
            assert_eq!(slim(checked).track_hover, default(checked).track_hover);
            assert_eq!(slim(checked).thumb, default(checked).thumb);
        }
    }
}

/// Catches a slim toggle's text drifting from the default variant's: the two differ in the track,
/// the thumb and the outline only, so the label, the supporting text and the gap between text and
/// track must stay identical at both tiers and at any zoom. A slim toggle beside a default one, or
/// beside a checkbox, must line its text up with them.
#[test]
fn slim_keeps_the_text_of_the_default_variant() {
    let tokens = MoonThemeConfig::moon_terminal()
        .with_font_delta(3.0)
        .with_ui_scale(1.5)
        .dark;

    for tier in MoonToggleSize::SUPPORTED_TIERS {
        let size = MoonToggleSize::Tier(tier);
        let slim = size.resolve(MoonToggleVariant::Slim, &tokens).choice();
        let default = size.resolve(MoonToggleVariant::Default, &tokens).choice();

        assert_eq!(slim.font_size, default.font_size, "{tier:?}");
        assert_eq!(slim.line_height, default.line_height, "{tier:?}");
        assert_eq!(slim.label_weight, default.label_weight, "{tier:?}");
        assert_eq!(
            slim.description_weight, default.description_weight,
            "{tier:?}"
        );
        assert_eq!(slim.gap, default.gap, "{tier:?}");
        assert_eq!(slim.description_gap, default.description_gap, "{tier:?}");
    }
}

/// Catches the thumb's travel state losing a case that only shows up in motion: a toggle rendered
/// already checked must show its thumb at that end rather than sliding in from the other; a change
/// must start a travel; the travel must survive the renders that happen while it runs, or the
/// animation is cut off after one frame and the thumb jumps; and a change mid-flight must turn the
/// thumb around from the end it was heading for instead of restarting from where it began. A
/// travel counts as settled only once its duration has passed, which is what lets the track hand
/// its cap to a thumb that has come to a stop on it.
#[test]
fn thumb_travel_starts_survives_and_turns_around() {
    assert_eq!(ThumbTravel::initial(true), ThumbTravel::Resting(true));
    assert_eq!(ThumbTravel::initial(false).from(), None);
    assert!(ThumbTravel::initial(false).settled());

    let resting = ThumbTravel::Resting(false);
    assert_eq!(resting.next(false), resting);

    let travelling = resting.next(true);
    assert!(
        matches!(
            travelling,
            ThumbTravel::Travelling {
                from: false,
                to: true,
                ..
            }
        ),
        "a change must start a travel from the end it is leaving"
    );
    assert_eq!(travelling.from(), Some(false));
    assert!(!travelling.settled(), "a travel starts unsettled");
    // Re-rendering mid-travel must not restart or end it.
    assert_eq!(travelling.next(true), travelling);

    let turned = travelling.next(false);
    assert!(
        matches!(
            turned,
            ThumbTravel::Travelling {
                from: true,
                to: false,
                ..
            }
        ),
        "a change mid-flight must turn around from the end it was heading for"
    );

    // The clock is the only thing that settles a travel.
    let arrived = ThumbTravel::Travelling {
        from: false,
        to: true,
        since: std::time::Instant::now() - THUMB_TRAVEL,
    };
    assert!(arrived.settled());
    assert_eq!(arrived.from(), Some(false));
}

struct TravellingToggleHarness {
    checked: bool,
}

impl gpui::Render for TravellingToggleHarness {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        _: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        use gpui::ParentElement as _;

        gpui::div().child(
            MoonToggle::new("travelling")
                .size(MoonSize::Md)
                .checked(self.checked),
        )
    }
}

/// Catches the thumb teleporting between ends: when a rendered toggle is switched on, the thumb
/// must still be at the unchecked end on the first frame and arrive at the checked end only after
/// the travel's 200ms, so the movement is the animation rather than a jump. A toggle that is
/// switched off again must travel back the same way.
#[gpui::test]
fn the_thumb_slides_between_ends_over_the_travel_duration(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init);
    let window = cx.add_window(|_, _| TravellingToggleHarness { checked: false });
    let mut cx = gpui::VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    let track = cx.debug_bounds("travelling:track").expect("track renders");
    let resting_off = cx.debug_bounds("travelling:thumb").expect("thumb renders");
    assert_eq!(resting_off.left() - track.left(), px(2.));

    window
        .update(&mut cx, |view, _, cx| {
            view.checked = true;
            cx.notify();
        })
        .expect("window stays open");
    cx.run_until_parked();

    let started = cx.debug_bounds("travelling:thumb").expect("thumb renders");
    assert_eq!(
        started.left(),
        resting_off.left(),
        "the thumb must start its travel at the end it is leaving"
    );

    // GPUI animations read the wall clock rather than the test clock, so the travel is waited out
    // and then a repaint is forced to draw the frame that lands the thumb.
    std::thread::sleep(THUMB_TRAVEL + std::time::Duration::from_millis(50));
    window
        .update(&mut cx, |_, _, cx| cx.notify())
        .expect("window stays open");
    cx.run_until_parked();

    let arrived = cx.debug_bounds("travelling:thumb").expect("thumb renders");
    assert_eq!(
        track.right() - arrived.right(),
        px(2.),
        "the thumb must reach the checked end once the travel is over"
    );
}

/// Catches a slim thumb's ring going back to being a border over the thumb's own colour. Drawn that
/// way, a pixel the ring only partly covers mixes the thumb's white into the ring instead of the
/// track behind it, and the thumb picks up a pale halo on the circle's diagonals where the ring
/// runs between two pixel centres. The ring is two discs instead: the thumb is the ring's colour
/// and a face inset by the ring's width carries the thumb's own. A default toggle draws no ring, so
/// it has no face either.
#[gpui::test]
fn a_slim_thumb_is_a_ring_around_a_face_rather_than_a_bordered_disc(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init);
    for (tier, ring) in [(MoonSize::Sm, 1.), (MoonSize::Md, 1.)] {
        for checked in [false, true] {
            let mut cx = render_toggle(
                cx,
                SizedToggleHarness {
                    variant: MoonToggleVariant::Slim,
                    ..SizedToggleHarness::bare(tier, checked)
                },
            );
            let thumb = cx.debug_bounds("sized:thumb").expect("thumb must render");
            let face = cx
                .debug_bounds("sized:thumb-face")
                .expect("a slim thumb must draw its face inside the ring");

            assert_eq!(face.left() - thumb.left(), px(ring), "{tier:?}");
            assert_eq!(thumb.right() - face.right(), px(ring), "{tier:?}");
            assert_eq!(face.top() - thumb.top(), px(ring), "{tier:?}");
            assert_eq!(thumb.bottom() - face.bottom(), px(ring), "{tier:?}");
        }

        let mut cx = render_toggle(cx, SizedToggleHarness::bare(tier, true));
        assert!(
            cx.debug_bounds("sized:thumb-face").is_none(),
            "{tier:?} a default thumb draws no ring, so it needs no face"
        );
    }
}
