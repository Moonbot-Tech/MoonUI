//! Bundled UI fonts, registered with the text system so MoonUI renders in Inter and Geist Mono
//! without the user installing either.

use std::borrow::Cow;

use gpui::App;

/// Inter and Geist Mono (both SIL Open Font License, see each family's `OFL.txt` under
/// `assets/fonts/`) at the weights MoonUI uses: Regular, Medium, SemiBold and Bold.
///
/// The Inter files are the 18pt optical-size cut with their family renamed from "Inter 18pt" to
/// "Inter", so the theme's `"Inter"` family resolves to them on every platform. The Geist Mono
/// files are the unmodified static cuts from one Geist release, whose typographic family is
/// already "Geist Mono".
///
/// Every file has its own PostScript name, and an app should not register its own copy of either
/// family: the macOS text system maps a shaped run back to its font by PostScript name, so a
/// second file under an existing name draws one cut's glyph ids from the other cut's outlines.
pub(crate) const BUNDLED_FONTS: &[&[u8]] = &[
    include_bytes!("../../../assets/fonts/inter/Inter-Regular.ttf"),
    include_bytes!("../../../assets/fonts/inter/Inter-Medium.ttf"),
    include_bytes!("../../../assets/fonts/inter/Inter-SemiBold.ttf"),
    include_bytes!("../../../assets/fonts/inter/Inter-Bold.ttf"),
    include_bytes!("../../../assets/fonts/geist-mono/GeistMono-Regular.ttf"),
    include_bytes!("../../../assets/fonts/geist-mono/GeistMono-Medium.ttf"),
    include_bytes!("../../../assets/fonts/geist-mono/GeistMono-SemiBold.ttf"),
    include_bytes!("../../../assets/fonts/geist-mono/GeistMono-Bold.ttf"),
];

/// Registers the bundled fonts with the app's text system.
///
/// The bytes are `'static`, so platforms that reference font data in place (DirectWrite's
/// in-memory loader) never outlive them. The web platform loads the same files itself when it
/// creates its text system.
pub(crate) fn init(cx: &mut App) {
    if cfg!(target_family = "wasm") {
        return;
    }
    let fonts = BUNDLED_FONTS
        .iter()
        .map(|bytes| Cow::Borrowed(*bytes))
        .collect();
    if let Err(error) = cx.text_system().add_fonts(fonts) {
        log::error!("failed to register bundled MoonUI fonts: {error:#}");
    }
}

#[cfg(test)]
mod tests;
