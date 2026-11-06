//! Regression coverage for MoonCheckbox sizing on the shared size scale.

use super::super::foundation::MoonSize;
use super::{MoonCheckboxSize, size_for};

/// Catches `checkbox.rs:size_for` mapping a tier to the wrong box, or a tier the checkbox does not
/// support failing to resolve to the nearest one it has (`Sm` or `Md`).
#[test]
fn checkbox_tiers_resolve_to_nearest_supported_size() {
    let box_for = |tier: MoonSize| size_for(tier.into());

    assert_eq!(box_for(MoonSize::Xs), crate::Size::XSmall);
    assert_eq!(box_for(MoonSize::Sm), crate::Size::Small);
    assert_eq!(box_for(MoonSize::Md), crate::Size::Medium);
    assert_eq!(box_for(MoonSize::Lg), crate::Size::Medium);
    assert_eq!(box_for(MoonSize::Xl), crate::Size::Medium);
    assert_eq!(box_for(MoonSize::Xxl), crate::Size::Medium);
}

/// Catches renaming `MoonSize` variants without a serde alias, which would fail to load sizes
/// saved under the previous `XSmall`/`Small`/`Medium`/`Large` names.
#[test]
fn moon_size_reads_previous_variant_names() {
    for (saved, tier) in [
        ("\"XSmall\"", MoonSize::Xs),
        ("\"Small\"", MoonSize::Sm),
        ("\"Medium\"", MoonSize::Md),
        ("\"Large\"", MoonSize::Lg),
    ] {
        assert_eq!(serde_json::from_str::<MoonSize>(saved).unwrap(), tier);
    }
}

/// Catches a fixed Md default, which renders compact and standard density rows too large.
#[test]
fn checkbox_default_follows_density_without_overriding_explicit_sizes() {
    use super::MoonCheckbox;
    use crate::moon::MoonThemeTokens;
    let mut tokens = MoonThemeTokens::default();
    for (tier, expected) in [
        (MoonSize::Xs, crate::Size::XSmall),
        (MoonSize::Sm, crate::Size::Small),
        (MoonSize::Md, crate::Size::Medium),
        (MoonSize::Lg, crate::Size::Medium),
    ] {
        tokens.scale.tier = tier;
        assert_eq!(
            size_for(MoonCheckbox::new("default").resolved_size(&tokens)),
            expected
        );
        assert_eq!(
            size_for(
                MoonCheckbox::new("fixed")
                    .size(MoonSize::Sm)
                    .resolved_size(&tokens)
            ),
            crate::Size::Small
        );
        assert_eq!(
            size_for(
                MoonCheckbox::new("custom")
                    .size(MoonCheckboxSize::Custom {
                        box_size: 19.0,
                        font_size: 12.0,
                        line_height: 16.0,
                        gap: 4.0,
                        radius: 2.0,
                    })
                    .resolved_size(&tokens)
            ),
            crate::Size::Size(gpui::px(19.0))
        );
    }
}

/// Catches `checkbox.rs:MoonCheckboxMetrics::base_for_size` restoring XSmall to Small metrics,
/// which makes Compact checkboxes as large as Standard instead of retaining their compact design.
#[test]
fn compact_checkbox_metrics_are_strictly_below_standard() {
    use crate::checkbox::MoonCheckboxMetrics;
    use crate::moon::MoonThemeConfig;

    let xs = MoonCheckboxMetrics::resolve(
        crate::Size::XSmall,
        &MoonThemeConfig::moon_terminal()
            .with_tier(MoonSize::Xs)
            .dark,
    );
    let sm = MoonCheckboxMetrics::resolve(
        crate::Size::Small,
        &MoonThemeConfig::moon_terminal()
            .with_tier(MoonSize::Sm)
            .dark,
    );
    assert!(xs.box_size < sm.box_size);
    assert!(xs.font_size < sm.font_size);
    assert!(xs.line_height < sm.line_height);
    assert!(xs.gap < sm.gap);
    assert!(xs.mark_size < sm.mark_size);
    assert!(xs.mark_stroke < sm.mark_stroke);
}

/// An opaque gradient from `top` at the top edge to `bottom` at the bottom edge, written out
/// rather than taken from the skin.
fn top_to_bottom(top: u32, bottom: u32) -> gpui::Background {
    use crate::moon::rgba_from;
    use gpui::{linear_color_stop, linear_gradient};

    linear_gradient(
        180.0,
        linear_color_stop(rgba_from(top, 1.0), 0.0),
        linear_color_stop(rgba_from(bottom, 1.0), 1.0),
    )
}

/// Catches `checkbox.rs:ChoiceColors::box_facet` ignoring the skin or the disabled flag: every
/// flat theme's checkboxes and radios would be repainted as wells and lit tones, or a disabled
/// box on the faceted theme would lose the flat fill it dims to half. With no facet resolved,
/// a box paints `ChoiceColors::resolve` alone, exactly as it did before skins existed.
#[test]
fn flat_themes_and_disabled_boxes_resolve_no_facet() {
    use crate::checkbox::ChoiceColors;
    use crate::moon::{MoonThemeConfig, MoonThemeTokens, MoonTone};

    let flat_sides = [
        MoonThemeConfig::moon_terminal().dark,
        MoonThemeConfig::moon_terminal().light,
        MoonThemeConfig::moon_graphite().dark,
        MoonThemeConfig::moon_obsidian().light,
        MoonThemeTokens::default(),
    ];
    for tokens in &flat_sides {
        for tone in [None, Some(MoonTone::Warning)] {
            for checked in [false, true] {
                for disabled in [false, true] {
                    assert_eq!(
                        ChoiceColors::box_facet(tokens, tone, checked, disabled),
                        None,
                        "a flat theme never resolves a facet"
                    );
                }
            }
        }
    }

    let faceted = MoonThemeConfig::moon_obsidian().dark;
    for checked in [false, true] {
        assert_eq!(
            ChoiceColors::box_facet(&faceted, None, checked, true),
            None,
            "a disabled box keeps the flat colours"
        );
    }
}

