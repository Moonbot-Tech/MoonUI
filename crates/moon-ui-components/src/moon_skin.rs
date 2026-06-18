use gpui::{Hsla, rgb};

#[derive(Clone, Copy)]
#[allow(dead_code)]
pub(crate) struct MoonSkinPalette {
    pub shell: u32,
    pub shell_high: u32,
    pub panel: u32,
    pub panel_high: u32,
    pub border: u32,
    pub text: u32,
    pub text_soft: u32,
    pub text_muted: u32,
    pub table_head: u32,
    pub table_body: u32,
    pub table_selected: u32,
    pub green: u32,
    pub red: u32,
    pub orange: u32,
    pub amber: u32,
    pub blue: u32,
    pub accent: u32,
    pub yellow: u32,
}

impl MoonSkinPalette {
    pub const TERMINAL: Self = Self {
        shell: 0x131416,
        shell_high: 0x1A1C1F,
        panel: 0x20232A,
        panel_high: 0x23262D,
        border: 0x2A2D31,
        text: 0xE8E4DC,
        text_soft: 0x97928A,
        text_muted: 0x5E5A53,
        table_head: 0x20232A,
        table_body: 0x1A1C1F,
        table_selected: 0x212120,
        green: 0x2FA85C,
        red: 0xFF4A4A,
        orange: 0xFF8E5A,
        amber: 0xFFB347,
        blue: 0x7FC9FF,
        accent: 0xD2691E,
        yellow: 0xFFD93D,
    };
}

pub(crate) fn moon_color(rgb_hex: u32, alpha: f32) -> Hsla {
    let color: Hsla = rgb(rgb_hex).into();
    color.opacity(alpha)
}
