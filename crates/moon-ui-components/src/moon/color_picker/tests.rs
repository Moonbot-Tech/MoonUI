//! Regression coverage for the Moon colour-picker hex field and reuse palette.

use super::{MAX_CUSTOM_COLORS, hex_label, parse_hex_rgb, push_custom, rgb8_of};
use gpui::{Hsla, rgb};

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
