//! Regression coverage for the primitive colour scales and their packed colour type.

use gpui::{Hsla, rgb, rgba};

use super::{MoonColor, MoonColorScale};
use crate::moon::tokens::relative_luminance;

/// Every opaque ramp, by name. `NEUTRAL_ALPHA` is left out: its steps share one colour
/// and differ only in opacity, so it has its own test.
const SOLID_SCALES: [(&str, MoonColorScale); 27] = [
    ("Neutral", MoonColorScale::NEUTRAL),
    ("Brand", MoonColorScale::BRAND),
    ("Red", MoonColorScale::RED),
    ("Orange", MoonColorScale::ORANGE),
    ("Amber", MoonColorScale::AMBER),
    ("Yellow", MoonColorScale::YELLOW),
    ("Lime", MoonColorScale::LIME),
    ("Green", MoonColorScale::GREEN),
    ("Emerald", MoonColorScale::EMERALD),
    ("Teal", MoonColorScale::TEAL),
    ("Cyan", MoonColorScale::CYAN),
    ("Sky", MoonColorScale::SKY),
    ("Blue", MoonColorScale::BLUE),
    ("Indigo", MoonColorScale::INDIGO),
    ("Violet", MoonColorScale::VIOLET),
    ("Purple", MoonColorScale::PURPLE),
    ("Fuchsia", MoonColorScale::FUCHSIA),
    ("Pink", MoonColorScale::PINK),
    ("Rose", MoonColorScale::ROSE),
    ("Slate", MoonColorScale::SLATE),
    ("Gray", MoonColorScale::GRAY),
    ("Zinc", MoonColorScale::ZINC),
    ("Stone", MoonColorScale::STONE),
    ("Taupe", MoonColorScale::TAUPE),
    ("Mauve", MoonColorScale::MAUVE),
    ("Mist", MoonColorScale::MIST),
    ("Olive", MoonColorScale::OLIVE),
];

/// List a scale's steps from lightest to darkest, each paired with its step number.
///
/// Args:
///     scale: The ramp to unpack.
///
/// Returns:
///     The eleven `(step, colour)` pairs, `50` first.
fn steps(scale: MoonColorScale) -> [(u16, MoonColor); 11] {
    [
        (50, scale.c50),
        (100, scale.c100),
        (200, scale.c200),
        (300, scale.c300),
        (400, scale.c400),
        (500, scale.c500),
        (600, scale.c600),
        (700, scale.c700),
        (800, scale.c800),
        (900, scale.c900),
        (950, scale.c950),
    ]
}

/// Catches two steps of a `primitives.rs` ramp landing in each other's fields — `c600` and `c700`
/// swapped while updating the values, say. Every ramp runs light to dark, so a swap breaks the
/// order, and a palette mapped onto that ramp would get a lighter or darker ink than intended.
#[test]
fn every_solid_scale_darkens_from_c50_to_c950() {
    for (name, scale) in SOLID_SCALES {
        for pair in steps(scale).windows(2) {
            let (lighter_step, lighter) = pair[0];
            let (darker_step, darker) = pair[1];
            let lighter_luminance = relative_luminance(lighter.rgb_hex());
            let darker_luminance = relative_luminance(darker.rgb_hex());
            assert!(
                darker_luminance < lighter_luminance,
                "{name}/{darker_step} #{:06X} is not darker than {name}/{lighter_step} #{:06X}",
                darker.rgb_hex(),
                lighter.rgb_hex(),
            );
        }
    }
}

/// Catches `primitives.rs:MoonColorScale::NEUTRAL_ALPHA` gaining a tinted step or steps out of
/// order. The ramp is white fading out: a non-white step would tint every surface it is laid over,
/// and a later step more opaque than an earlier one would invert the strength of an alpha overlay.
#[test]
fn neutral_alpha_scale_is_white_fading_from_c50_to_c950() {
    let steps = steps(MoonColorScale::NEUTRAL_ALPHA);
    for (step, color) in steps {
        assert_eq!(
            color.rgb_hex(),
            0xFFFFFF,
            "Neutral (alpha)/{step} is #{:06X}, not white",
            color.rgb_hex()
        );
    }
    for pair in steps.windows(2) {
        let (earlier_step, earlier) = pair[0];
        let (later_step, later) = pair[1];
        assert!(
            later.alpha() < earlier.alpha(),
            "Neutral (alpha)/{later_step} alpha {} is not below Neutral (alpha)/{earlier_step} alpha {}",
            later.alpha(),
            earlier.alpha(),
        );
    }
}

/// An opaque `0xRRGGBB` sample whose three channel bytes all differ.
const OPAQUE_SAMPLE: u32 = 0x7F56D9;

/// A translucent `0xRRGGBBAA` sample whose four bytes all differ, so decoding them in any other
/// order yields a different colour.
const TRANSLUCENT_SAMPLE: u32 = 0x7F56D980;

/// Catches `primitives.rs:MoonColor::rgb` packing without the opaque alpha byte, which paints every
/// solid primitive fully transparent, or `MoonColor` decoding its bytes in a different order than
/// GPUI's own `rgb`/`rgba`, which recolours every step. GPUI's decoders are the reference.
#[test]
fn colors_convert_in_gpui_byte_order() {
    assert_eq!(
        Hsla::from(MoonColor::rgb(OPAQUE_SAMPLE)),
        Hsla::from(rgb(OPAQUE_SAMPLE))
    );
    assert_eq!(
        Hsla::from(MoonColor::rgba(TRANSLUCENT_SAMPLE)),
        Hsla::from(rgba(TRANSLUCENT_SAMPLE))
    );
    assert_eq!(MoonColor::rgb(OPAQUE_SAMPLE).rgb_hex(), OPAQUE_SAMPLE);
}
