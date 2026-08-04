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

    /// The ink for a row marked as selected by [`selected_background`].
    ///
    /// That fill is an accent *tint* — 11% accent fading to nothing — so this ink lands on the
    /// panel underneath, not on the accent. Do not "fix" it by measuring against the accent: a
    /// dark ink chosen for the accent is black text on a dark panel, which is every selected list,
    /// menu and dropdown row turned unreadable. When the fill really is solid, ask [`Self::ink_on`].
    ///
    /// [`selected_background`]: super::foundation::selected_background
    pub fn selected_fg(self) -> u32 {
        if self.is_light() {
            self.text
        } else {
            self.accent_fg
        }
    }

    /// The ink to print on top of a **solid** `fill`.
    ///
    /// Chosen by measured contrast rather than by "which theme is this": the dark palette's accent
    /// is a *light* orange, so the theme-shaped answer (`accent_fg`, `#FFCF94`) on a solid accent
    /// lands at 1.24:1. Candidates are the palette's own inks, so a custom palette gets the same
    /// treatment instead of the same assumption.
    ///
    /// Args:
    ///     fill: The `0xRRGGBB` background the text is painted on. Only pass an opaque fill —
    ///         a translucent one is not the colour the eye sees.
    ///
    /// Returns:
    ///     The palette ink with the highest contrast against `fill`.
    pub fn ink_on(self, fill: u32) -> u32 {
        [self.on_accent, self.shell, self.text, self.accent_fg]
            .into_iter()
            .max_by(|a, b| contrast_ratio(*a, fill).total_cmp(&contrast_ratio(*b, fill)))
            .unwrap_or(self.on_accent)
    }
}

/// WCAG 2.x relative luminance of an `0xRRGGBB` colour.
///
/// Deliberately not routed through this crate's other sRGB decode (`theme::color`'s oklab path):
/// that one uses the sRGB spec's 0.04045 knee, while WCAG 2.x specifies 0.03928. The two agree to
/// well within a rounding step for every colour in the palettes, but this function exists to answer
/// a WCAG question, so it follows the WCAG definition rather than borrowing a near-neighbour.
pub(crate) fn relative_luminance(color: u32) -> f32 {
    let channel = |shift: u32| {
        let c = ((color >> shift) & 0xFF) as f32 / 255.0;
        if c <= 0.03928 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(16) + 0.7152 * channel(8) + 0.0722 * channel(0)
}

/// WCAG 2.x contrast ratio between two opaque `0xRRGGBB` colours, from 1.0 to 21.0.
///
/// Symmetric in its arguments — the brighter colour is found, not assumed to be either one.
pub(crate) fn contrast_ratio(a: u32, b: u32) -> f32 {
    let (la, lb) = (relative_luminance(a), relative_luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
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
mod tests;
