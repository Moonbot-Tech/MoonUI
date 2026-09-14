//! Invariants of the bundled font set that no platform test can see.

use std::collections::HashSet;

use ttf_parser::{Face, name_id};

use super::BUNDLED_FONTS;

/// This module's own source and the web platform's, which keeps a second copy of the list.
const DESKTOP_LIST: &str = include_str!("../fonts.rs");
const WEB_LIST: &str = include_str!("../../../moon-gpui-web/src/platform.rs");

/// macOS keeps one font per PostScript name, so a duplicate shapes with one file and draws with
/// the other (MoonTerminal#558).
#[test]
fn bundled_fonts_have_unique_postscript_names() {
    let mut seen = HashSet::new();
    for bytes in BUNDLED_FONTS {
        let face = Face::parse(bytes, 0).expect("bundled font must parse");
        let name = face
            .names()
            .into_iter()
            .filter(|name| name.name_id == name_id::POST_SCRIPT_NAME)
            .find_map(|name| name.to_string())
            .expect("bundled font must carry a PostScript name");
        assert!(
            seen.insert(name.clone()),
            "two bundled fonts share the PostScript name {name}"
        );
    }
}

/// The `include_bytes!` paths under `assets/fonts/` in one source file, in order.
fn font_paths(source: &str) -> Vec<&str> {
    source
        .lines()
        .filter_map(|line| line.trim().strip_prefix("include_bytes!(\""))
        .filter_map(|rest| rest.split('"').next())
        .filter(|path| path.contains("/assets/fonts/"))
        .collect()
}

/// `moon-gpui-web` cannot depend on this crate, so it embeds the same files by its own list; a
/// weight added on one side only would silently drop out of the wasm build.
#[test]
fn web_platform_bundles_the_same_files() {
    let desktop = font_paths(DESKTOP_LIST);
    let web = font_paths(WEB_LIST);
    assert_eq!(
        desktop.len(),
        BUNDLED_FONTS.len(),
        "every BUNDLED_FONTS entry must be an include_bytes! of a file under assets/fonts"
    );
    assert_eq!(
        desktop, web,
        "moon-gpui-web/src/platform.rs must embed the same font files, in the same order, as \
         moon-ui-components/src/fonts.rs"
    );
}
