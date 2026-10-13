//! Regression coverage for the Moon colour-picker hex field and reuse palette.

use super::{
    MAX_CUSTOM_COLORS, MoonColorPickerTrigger, TriggerChrome, hex_label, parse_hex_rgb,
    push_all_custom, push_custom, rgb8_of, trigger_chrome,
};
use crate::moon::{MoonScale, MoonThemeTokens};
use gpui::{Hsla, hsla, rgb};

/// Builds an opaque colour from its independently chosen six-digit RGB value.
fn color(value: u32) -> Hsla {
    rgb(value).into()
}

/// Reads a list in the user-visible hexadecimal form instead of comparing floating-point HSLA.
fn labels(colors: &[Hsla]) -> Vec<String> {
    colors
        .iter()
        .map(|color| hex_label(*color).to_string())
        .collect()
}

/// Catches removing trimming or case-insensitive ASCII hex acceptance from `color_picker.rs:parse_hex_rgb`.
///
/// Pasted lower-case or padded hex input would otherwise be rejected even though it names the
/// same orange swatch as the canonical readout.
#[test]
fn hex_parser_accepts_case_and_surrounding_whitespace() {
    let expected = Some([0xFF, 0x80, 0x00]);
    for text in ["#ff8000", "FF8000", "  #Ff8000  "] {
        assert_eq!(parse_hex_rgb(text).map(rgb8_of), expected, "{text:?}");
    }
}

/// Catches changing `color_picker.rs:parse_hex_rgb` to accept eight digits as an RGB value.
///
/// Invalid or alpha-bearing input would otherwise commit a colour that cannot round-trip through
/// the six-digit picker field.
#[test]
fn hex_parser_rejects_every_non_six_digit_input() {
    for text in ["", "#FFF", "#GGGGGG", "#FFFFFFFF", "12345", "#1234567"] {
        assert_eq!(parse_hex_rgb(text), None, "{text:?} must be rejected");
    }
}

/// Catches replacing `color_picker.rs:push_custom` front insertion with appending.
///
/// A newly committed colour must be the first reusable swatch rather than being hidden behind
/// older entries.
#[test]
fn new_custom_colour_becomes_the_front_entry() {
    let mut list = vec![color(0x112233)];

    assert!(push_custom(&mut list, color(0xA5A5A5)));
    assert_eq!(labels(&list), ["#A5A5A5", "#112233"]);
}

/// Catches removing the `already_front` early return from `color_picker.rs:push_custom`.
///
/// Recommitting the current front colour would otherwise produce a redundant persisted custom-
/// colour event even though the visible list did not change.
#[test]
fn recommitting_the_front_custom_colour_is_a_no_op() {
    let mut list = vec![color(0x808080), color(0x112233)];
    let before = labels(&list);

    assert!(!push_custom(&mut list, color(0x808080)));
    assert_eq!(labels(&list), before);
}

/// Catches removing RGB-byte de-duplication from `color_picker.rs:push_custom`.
///
/// Reusing an older swatch would otherwise show it twice instead of moving that same colour to
/// the first reusable position.
#[test]
fn an_existing_custom_colour_moves_to_front_without_a_duplicate() {
    let mut list = vec![color(0x112233), color(0xA5A5A5)];

    assert!(push_custom(&mut list, color(0xA5A5A5)));
    assert_eq!(labels(&list), ["#A5A5A5", "#112233"]);
}

/// Catches removing `color_picker.rs:push_custom`'s `MAX_CUSTOM_COLORS` truncation.
///
/// An unbounded history would keep growing the picker and push its reusable-swatch grid beyond
/// its intended fixed memory and visual budget.
#[test]
fn custom_colour_history_is_capped_with_the_newest_entries_first() {
    let mut list = Vec::new();
    for value in 1..=(MAX_CUSTOM_COLORS as u32 + 3) {
        assert!(push_custom(&mut list, color(value)));
    }

    assert_eq!(list.len(), MAX_CUSTOM_COLORS);
    assert_eq!(labels(&list).first(), Some(&"#000017".to_string()));
    assert_eq!(labels(&list).last(), Some(&"#000004".to_string()));
}

