//! Guards the surface and the arrow a tooltip paints under each theme skin.

use super::{MoonTooltipPlacement, MoonTooltipSize, tooltip_arrow_color, tooltip_facet};
use crate::moon::{MoonFacetKind, MoonPalette, MoonSkin, MoonThemeTokens, facet};

const ALL_PLACEMENTS: [MoonTooltipPlacement; 4] = [
    MoonTooltipPlacement::Top,
    MoonTooltipPlacement::Bottom,
    MoonTooltipPlacement::Left,
    MoonTooltipPlacement::Right,
];

/// Theme tokens for `palette` drawn with `skin`, with every length doubled.
fn doubled_skin_tokens(palette: MoonPalette, skin: MoonSkin) -> MoonThemeTokens {
    let mut tokens = MoonThemeTokens {
        palette,
        skin,
        ..MoonThemeTokens::default()
    };
    tokens.scale.ui = 2.0;
    tokens
}

/// A custom tooltip size whose only distinguishing value is its corner radius.
fn custom_size(radius: f32) -> MoonTooltipSize {
    MoonTooltipSize::Custom {
        min_height: 20.0,
        font_size: 10.0,
        line_height: 13.0,
        pad_x: 6.0,
        pad_y: 3.0,
        radius,
        gap: 6.0,
        arrow_size: 6.0,
    }
}

/// Catches `tooltip.rs:tooltip_facet` or `tooltip_arrow_color` ignoring the skin: a flat
/// tooltip would get a gradient popup surface, or an arrow that no longer matches its
/// `shell_high` body.
#[test]
fn a_flat_tooltip_keeps_its_solid_surface_and_matching_arrow() {
    for palette in [MoonPalette::TERMINAL, MoonPalette::LIGHT] {
        let tokens = doubled_skin_tokens(palette, MoonSkin::Flat);
        for size in [
            MoonTooltipSize::Compact,
            MoonTooltipSize::Normal,
            custom_size(3.0),
        ] {
            assert_eq!(tooltip_facet(&tokens, size), None);
        }
        for placement in ALL_PLACEMENTS {
            assert_eq!(
                tooltip_arrow_color(palette, &tokens, placement),
                (palette.shell_high, 0.98)
            );
        }
    }
}

/// Catches `tooltip.rs:tooltip_facet` giving every size one radius, overriding a caller's own
/// radius, or leaving the radius unscaled: a compact tooltip would be rounder than its height
/// allows, and at a doubled UI scale the corners would stay half as round as the box.
#[test]
fn a_faceted_tooltip_is_a_popup_whose_radius_follows_its_size_and_the_ui_scale() {
    let tokens = doubled_skin_tokens(MoonPalette::OBSIDIAN, MoonSkin::Facet);
    let popup = facet(MoonFacetKind::Popup, &tokens);

    for (size, radius) in [
        (MoonTooltipSize::Normal, 16.0),
        (MoonTooltipSize::Compact, 12.0),
        (custom_size(3.0), 6.0),
    ] {
        assert_eq!(
            tooltip_facet(&tokens, size),
            Some((popup.clone(), radius)),
            "{size:?}"
        );
    }
}

/// Catches `tooltip.rs:tooltip_arrow_color` keeping the flat `shell_high` under the Facet skin,
/// or swapping the two ends of the popup fill: the arrow would show as a patch of another
/// colour against the edge of the tooltip it is meant to continue.
#[test]
fn a_faceted_arrow_takes_the_colour_of_the_edge_it_hangs_from() {
    let p = MoonPalette::OBSIDIAN;
    let tokens = doubled_skin_tokens(p, MoonSkin::Facet);

    assert_eq!(
        tooltip_arrow_color(p, &tokens, MoonTooltipPlacement::Top),
        (p.card, 1.0),
        "a tooltip above its target points down, from the dark end of its fill"
    );
    for placement in [
        MoonTooltipPlacement::Bottom,
        MoonTooltipPlacement::Left,
        MoonTooltipPlacement::Right,
    ] {
        assert_eq!(
            tooltip_arrow_color(p, &tokens, placement),
            (p.panel_high, 1.0),
            "{placement:?}"
        );
    }
}
