use gpui::*;

use super::tokens::rgba_from;

pub const MOON_ICON_CHECK: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/moon-check.svg");
pub const MOON_ICON_CARET_DOWN: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/assets/moon-caret-down.svg");
pub const MOON_ICON_TOOLTIP_ARROW: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/assets/moon-tooltip-arrow.svg");

pub fn moon_icon(path: &'static str, size: f32, color: u32, alpha: f32) -> Svg {
    svg()
        .w(px(size))
        .h(px(size))
        .text_color(rgba_from(color, alpha))
        .external_path(path)
}
