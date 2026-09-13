//! Bundled font registration against the real platform text system.
//!
//! Catches `moon_ui::foundation::init` no longer registering the bundled Inter weights, or the
//! bundled files' family name drifting from "Inter": without them the platform falls back to a
//! system face (Segoe UI on Windows), where Regular and Medium collapse to one font.
//!
//! Runs without the libtest harness (`harness = false`): it needs the full platform, not
//! `headless()` (headless Windows uses a no-op text system), and quitting that platform ends the
//! process. A panic fails the test; reaching the final line passes it.

use gpui::{App, FontWeight, font};

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        moon_ui::foundation::init(cx);
        let text_system = cx.text_system().clone();

        assert!(
            text_system
                .all_font_names()
                .iter()
                .any(|name| name == "Inter"),
            "bundled Inter must be registered under the family name \"Inter\""
        );

        let inter = |weight| {
            let mut descriptor = font("Inter");
            descriptor.weight = weight;
            text_system.resolve_font(&descriptor)
        };
        let ids = [
            FontWeight::NORMAL,
            FontWeight::MEDIUM,
            FontWeight::SEMIBOLD,
            FontWeight::BOLD,
        ]
        .map(inter);
        for (i, a) in ids.iter().enumerate() {
            for b in &ids[i + 1..] {
                assert_ne!(a, b, "each Inter weight must resolve to its own face");
            }
        }

        let geist = text_system.resolve_font(&font("Geist Mono"));
        assert!(
            !ids.contains(&geist),
            "Geist Mono must resolve separately from Inter"
        );

        println!("bundled fonts: Inter 400/500/600/700 and Geist Mono resolve to distinct faces");
        cx.quit();
    });
}
