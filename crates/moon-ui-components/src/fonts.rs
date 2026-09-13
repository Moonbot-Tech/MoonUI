//! Bundled UI fonts, registered with the text system so MoonUI renders in Inter and Geist Mono
//! without the user installing either.

use std::borrow::Cow;

use gpui::App;

/// Inter (SIL Open Font License, see `assets/fonts/inter/OFL.txt`) at the weights MoonUI uses,
/// and Geist Mono for mono text.
///
/// The Inter files are the 18pt optical-size cut with their family renamed from "Inter 18pt" to
/// "Inter", so the theme's `"Inter"` family resolves to them on every platform.
pub(crate) const BUNDLED_FONTS: &[&[u8]] = &[
    include_bytes!("../../../assets/fonts/inter/Inter-Regular.ttf"),
    include_bytes!("../../../assets/fonts/inter/Inter-Medium.ttf"),
    include_bytes!("../../../assets/fonts/inter/Inter-SemiBold.ttf"),
    include_bytes!("../../../assets/fonts/inter/Inter-Bold.ttf"),
    include_bytes!("../../../assets/fonts/geist-mono/GeistMono-Regular.ttf"),
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
