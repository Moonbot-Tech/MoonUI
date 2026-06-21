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

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct MoonPalette {
    pub shell: u32,
    pub shell_high: u32,
    pub panel: u32,
    pub panel_high: u32,
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
        panel: 0x20232A,
        panel_high: 0x23262D,
        chart_bg: 0x14171B,
        border: 0x2A2D31,
        border_hover: 0x343840,
        shadow: 0x000000,
        overlay: 0xFFFFFF,
        on_accent: 0xFFFFFF,
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

    pub const LIGHT: Self = Self {
        shell: 0xF3F5F7,
        shell_high: 0xFFFFFF,
        panel: 0xE7EBEF,
        panel_high: 0xFFFFFF,
        chart_bg: 0xF5F7FA,
        border: 0xCDD3DA,
        border_hover: 0xB8C0C8,
        shadow: 0x000000,
        overlay: 0x000000,
        on_accent: 0xFFFFFF,
        text: 0x18202A,
        text_soft: 0x4F5B68,
        text_muted: 0x7A8591,
        table_head: 0xEEF2F5,
        table_body: 0xFFFFFF,
        table_selected: 0xE7F1FF,
        green: 0x168A49,
        red: 0xD92D3A,
        orange: 0xC85F17,
        amber: 0xB97800,
        blue: 0x126CBF,
        accent: 0xB95C18,
        yellow: 0xB48A00,
    };

    pub fn active(cx: &App) -> Self {
        super::theme::MoonTheme::global(cx)
            .map(|theme| theme.palette)
            .unwrap_or(Self::TERMINAL)
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
