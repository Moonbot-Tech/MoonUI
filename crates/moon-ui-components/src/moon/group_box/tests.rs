//! Guards what a group box paints under each theme skin.

use super::group_box_facet;
use crate::moon::{
    MoonBackgroundPolicy, MoonFacetKind, MoonPalette, MoonSkin, MoonThemeTokens, facet,
};

/// Theme tokens for `palette` drawn with `skin`, at the default scale.
fn skin_tokens(palette: MoonPalette, skin: MoonSkin) -> MoonThemeTokens {
    MoonThemeTokens {
        palette,
        skin,
        ..MoonThemeTokens::default()
    }
}

/// Catches `group_box.rs:group_box_facet` ignoring the skin: every flat theme's group box would
/// trade its `shell_high` fill and palette border for a gradient card with a shadow.
#[test]
fn a_flat_group_box_keeps_its_fill_and_border() {
    for palette in [MoonPalette::TERMINAL, MoonPalette::LIGHT] {
        let tokens = skin_tokens(palette, MoonSkin::Flat);
        for policy in [
            MoonBackgroundPolicy::Opaque,
            MoonBackgroundPolicy::Transparent,
            MoonBackgroundPolicy::NoFill,
        ] {
            assert_eq!(group_box_facet(&tokens, policy), None);
        }
    }
}

/// Catches `group_box.rs:group_box_facet` filling a box that asked for no fill, or leaving the
/// card's outer shadow on it. GPUI paints a shadow under the whole of its element, so a frame
/// drawn inside a popup would smear a dark patch over the popup behind it.
#[test]
fn a_faceted_group_box_is_a_card_and_a_see_through_one_keeps_only_its_edge() {
    let tokens = skin_tokens(MoonPalette::OBSIDIAN, MoonSkin::Facet);
    let card = facet(MoonFacetKind::Card, &tokens);

    let filled = group_box_facet(&tokens, MoonBackgroundPolicy::Opaque)
        .expect("the Facet skin resolves a group box facet");
    assert_eq!(filled.fill, Some(card.fill));
    assert_eq!(filled.edge, card.edge);
    assert_eq!(filled.shadows, card.shadows);

    for policy in [
        MoonBackgroundPolicy::Transparent,
        MoonBackgroundPolicy::NoFill,
    ] {
        let frame =
            group_box_facet(&tokens, policy).expect("the Facet skin resolves a group box facet");
        assert_eq!(frame.fill, None, "{policy:?}");
        assert_eq!(frame.edge, card.edge);
        assert_eq!(
            frame.shadows,
            vec![card.shadows[0].clone()],
            "only the inset highlight along the top edge"
        );
        assert!(frame.shadows[0].inset);
    }
}
