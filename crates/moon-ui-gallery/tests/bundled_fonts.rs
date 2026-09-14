//! Bundled font registration against the real platform text system.
//!
//! Catches `moon_ui::foundation::init` no longer registering one of the bundled Inter or Geist Mono
//! weights, or a bundled file's family name drifting: without them the platform falls back to the
//! nearest registered weight or to a system face (Segoe UI on Windows). Also catches a Geist Mono
//! weight shipping as a Latin-only subset, the cut apps registered on top of MoonUI before it
//! bundled Medium and SemiBold.
//!
//! Runs without the libtest harness (`harness = false`): it needs the full platform, not
//! `headless()` (headless Windows uses a no-op text system), and quitting that platform ends the
//! process. A panic fails the test; reaching the final line passes it.

use gpui::{App, FontId, FontWeight, TextSystem, font, px};

const WEIGHTS: [FontWeight; 4] = [
    FontWeight::NORMAL,
    FontWeight::MEDIUM,
    FontWeight::SEMIBOLD,
    FontWeight::BOLD,
];

/// Resolves every bundled weight of `family` and checks each one resolves to its own registered face.
///
/// DirectWrite hands out a new `FontId` for every requested weight, even one it serves with the
/// nearest face, so distinct ids prove nothing. The stem of `|` does: it widens with every weight
/// step in both bundled families.
fn resolve_weights(text_system: &TextSystem, family: &'static str) -> [FontId; 4] {
    assert!(
        text_system
            .all_font_names()
            .iter()
            .any(|name| name == family),
        "bundled {family} must be registered under the family name \"{family}\""
    );
    let ids = WEIGHTS.map(|weight| {
        let mut descriptor = font(family);
        descriptor.weight = weight;
        text_system.resolve_font(&descriptor)
    });
    let stems = ids.map(|id| {
        text_system
            .typographic_bounds(id, px(1000.), '|')
            .unwrap_or_else(|error| panic!("{family} lacks '|': {error}"))
            .size
            .width
    });
    for (pair, weights) in stems.windows(2).zip(WEIGHTS.windows(2)) {
        assert!(
            pair[0] < pair[1],
            "{family} {:?} must draw with its own face, not {:?}'s: stems {stems:?}",
            weights[1],
            weights[0],
        );
    }
    ids
}

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        moon_ui::foundation::init(cx);
        let text_system = cx.text_system().clone();

        resolve_weights(&text_system, "Inter");
        let geist = resolve_weights(&text_system, "Geist Mono");

        // Mono tables mix Latin, Cyrillic and box drawing; every Geist Mono weight must carry them
        // itself and draw them at one monospace advance, which Inter or a system face would not.
        let size = px(10.);
        for (weight, id) in WEIGHTS.iter().zip(geist) {
            let advance = |ch| {
                text_system
                    .advance(id, size, ch)
                    .unwrap_or_else(|error| panic!("Geist Mono {weight:?} lacks {ch:?}: {error}"))
                    .width
            };
            let digit = advance('0');
            for ch in ['i', 'W', 'Ж', 'ё', '│', '→'] {
                assert_eq!(
                    advance(ch),
                    digit,
                    "Geist Mono {weight:?} must draw {ch:?} at the same advance as '0'"
                );
            }
        }

        println!(
            "bundled fonts: Inter and Geist Mono 400/500/600/700 resolve to their own full faces"
        );
        cx.quit();
    });
}
