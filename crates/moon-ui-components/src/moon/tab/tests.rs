//! Guards what a tab strip paints under each theme skin.

use super::strip_facet_rule;
use crate::moon::{MoonPalette, MoonSkin, MoonThemeTokens, rgba_from};

/// Theme tokens for `palette` drawn with `skin`, at the default scale.
fn skin_tokens(palette: MoonPalette, skin: MoonSkin) -> MoonThemeTokens {
    MoonThemeTokens {
        palette,
        skin,
        ..MoonThemeTokens::default()
    }
}

/// Catches `tab.rs:strip_facet_rule` ignoring the skin: every flat theme would lose the strip's
/// `shell_high` fill and its solid bottom line, which `render_with_theme` keeps only while this
/// answers `None`.
#[test]
fn a_flat_strip_keeps_its_fill_and_solid_line() {
    for palette in [
        MoonPalette::TERMINAL,
        MoonPalette::LIGHT,
        MoonPalette::GRAPHITE,
        MoonPalette::OBSIDIAN,
    ] {
        assert_eq!(
            strip_facet_rule(palette, &skin_tokens(palette, MoonSkin::Flat)),
            None
        );
    }
}

/// Catches `tab.rs:strip_facet_rule` taking the palette from the tokens instead of the palette
/// the strip was asked to paint with, or drawing the rule at the flat line's strength: a
/// faceted strip would carry a hard line across a surface that is meant to fade at its ends.
#[test]
fn a_faceted_strip_draws_a_faint_rule_in_the_overlay_ink_of_its_own_palette() {
    let tokens = skin_tokens(MoonPalette::TERMINAL, MoonSkin::Facet);
    let p = MoonPalette::OBSIDIAN;

    assert_eq!(
        strip_facet_rule(p, &tokens),
        Some(rgba_from(p.overlay, 0.07))
    );
}