/// Catches removing the reverse iteration in `color_picker.rs:push_all_custom`.
///
/// A persisted or shared most-recent-first reuse palette would otherwise show its custom
/// swatches oldest-first after it is seeded or replaced.
#[test]
fn seeded_custom_colours_preserve_most_recent_first_order() {
    let most_recent_first = vec![color(0xA5A5A5), color(0x8A2BE2), color(0x3A6EA5)];
    let mut list = Vec::new();

    push_all_custom(&mut list, most_recent_first.iter().copied());

    assert_eq!(list, most_recent_first);
}

/// Catches reverting `color_picker.rs:rgb8_of` to truncate `c.r * 255.0` with `as u8` instead
/// of rounding each channel.
///
/// The picker palette's 50% gray swatch would otherwise display and round-trip as `#7F7F7F`
/// instead of the nearest-byte `#808080`.
#[test]
fn hsl_middle_gray_rounds_to_the_picker_hex_byte() {
    let picker_middle_gray = hsla(0.0, 0.0, 0.5, 1.0);

    assert_eq!(rgb8_of(picker_middle_gray), [0x80, 0x80, 0x80]);
}

/// Catches a default trigger that drifts off the original 128px field, which would restyle every
/// existing picker. The oracle is the pre-variant render: 128px wide, a 26px fitted row, a 14px
/// swatch with 3px corners and the hex label always shown.
#[test]
fn default_trigger_keeps_the_original_field() {
    assert_eq!(
        MoonColorPickerTrigger::default(),
        MoonColorPickerTrigger::Field
    );
    let tokens = MoonThemeTokens::default();
    let field = TriggerChrome {
        width: Some(128.0),
        field_height: Some(tokens.fit_height(26.0, 13.0, 6.5)),
        swatch_w: tokens.ui(14.0),
        swatch_h: tokens.ui(14.0),
        swatch_radius: tokens.ui(3.0),
        show_hex: true,
    };
    for show_hex in [None, Some(false), Some(true)] {
        assert_eq!(
            trigger_chrome(&tokens, MoonColorPickerTrigger::Field, show_hex),
            field
        );
    }
}

/// Catches the swatch trigger keeping the field frame or the fixed width, so a Settings colour row
/// stays a 128px box. The oracle is the mockup's `.swatch`: 28x20 px, 3px corners, no frame, and
/// no hex label unless asked for.
#[test]
fn swatch_trigger_is_a_frameless_mockup_sized_swatch() {
    let tokens = MoonThemeTokens::default();
    let swatch = trigger_chrome(&tokens, MoonColorPickerTrigger::Swatch, None);
    assert_eq!(swatch.width, None);
    assert_eq!(swatch.field_height, None);
    assert_eq!((swatch.swatch_w, swatch.swatch_h), (28.0, 20.0));
    assert_eq!(swatch.swatch_radius, 3.0);
    assert!(!swatch.show_hex);
    assert!(trigger_chrome(&tokens, MoonColorPickerTrigger::Swatch, Some(true)).show_hex);
}

/// Catches the swatch trigger ignoring the UI scale, which would leave it tiny at 150% zoom.
/// Hand-computed: 28x20 at 1.5x is 42x30.
#[test]
fn swatch_trigger_follows_the_ui_scale() {
    let tokens = MoonThemeTokens {
        scale: MoonScale {
            ui: 1.5,
            ..MoonScale::default()
        },
        ..MoonThemeTokens::default()
    };
    let swatch = trigger_chrome(&tokens, MoonColorPickerTrigger::Swatch, None);
    assert_eq!((swatch.swatch_w, swatch.swatch_h), (42.0, 30.0));
}
