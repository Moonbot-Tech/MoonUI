//! Badge density, tier and custom-scaling regression coverage.
use super::super::{foundation::MoonSize, theme::MoonThemeTokens};
use super::{MoonBadge, MoonBadgeSize};

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