/// Catches a faceted box drifting from its spec: unchecked, it is the skin's well, darkest at
/// its top and edged in the highlight ink at 6%; checked, it is the accent lit from above —
/// lifted 25% at the top, sunk 6% at the bottom, edged in the accent sunk 20% — unless a tone is
/// set, which then replaces the accent. Reading a soft tint for the checked box, or the accent
/// for a toned one, would leave the mark's ink on a fill it was not chosen for.
#[test]
fn a_faceted_box_is_a_well_until_checked_and_then_its_tone_lit_from_above() {
    use crate::checkbox::ChoiceColors;
    use crate::moon::{
        MoonFacetKind, MoonFacetTint, MoonThemeConfig, MoonTone, facet, facet_tinted, rgba_from,
    };

    let tokens = MoonThemeConfig::moon_obsidian().dark;
    let p = tokens.palette;
    // `gutter` 0x08090B sunk by 15%.
    const WELL_TOP: u32 = 0x070809;
    // The accent 0xFFB347 lifted by 25%, sunk by 6% and sunk by 20%.
    const ACCENT_TOP: u32 = 0xFFC675;
    const ACCENT_BOTTOM: u32 = 0xF0A843;
    const ACCENT_EDGE: u32 = 0xCC8F39;
    // The Info tone, `blue` 0x7FC9FF, sunk by 20%.
    const BLUE_EDGE: u32 = 0x66A1CC;

    let unchecked = ChoiceColors::box_facet(&tokens, None, false, false).expect("faceted");
    assert_eq!(unchecked, facet(MoonFacetKind::Socket, &tokens));
    assert_eq!(unchecked.fill, top_to_bottom(WELL_TOP, p.gutter));
    assert_eq!(unchecked.edge, rgba_from(p.overlay, 0.17));
    assert_eq!(
        ChoiceColors::box_facet(&tokens, Some(MoonTone::Info), false, false),
        Some(unchecked),
        "a tone colours only a checked box"
    );

    let checked = ChoiceColors::box_facet(&tokens, None, true, false).expect("faceted");
    assert_eq!(
        checked,
        facet_tinted(p.accent, MoonFacetTint::Solid, &tokens)
    );
    assert_eq!(checked.fill, top_to_bottom(ACCENT_TOP, ACCENT_BOTTOM));
    assert_eq!(checked.edge, rgba_from(ACCENT_EDGE, 1.0));

    let toned =
        ChoiceColors::box_facet(&tokens, Some(MoonTone::Info), true, false).expect("faceted");
    assert_eq!(toned, facet_tinted(p.blue, MoonFacetTint::Solid, &tokens));
    assert_eq!(toned.edge, rgba_from(BLUE_EDGE, 1.0));
}

/// A checked small checkbox, shrunk to its content so the probes measure the control.
struct FacetedCheckboxHarness;

impl gpui::Render for FacetedCheckboxHarness {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        _: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        use gpui::ParentElement as _;

        crate::h_flex().child(
            super::MoonCheckbox::new("faceted")
                .size(MoonSize::Sm)
                .checked(true),
        )
    }
}

/// Catches the faceted box changing the checkbox's geometry — a facet applied with a border of
/// another width, or the box resized to the design mock-up's: the mark is placed from the box's
/// 1px border, so under the faceted theme the box must keep its 16px and the 12px mark must stay
/// centred in it, exactly as under a flat theme.
#[gpui::test]
fn a_faceted_checkbox_keeps_its_box_and_centred_mark(cx: &mut gpui::TestAppContext) {
    use crate::moon::{MoonTheme, MoonThemeConfig};
    use gpui::{px, size};

    cx.update(crate::init);
    for (config, faceted) in [
        (MoonThemeConfig::moon_terminal(), false),
        (MoonThemeConfig::moon_obsidian(), true),
    ] {
        cx.update(|cx| MoonTheme::install_config(config, cx));
        cx.update(|cx| assert_eq!(MoonTheme::active_tokens(cx).skin.is_facet(), faceted));

        let window = cx.add_window(|_, _| FacetedCheckboxHarness);
        let mut cx = gpui::VisualTestContext::from_window(window.into(), cx);
        cx.run_until_parked();

        let box_bounds = cx.debug_bounds("faceted:box").expect("box must render");
        let mark = cx.debug_bounds("faceted:mark").expect("mark must render");
        assert_eq!(box_bounds.size, size(px(16.), px(16.)), "faceted={faceted}");
        assert_eq!(mark.size, size(px(12.), px(12.)), "faceted={faceted}");
        assert_eq!(mark.center(), box_bounds.center(), "faceted={faceted}");
    }
}
