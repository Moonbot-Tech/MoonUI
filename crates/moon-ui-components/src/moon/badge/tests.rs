//! Badge density, tier, custom-scaling and Facet-skin regression coverage.
use super::super::{
    MoonPalette, MoonSkin, MoonTone, foundation::MoonSize, rgba_from, theme::MoonThemeTokens,
};
use super::{MoonBadge, MoonBadgeSize, MoonBadgeVariant};
use gpui::{linear_color_stop, linear_gradient};

/// Catches selecting control height or legacy font scaling: badges must stay text-height at every density.
#[test]
fn badge_density_is_text_height_and_zoom_only() {
    for (tier, height, font) in [
        (MoonSize::Xs, 16.0, 11.0),
        (MoonSize::Sm, 20.0, 12.0),
        (MoonSize::Md, 24.0, 14.0),
        (MoonSize::Xxl, 24.0, 14.0),
    ] {
        let mut tokens = MoonThemeTokens::default();
        tokens.scale.tier = tier;
        tokens.scale.ui = 1.5;
        tokens.scale.font = 2.0;
        tokens.scale.font_delta = 6.0;
        let m = MoonBadge::new("density").metrics(&tokens);
        assert_eq!(
            (m.height, m.min_width, m.line_height),
            (height * 1.5, height * 1.5, height * 1.5)
        );
        assert_eq!(m.font_size, font * 1.5);
    }
}

/// Catches ignoring explicit tiers when density differs, which resizes pinned badges.
#[test]
fn badge_tiers_override_density() {
    let tokens = MoonThemeTokens::default();
    for (size, height) in [
        (MoonBadgeSize::Tier(MoonSize::Xs), 16.0),
        (MoonBadgeSize::Tier(MoonSize::Sm), 20.0),
        (MoonSize::Lg.into(), 24.0),
    ] {
        assert_eq!(
            MoonBadge::new("tier").size(size).metrics(&tokens).height,
            height
        );
    }
}

/// Catches dropping the Custom line-fit rule or applying font scaling twice.
#[test]
fn badge_custom_preserves_legacy_scaling_and_fit() {
    let mut tokens = MoonThemeTokens::default();
    tokens.scale.ui = 1.5;
    tokens.scale.font = 2.0;
    tokens.scale.font_delta = 3.0;
    let m = MoonBadge::new("custom")
        .size(MoonBadgeSize::Custom {
            height: 13.0,
            radius: 4.0,
            font_size: 8.5,
            line_height: 11.0,
            pad_x: 4.0,
            min_width: 16.0,
        })
        .metrics(&tokens);
    assert_eq!(
        (m.height, m.font_size, m.line_height, m.pad_x),
        (28.0, 20.0, 25.0, 6.0)
    );
}

/// Theme tokens painting `palette` with `skin` at the reference scale.
fn tokens_with(palette: MoonPalette, skin: MoonSkin) -> MoonThemeTokens {
    MoonThemeTokens {
        skin,
        palette,
        ..MoonThemeTokens::default()
    }
}

/// The wash of `color` from `top` opacity along the top edge to `bottom` along the bottom
/// edge, written out here rather than taken from the code under test.
fn wash(color: u32, top: f32, bottom: f32) -> gpui::Background {
    linear_gradient(
        180.0,
        linear_color_stop(rgba_from(color, top), 0.0),
        linear_color_stop(rgba_from(color, bottom), 1.0),
    )
}

/// Catches `badge.rs:outline_wash` losing the wash of an outline badge on the faceted theme,
/// tinting it with the wrong colour, or ignoring the strength a faded badge passes. The oracle
/// is the depth study's outlined chip: the tone at 14% along the top down to 4%.
#[test]
fn an_outline_badge_under_the_facet_skin_is_washed_with_its_tone() {
    for palette in [MoonPalette::OBSIDIAN, MoonPalette::LIGHT] {
        let tokens = tokens_with(palette, MoonSkin::Facet);
        let tone = MoonTone::Warning.color(palette);
        let badge = || {
            MoonBadge::new("3")
                .variant(MoonBadgeVariant::Outline)
                .tone(MoonTone::Warning)
        };

        let plain = badge();
        assert_eq!(
            plain.outline_wash(plain.style(palette), 1.0, &tokens),
            Some(wash(tone, 0.14, 0.04))
        );

        // Rendering passes the factor a disabled badge fades its flat fill by; both ends of
        // the wash follow it.
        assert_eq!(
            plain.outline_wash(plain.style(palette), 0.5, &tokens),
            Some(wash(tone, 0.07, 0.02))
        );

        // A caller's fill colour still tints the badge.
        let recoloured = badge().bg_color(palette.blue);
        assert_eq!(
            recoloured.outline_wash(recoloured.style(palette), 1.0, &tokens),
            Some(wash(palette.blue, 0.14, 0.04))
        );
    }
}

