use gpui::*;

use super::tokens::rgba_from;

// Chrome/control icon paths, resolved against the embedded MoonAssets source
// (the `moon-ui-components-assets` RustEmbed set: `icons/**/*.svg`) via
// `svg().path(...)`. They must NOT use `external_path`/`CARGO_MANIFEST_DIR`:
// that bakes the build machine's absolute path into the binary, which does not
// exist on any other machine, so a distributed build renders the glyph blank.
pub const MOON_ICON_CHECK: &str = "icons/moon-check.svg";
pub const MOON_ICON_CARET_DOWN: &str = "icons/moon-caret-down.svg";
pub const MOON_ICON_TOOLTIP_ARROW: &str = "icons/moon-tooltip-arrow.svg";
pub const MOON_ICON_WINDOW_CLOSE: &str = "icons/moon-window-close.svg";

pub fn moon_icon(path: &'static str, size: f32, color: u32, alpha: f32) -> Svg {
    svg()
        .w(px(size))
        .h(px(size))
        .text_color(rgba_from(color, alpha))
        .path(path)
}
