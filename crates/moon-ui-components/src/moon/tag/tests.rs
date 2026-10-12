//! Regression coverage for the Moon tag variants, radius and label contrast.

// Do not use `super::*`: the glob would import the `gpui::test` macro, causing `#[test]` to
// recursively expand into itself.
use super::{
    LABEL_CONTRAST_FLOOR, MoonTag, MoonTagRadius, MoonTagVariant, SOFT_FILL_ALPHA, TagChrome,
    composite, contrast_ratio, tag_chrome,
};
use crate::moon::{MoonPalette, MoonThemeTokens, MoonTone, rgba_from};

/// The status tones a soft tag is offered in, on both built-in palettes.
const SOFT_TONES: [MoonTone; 4] = [
    MoonTone::Positive,
    MoonTone::Warning,
    MoonTone::Info,
    MoonTone::Danger,
];

/// Theme tokens painting with `palette` at the reference scale.
fn tokens_with(palette: MoonPalette) -> MoonThemeTokens {
    MoonThemeTokens {
        palette,
        ..MoonThemeTokens::default()
    }
}

/// Catches a default builder that drifts off the original outline pill, which would restyle
/// every existing tag in the terminal. The oracle is the pre-variant render: a 0.48 tone border,
/// a transparent fill, the tone as label, a 999 radius, 10px padding and a 22px row.
#[test]
fn default_tag_keeps_the_original_outline_pill() {
    let tag = MoonTag::new();
    assert_eq!(tag.variant, MoonTagVariant::Outline);
    assert_eq!(tag.radius, MoonTagRadius::Full);
    assert!(!tag.compact);

    for palette in [MoonPalette::TERMINAL, MoonPalette::LIGHT] {
        let tokens = tokens_with(palette);
        for tone in SOFT_TONES {
            let color = tone.color(palette);
            assert_eq!(
                tag_chrome(
                    &tokens,
                    tone,
                    MoonTagVariant::default(),
                    MoonTagRadius::default(),
                    false
                ),
                TagChrome {
                    border: rgba_from(color, 0.48),
                    background: rgba_from(color, 0.0),
                    text: color,
                    radius: tokens.ui(999.0),
                    pad_x: tokens.ui(10.0),
                    min_h: tokens.fit_height(22.0, 14.0, 4.0),
                }
            );
        }
    }
}

/// Catches a soft or filled label that falls under the WCAG AA floor on its own fill — the
/// Settings save-mode chip turning unreadable on one theme. The fill the eye sees is recomposited
/// here from the panel and the tone, independently of the label choice.
#[test]
fn soft_and_filled_labels_clear_the_contrast_floor() {
    for palette in [MoonPalette::TERMINAL, MoonPalette::LIGHT] {
        let tokens = tokens_with(palette);
        for tone in SOFT_TONES {
            let soft = tag_chrome(
                &tokens,
                tone,
                MoonTagVariant::Soft,
                MoonTagRadius::Small,
                true,
            );
            let fill = composite(tone.color(palette), palette.panel, SOFT_FILL_ALPHA);
            let ratio = contrast_ratio(soft.text, fill);
            assert!(
                ratio >= LABEL_CONTRAST_FLOOR,
                "soft {tone:?} on {:06X}: {ratio:.2}",
                palette.shell
            );
        }
    }

    let filled = [
        (MoonPalette::LIGHT, &SOFT_TONES[..]),
        (
            MoonPalette::TERMINAL,
            &[MoonTone::Warning, MoonTone::Info][..],
        ),
    ];
    for (palette, tones) in filled {
        let tokens = tokens_with(palette);
        for tone in tones {
            let chrome = tag_chrome(
                &tokens,
                *tone,
                MoonTagVariant::Filled,
                MoonTagRadius::Small,
                true,
            );
            let ratio = contrast_ratio(chrome.text, tone.color(palette));
            assert!(
                ratio >= LABEL_CONTRAST_FLOOR,
                "filled {tone:?} on {:06X}: {ratio:.2}",
                palette.shell
            );
        }
    }
}

/// Catches the soft variant keeping a visible border or the filled one losing its solid fill,
/// either of which makes the chip look like the outline tag again.
#[test]
fn soft_and_filled_paint_their_fill_not_a_border() {
    let tokens = tokens_with(MoonPalette::LIGHT);
    let color = MoonTone::Info.color(MoonPalette::LIGHT);

    let soft = tag_chrome(
        &tokens,
        MoonTone::Info,
        MoonTagVariant::Soft,
        MoonTagRadius::Full,
        false,
    );
    assert_eq!(soft.border, rgba_from(color, 0.0));
    assert_eq!(soft.background, rgba_from(color, 0.14));

    let filled = tag_chrome(
        &tokens,
        MoonTone::Info,
        MoonTagVariant::Filled,
        MoonTagRadius::Full,
        false,
    );
    assert_eq!(filled.background, rgba_from(color, 1.0));
}

/// Catches the small radius or compact geometry being ignored, which leaves the Settings chip a
/// tall pill. The oracle is the mockup's `.mode` chip: 3px corners, 7px side padding, 18px row.
#[test]
fn small_radius_and_compact_geometry_match_the_status_chip() {
    let tokens = tokens_with(MoonPalette::TERMINAL);
    let chip = tag_chrome(
        &tokens,
        MoonTone::Info,
        MoonTagVariant::Soft,
        MoonTagRadius::Small,
        true,
    );

    assert_eq!(chip.radius, tokens.ui(3.0));
    assert_eq!(chip.pad_x, tokens.ui(7.0));
    assert_eq!(chip.min_h, tokens.fit_height(18.0, 14.0, 2.0));
}

/// Catches a compositing slip (alpha applied to the wrong side) that would measure soft-label
/// contrast against the wrong colour. Hand-computed: 25% white over black is 0x404040.
#[test]
fn composite_blends_the_foreground_over_the_background() {
    assert_eq!(composite(0xFFFFFF, 0x000000, 0.25), 0x404040);
    assert_eq!(composite(0x123456, 0xABCDEF, 1.0), 0x123456);
    assert_eq!(composite(0x123456, 0xABCDEF, 0.0), 0xABCDEF);
}