/// Catches `badge.rs:MoonBadge::render_with_theme` painting a wash on a flat theme, not
/// painting the wash it resolves on the faceted one, leaving a disabled badge's wash at full
/// strength, or washing over a soft badge or a fill the caller set. The flat colours are the
/// flat design's own: the tone at 8% under a 27% border, both halved when disabled.
#[test]
fn only_the_outline_badge_of_the_faceted_theme_paints_a_wash() {
    use super::super::MoonThemeConfig;
    use super::super::button::scene_probe::{Painted, Pointer, painted};

    let p = MoonPalette::OBSIDIAN;
    let tone = MoonTone::Warning.color(p);
    let obsidian_with = |skin: MoonSkin| {
        let mut config = MoonThemeConfig::moon_obsidian();
        config.dark.skin = skin;
        config
    };
    let outline = || {
        MoonBadge::new("3")
            .variant(MoonBadgeVariant::Outline)
            .tone(MoonTone::Warning)
    };
    let flat_outline = Painted::flat(rgba_from(tone, 0.08), rgba_from(tone, 0.27));

    assert_eq!(
        painted(obsidian_with(MoonSkin::Flat), Pointer::Away, outline),
        flat_outline
    );
    assert_eq!(
        painted(obsidian_with(MoonSkin::Flat), Pointer::Away, move || {
            outline().disabled(true)
        }),
        Painted::flat(rgba_from(tone, 0.04), rgba_from(tone, 0.135))
    );

    assert_eq!(
        painted(obsidian_with(MoonSkin::Facet), Pointer::Away, outline),
        Painted {
            fill: Some(wash(tone, 0.14, 0.04)),
            ..flat_outline.clone()
        }
    );
    assert_eq!(
        painted(obsidian_with(MoonSkin::Facet), Pointer::Away, move || {
            outline().disabled(true)
        }),
        Painted {
            fill: Some(wash(tone, 0.07, 0.02)),
            border: Some(rgba_from(tone, 0.135)),
            shadows: Vec::new(),
        },
        "a disabled badge halves the wash with the rest of its paint"
    );

    assert_eq!(
        painted(obsidian_with(MoonSkin::Facet), Pointer::Away, move || {
            outline().bg_alpha(0.3)
        }),
        Painted::flat(rgba_from(tone, 0.3), rgba_from(tone, 0.27)),
        "an explicit fill opacity is painted as given"
    );
    assert_eq!(
        painted(obsidian_with(MoonSkin::Facet), Pointer::Away, || {
            MoonBadge::new("3").tone(MoonTone::Warning)
        }),
        Painted::flat(rgba_from(tone, 0.14), rgba_from(tone, 0.0)),
        "a soft badge keeps its own fill"
    );
}

/// Catches `badge.rs:outline_wash` answering for the Flat skin, which would repaint the
/// outline badges of every existing theme, for a solid or soft badge, whose own fill it would
/// replace, or for a badge whose caller set the fill opacity and must get exactly that fill.
#[test]
fn flat_solid_soft_and_explicitly_filled_badges_get_no_wash() {
    for palette in [MoonPalette::OBSIDIAN, MoonPalette::LIGHT] {
        let faceted = tokens_with(palette, MoonSkin::Facet);
        let flat = tokens_with(palette, MoonSkin::Flat);
        let wash_of = |badge: MoonBadge, tokens: &MoonThemeTokens| {
            badge.outline_wash(badge.style(palette), 1.0, tokens)
        };

        for variant in [
            MoonBadgeVariant::Solid,
            MoonBadgeVariant::Soft,
            MoonBadgeVariant::Outline,
        ] {
            assert_eq!(
                wash_of(MoonBadge::new("3").variant(variant), &flat),
                None,
                "{variant:?} is washed under the Flat skin"
            );
        }
        for variant in [MoonBadgeVariant::Solid, MoonBadgeVariant::Soft] {
            assert_eq!(
                wash_of(MoonBadge::new("3").variant(variant), &faceted),
                None,
                "{variant:?} keeps its own fill under the Facet skin"
            );
        }
        assert_eq!(
            wash_of(
                MoonBadge::new("3")
                    .variant(MoonBadgeVariant::Outline)
                    .bg_alpha(0.3),
                &faceted
            ),
            None,
            "an explicit fill opacity is kept as given"
        );
    }
}
