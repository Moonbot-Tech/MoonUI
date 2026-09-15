//! Regression coverage for MoonCheckbox sizing on the shared size scale.

use super::super::foundation::MoonSize;
use super::{MoonCheckboxSize, size_for};

/// Catches `checkbox.rs:size_for` mapping a tier to the wrong box, or a tier the checkbox does not
/// support failing to resolve to the nearest one it has (`Sm` or `Md`).
#[test]
fn checkbox_tiers_resolve_to_nearest_supported_size() {
    let box_for = |tier: MoonSize| size_for(tier.into());

    assert_eq!(box_for(MoonSize::Xs), crate::Size::Small);
    assert_eq!(box_for(MoonSize::Sm), crate::Size::Small);
    assert_eq!(box_for(MoonSize::Md), crate::Size::Medium);
    assert_eq!(box_for(MoonSize::Lg), crate::Size::Medium);
    assert_eq!(box_for(MoonSize::Xl), crate::Size::Medium);
    assert_eq!(box_for(MoonSize::Xxl), crate::Size::Medium);
}

/// Catches the deprecated `Compact`/`Normal` aliases drifting from the tiers they replaced, which
/// would silently resize checkboxes in apps that have not migrated yet.
#[test]
#[allow(deprecated)]
fn deprecated_checkbox_sizes_keep_their_tiers() {
    assert_eq!(MoonCheckboxSize::Compact, MoonSize::Sm.into());
    assert_eq!(MoonCheckboxSize::Normal, MoonSize::Md.into());
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
        (MoonSize::Xs, crate::Size::Small),
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
