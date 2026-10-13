//! Regression coverage for the Moon tag variants, radius and label contrast.

// Do not use `super::*`: the glob would import the `gpui::test` macro, causing `#[test]` to
// recursively expand into itself.
use std::{cell::Cell, rc::Rc};

use gpui::{
    Context, IntoElement, Modifiers, ParentElement as _, Render, VisualTestContext, Window, point,
    px,
};

use super::{
    LABEL_CONTRAST_FLOOR, MoonTag, MoonTagRadius, MoonTagVariant, SOFT_FILL_ALPHA, TagChrome,
    composite, contrast_ratio, count_color, tag_chrome,
};
use crate::h_flex;
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

/// The severity tones a tinted chip is offered in on the health strip.
const SEVERITY_TONES: [MoonTone; 3] = [MoonTone::Danger, MoonTone::Warning, MoonTone::Info];

/// Catches a tinted chip whose quiet label or toned count falls under the AA floor on its own
/// wash, or one that loses its tinted border and reads as a plain neutral chip. The wash the eye
/// sees is recomposited here from the panel and the tone, independently of the chip's own choice.
#[test]
fn tinted_chip_label_and_count_clear_the_contrast_floor() {
    for palette in [MoonPalette::TERMINAL, MoonPalette::LIGHT] {
        let tokens = tokens_with(palette);
        for tone in SEVERITY_TONES {
            let color = tone.color(palette);
            let chip = tag_chrome(
                &tokens,
                tone,
                MoonTagVariant::Tinted,
                MoonTagRadius::Small,
                false,
            );
            let fill = composite(color, palette.panel, SOFT_FILL_ALPHA);
            assert_eq!(chip.border, rgba_from(color, 0.35));
            assert_eq!(chip.background, rgba_from(color, SOFT_FILL_ALPHA));
            for (what, ink) in [
                ("label", chip.text),
                ("count", count_color(tone, MoonTagVariant::Tinted, palette)),
            ] {
                let ratio = contrast_ratio(ink, fill);
                assert!(
                    ratio >= LABEL_CONTRAST_FLOOR,
                    "tinted {tone:?} {what} on {fill:06X}: {ratio:.2}"
                );
            }
        }
    }
}

/// Catches a neutral tinted chip picking up a wash or a tone border, which would make a zero
/// count look like an alarm. The oracle is the mockup's plain `.chip`: panel border, no fill,
/// body-text count.
#[test]
fn neutral_tinted_chip_stays_plain() {
    for palette in [MoonPalette::TERMINAL, MoonPalette::LIGHT] {
        let tokens = tokens_with(palette);
        let chip = tag_chrome(
            &tokens,
            MoonTone::Default,
            MoonTagVariant::Tinted,
            MoonTagRadius::Small,
            false,
        );
        assert_eq!(chip.border, rgba_from(palette.border, 1.0));
        assert_eq!(chip.background.a, 0.0);
        assert_eq!(
            count_color(MoonTone::Default, MoonTagVariant::Tinted, palette),
            palette.text
        );
        assert!(contrast_ratio(chip.text, palette.panel) >= LABEL_CONTRAST_FLOOR);
    }
}

/// Catches `severity_count` keeping the alarm tone on a zero count, or dimming a live one: the
/// strip would show a red `Offline 0` or a faded `Offline 3`.
#[test]
fn severity_count_goes_neutral_and_dimmed_only_at_zero() {
    let zero = MoonTag::new().severity_count(0, MoonTone::Danger);
    assert_eq!(zero.tone, MoonTone::Default);
    assert!(zero.dimmed);
    assert_eq!(zero.count.as_ref().map(|c| c.as_ref()), Some("0"));

    let live = MoonTag::new().severity_count(3, MoonTone::Warning);
    assert_eq!(live.tone, MoonTone::Warning);
    assert!(!live.dimmed);
    assert_eq!(live.count.as_ref().map(|c| c.as_ref()), Some("3"));
}

/// Catches a new tag that is suddenly clickable, selected, dimmed or counted, any of which would
/// restyle every existing tag in the terminal.
#[test]
fn a_new_tag_has_no_count_click_or_selection() {
    let tag = MoonTag::new();
    assert!(tag.count.is_none());
    assert!(tag.on_click.is_none());
    assert!(!tag.selected);
    assert!(!tag.dimmed);
}

/// A window rendering one clickable tag at the origin.
struct ClickableTagHarness {
    clicks: Rc<Cell<usize>>,
}

impl Render for ClickableTagHarness {
    /// Render a clickable tinted chip in the window's top-left corner.
    ///
    /// Args:
    ///     _window: Test window.
    ///     _cx: Test view context.
    ///
    /// Returns:
    ///     The rendered tag.
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let clicks = self.clicks.clone();
        h_flex().child(
            MoonTag::new()
                .variant(MoonTagVariant::Tinted)
                .severity_count(2, MoonTone::Danger)
                .child("Offline")
                .selected(true)
                .on_click("chip", move |_, _, _| clicks.set(clicks.get() + 1)),
        )
    }
}

/// Catches `MoonTag::on_click` being stored but never attached to the rendered chip, which leaves
/// a health-strip filter that does nothing when pressed.
#[gpui::test]
fn clickable_tag_dispatches_its_click(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init);
    let clicks = Rc::new(Cell::new(0));
    let window = cx.add_window({
        let clicks = clicks.clone();
        move |_, _| ClickableTagHarness { clicks }
    });
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();

    cx.simulate_click(point(px(4.0), px(4.0)), Modifiers::default());
    cx.run_until_parked();
    assert_eq!(clicks.get(), 1);
}
