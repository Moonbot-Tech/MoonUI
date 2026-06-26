use gpui::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct MoonRect {
    /// GPUI logical pixels, matching CSS px in the designer reference at 1x.
    /// This is not a physical monitor pixel coordinate.
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl MoonRect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MoonPalette {
    pub shell: u32,
    pub shell_high: u32,
    pub surface: u32,
    pub panel: u32,
    pub panel_high: u32,
    pub chrome: u32,
    pub panel_head: u32,
    pub gutter: u32,
    pub chart_bg: u32,
    pub border: u32,
    pub border_hover: u32,
    pub shadow: u32,
    pub overlay: u32,
    pub on_accent: u32,
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
    pub accent_fg: u32,
    pub accent_tint_a: f32,
    pub yellow: u32,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct MoonMetrics {
    pub header_top_h: f32,
    pub toolbar_h: f32,
    pub status_h: f32,
    pub dock_tab_h: f32,
    pub table_header_h: f32,
    pub table_row_h: f32,
    pub button_radius: f32,
    pub container_radius: f32,
    pub hairline: f32,
}

impl MoonMetrics {
    /// Geometry extracted from `MoonBot Terminal Design.html` at 1x CSS px.
    pub const TERMINAL: Self = Self {
        header_top_h: 32.0,
        toolbar_h: 32.0,
        status_h: 22.0,
        dock_tab_h: 26.0,
        table_header_h: 26.0,
        table_row_h: 25.0,
        button_radius: 4.0,
        container_radius: 8.0,
        hairline: 1.0,
    };

    pub fn active(cx: &App) -> Self {
        super::theme::MoonTheme::global(cx)
            .map(|theme| theme.metrics)
            .unwrap_or(Self::TERMINAL)
    }
}

impl MoonPalette {
    pub const TERMINAL: Self = Self {
        shell: 0x131416,
        shell_high: 0x1A1C1F,
        surface: 0x16181B,
        panel: 0x20232A,
        panel_high: 0x22252B,
        chrome: 0x1A1C1F,
        panel_head: 0x22252B,
        gutter: 0x0F1012,
        chart_bg: 0x16181B,
        border: 0x2A2D31,
        border_hover: 0x343840,
        shadow: 0x000000,
        overlay: 0xFFFFFF,
        on_accent: 0xFFFFFF,
        text: 0xE8E4DC,
        text_soft: 0x97928A,
        text_muted: 0x7D7669,
        table_head: 0x20232A,
        table_body: 0x1A1C1F,
        table_selected: 0xFFB347,
        green: 0x1E8C5B,
        red: 0xE5484D,
        orange: 0xFF8E5A,
        amber: 0xFFB347,
        blue: 0x7FC9FF,
        accent: 0xFFB347,
        accent_fg: 0xFFCF94,
        accent_tint_a: 0.11,
        yellow: 0xFFD93D,
    };

    pub const LIGHT: Self = Self {
        shell: 0xEAEAEC,
        shell_high: 0xF0F0F2,
        surface: 0xFFFFFF,
        panel: 0xF2F2F4,
        panel_high: 0xE8E8EB,
        chrome: 0xF0F0F2,
        panel_head: 0xE8E8EB,
        gutter: 0xE1E1E4,
        chart_bg: 0xFFFFFF,
        border: 0xD7D7DB,
        border_hover: 0xC4C4CB,
        shadow: 0x000000,
        overlay: 0x000000,
        on_accent: 0xFFFFFF,
        text: 0x17171A,
        text_soft: 0x5C5C62,
        text_muted: 0x6E6E76,
        table_head: 0xE8E8EB,
        table_body: 0xFFFFFF,
        table_selected: 0x8A6326,
        green: 0x0E9F6E,
        red: 0xE5484D,
        orange: 0xC85F17,
        amber: 0x8A6326,
        blue: 0x2563EB,
        accent: 0x8A6326,
        accent_fg: 0x73521D,
        accent_tint_a: 0.10,
        yellow: 0xB48A00,
    };

    pub fn active(cx: &App) -> Self {
        super::theme::MoonTheme::global(cx)
            .map(|theme| theme.palette)
            .unwrap_or(Self::TERMINAL)
    }

    pub fn is_light(self) -> bool {
        let r = ((self.shell >> 16) & 0xFF) as f32;
        let g = ((self.shell >> 8) & 0xFF) as f32;
        let b = (self.shell & 0xFF) as f32;
        (0.2126 * r + 0.7152 * g + 0.0722 * b) >= 128.0
    }

    pub fn selected_fg(self) -> u32 {
        if self.is_light() {
            self.text
        } else {
            self.accent_fg
        }
    }
}

impl Default for MoonPalette {
    fn default() -> Self {
        Self::TERMINAL
    }
}

impl Default for MoonMetrics {
    fn default() -> Self {
        Self::TERMINAL
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MoonTone {
    Default,
    Muted,
    Positive,
    Negative,
    Warning,
    Info,
    Danger,
    Accent,
    Notice,
}

impl MoonTone {
    pub fn color(self, palette: MoonPalette) -> u32 {
        match self {
            Self::Default => palette.text,
            Self::Muted => palette.text_soft,
            Self::Positive => palette.green,
            Self::Negative => palette.orange,
            Self::Warning => palette.amber,
            Self::Info => palette.blue,
            Self::Danger => palette.red,
            Self::Accent => palette.accent,
            Self::Notice => palette.yellow,
        }
    }
}

pub fn rgba_from(rgb_hex: u32, alpha: f32) -> Hsla {
    rgba((rgb_hex << 8) | ((alpha * 255.0).round() as u32)).into()
}

pub fn rgb_from(rgb_hex: u32) -> Hsla {
    rgb(rgb_hex).into()
}
