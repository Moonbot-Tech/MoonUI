use gpui::{App, Hsla, rgb, rgba};
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
    pub window: u32,
    pub surface: u32,
    pub panel: u32,
    pub panel_high: u32,
    pub chrome: u32,
    pub tabbar: u32,
    pub panel_head: u32,
    pub gutter: u32,
    pub chart_bg: u32,
    pub card: u32,
    pub row_alt: u32,
    pub head_row: u32,
    pub border: u32,
    pub border_soft: u32,
    pub border_card: u32,
    pub border_hover: u32,
    pub row_line: u32,
    pub shadow: u32,
    pub overlay: u32,
    pub on_accent: u32,
    pub text: u32,
    pub text_soft: u32,
    pub text_dim: u32,
    pub text_muted: u32,
    pub text_faint: u32,
    pub table_head: u32,
    pub table_body: u32,
    pub table_selected: u32,
    pub green: u32,
    pub green_btn: u32,
    pub green_text: u32,
    pub red: u32,
    pub red_text: u32,
    pub red_soft_bd: u32,
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
        window: 0x131416,
        surface: 0x16181B,
        panel: 0x20232A,
        panel_high: 0x22252B,
        chrome: 0x1A1C1F,
        tabbar: 0x1A1C1F,
        panel_head: 0x22252B,
        gutter: 0x0F1012,
        chart_bg: 0x16181B,
        card: 0x1A1C1F,
        row_alt: 0x1A1C1F,
        head_row: 0x20232A,
        border: 0x2A2D31,
        border_soft: 0x2A2D31,
        border_card: 0x2A2D31,
        border_hover: 0x343840,
        row_line: 0x2A2D31,
        shadow: 0x000000,
        overlay: 0xFFFFFF,
        on_accent: 0xFFFFFF,
        text: 0xE8E4DC,
        text_soft: 0x97928A,
        text_dim: 0xE8E4DC,
        text_muted: 0x7D7669,
        text_faint: 0x7D7669,
        table_head: 0x20232A,
        table_body: 0x1A1C1F,
        table_selected: 0xFFB347,
        green: 0x1E8C5B,
        green_btn: 0x1E8C5B,
        green_text: 0x1E8C5B,
        red: 0xE5484D,
        red_text: 0xE5484D,
        red_soft_bd: 0xE5484D,
        orange: 0xFF8E5A,
        amber: 0xFFB347,
        blue: 0x7FC9FF,
        accent: 0xFFB347,
        accent_fg: 0xFFCF94,
        accent_tint_a: 0.11,
        yellow: 0xFFD93D,
    };

    pub const LIGHT: Self = Self {
        shell: 0xF3F5F7,
        shell_high: 0xFAFBFC,
        window: 0xF7F8FA,
        surface: 0xFFFFFF,
        panel: 0xF8FAFC,
        panel_high: 0xFFFFFF,
        chrome: 0xF5F7FA,
        tabbar: 0xF2F5F8,
        panel_head: 0xF5F7FA,
        gutter: 0xEEF2F6,
        chart_bg: 0xFFFFFF,
        card: 0xFFFFFF,
        row_alt: 0xFCFDFE,
        head_row: 0xF3F6F8,
        border: 0xD5DBE1,
        border_soft: 0xE1E5EA,
        border_card: 0xDCE2E8,
        border_hover: 0xB8C2CC,
        row_line: 0xECEFF2,
        shadow: 0x000000,
        overlay: 0x000000,
        on_accent: 0xFFFFFF,
        text: 0x17202A,
        text_soft: 0x4B5865,
        text_dim: 0x2D3945,
        text_muted: 0x768391,
        text_faint: 0x98A3AE,
        table_head: 0xF3F6F8,
        table_body: 0xFFFFFF,
        table_selected: 0x009DFF,
        green: 0x178A57,
        green_btn: 0x178A57,
        green_text: 0x0E6E45,
        red: 0xD2483F,
        red_text: 0xB7352F,
        red_soft_bd: 0xE1B5B0,
        orange: 0xD18A2B,
        amber: 0xB97824,
        blue: 0x2B6F9E,
        accent: 0x009DFF,
        accent_fg: 0x0A3F68,
        accent_tint_a: 0.08,
        yellow: 0xB8860B,
    };

    pub fn with_legacy_defaults(mut self) -> Self {
        if self.window == 0 {
            self.window = self.shell;
        }
        if self.tabbar == 0 {
            self.tabbar = self.chrome;
        }
        if self.card == 0 {
            self.card = self.table_body;
        }
        if self.row_alt == 0 {
            self.row_alt = self.chrome;
        }
        if self.head_row == 0 {
            self.head_row = self.table_head;
        }
        if self.border_soft == 0 {
            self.border_soft = self.border;
        }
        if self.border_card == 0 {
            self.border_card = self.border;
        }
        if self.row_line == 0 {
            self.row_line = self.border;
        }
        if self.text_dim == 0 {
            self.text_dim = self.text;
        }
        if self.text_faint == 0 {
            self.text_faint = self.text_muted;
        }
        if self.green_btn == 0 {
            self.green_btn = self.green;
        }
        if self.green_text == 0 {
            self.green_text = self.green;
        }
        if self.red_text == 0 {
            self.red_text = self.red;
        }
        if self.red_soft_bd == 0 {
            self.red_soft_bd = self.red;
        }
        self
    }

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
            Self::Positive => {
                if palette.is_light() {
                    palette.green_text
                } else {
                    palette.green
                }
            }
            Self::Negative => {
                if palette.is_light() {
                    palette.red_text
                } else {
                    palette.orange
                }
            }
            Self::Warning => palette.amber,
            Self::Info => {
                if palette.is_light() {
                    palette.blue
                } else {
                    palette.blue
                }
            }
            Self::Danger => {
                if palette.is_light() {
                    palette.red_text
                } else {
                    palette.red
                }
            }
            Self::Accent => {
                if palette.is_light() {
                    palette.accent_fg
                } else {
                    palette.accent
                }
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dark_terminal_palette_keeps_legacy_core_values() {
        let p = MoonPalette::TERMINAL;
        assert_eq!(p.shell, 0x131416);
        assert_eq!(p.shell_high, 0x1A1C1F);
        assert_eq!(p.panel, 0x20232A);
        assert_eq!(p.border, 0x2A2D31);
        assert_eq!(p.text, 0xE8E4DC);
        assert_eq!(p.green, 0x1E8C5B);
        assert_eq!(p.red, 0xE5484D);
        assert_eq!(p.orange, 0xFF8E5A);
        assert_eq!(p.blue, 0x7FC9FF);
        assert_eq!(p.accent, 0xFFB347);
    }

    #[test]
    fn light_palette_matches_neutral_terminal_spec() {
        let p = MoonPalette::LIGHT;
        assert_eq!(p.shell, 0xF3F5F7);
        assert_eq!(p.window, 0xF7F8FA);
        assert_eq!(p.chrome, 0xF5F7FA);
        assert_eq!(p.tabbar, 0xF2F5F8);
        assert_eq!(p.surface, 0xFFFFFF);
        assert_eq!(p.card, 0xFFFFFF);
        assert_eq!(p.row_alt, 0xFCFDFE);
        assert_eq!(p.head_row, 0xF3F6F8);
        assert_eq!(p.gutter, 0xEEF2F6);
        assert_eq!(p.border, 0xD5DBE1);
        assert_eq!(p.border_soft, 0xE1E5EA);
        assert_eq!(p.border_card, 0xDCE2E8);
        assert_eq!(p.row_line, 0xECEFF2);
        assert_eq!(p.text, 0x17202A);
        assert_eq!(p.text_soft, 0x4B5865);
        assert_eq!(p.text_dim, 0x2D3945);
        assert_eq!(p.text_muted, 0x768391);
        assert_eq!(p.text_faint, 0x98A3AE);
        assert_eq!(p.accent, 0x009DFF);
        assert_eq!(p.accent_fg, 0x0A3F68);
        assert_ne!(p.accent, p.accent_fg);
        assert_eq!(MoonTone::Accent.color(p), p.accent_fg);
        assert_eq!(MoonTone::Info.color(p), p.blue);
        assert_eq!(p.green_text, 0x0E6E45);
        assert_eq!(p.green_btn, 0x178A57);
        assert_eq!(p.red, 0xD2483F);
        assert_eq!(p.red_text, 0xB7352F);
        assert_eq!(p.red_soft_bd, 0xE1B5B0);
    }

    #[test]
    fn legacy_palette_defaults_fill_new_roles() {
        let legacy = MoonPalette {
            window: 0,
            tabbar: 0,
            card: 0,
            row_alt: 0,
            head_row: 0,
            border_soft: 0,
            border_card: 0,
            row_line: 0,
            text_dim: 0,
            text_faint: 0,
            green_btn: 0,
            green_text: 0,
            red_text: 0,
            red_soft_bd: 0,
            ..MoonPalette::TERMINAL
        }
        .with_legacy_defaults();

        assert_eq!(legacy.window, MoonPalette::TERMINAL.shell);
        assert_eq!(legacy.tabbar, MoonPalette::TERMINAL.chrome);
        assert_eq!(legacy.card, MoonPalette::TERMINAL.table_body);
        assert_eq!(legacy.green_text, MoonPalette::TERMINAL.green);
        assert_eq!(legacy.red_text, MoonPalette::TERMINAL.red);
    }
}
