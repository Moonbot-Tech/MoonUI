use std::{fs, path::Path};

use gpui::{App, Global, SharedString, px};
use serde::{Deserialize, Serialize};

use super::{
    foundation::ThemeMode,
    tokens::{MoonMetrics, MoonPalette, rgba_from},
};
use crate::theme::{Theme as BaseTheme, ThemeColor, ThemeMode as BaseThemeMode};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct MoonScale {
    /// Multiplies component geometry: control heights, gaps, paddings and hit areas.
    pub ui: f32,
    /// Multiplies text metrics before applying [`font_delta`].
    pub font: f32,
    /// Adds logical pixels to all Moon text metrics. This is the user-facing
    /// "font +X" knob, kept separate from full UI zoom.
    pub font_delta: f32,
}

impl Default for MoonScale {
    fn default() -> Self {
        Self {
            ui: 1.0,
            font: 1.0,
            font_delta: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MoonTextMetrics {
    pub font_size: f32,
    pub line_height: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct MoonTypography {
    pub font_family: SharedString,
    pub mono_font_family: SharedString,
    pub font_size: f32,
    pub mono_font_size: f32,
    pub rem_size: f32,
}

impl Default for MoonTypography {
    fn default() -> Self {
        Self {
            font_family: SharedString::from("Inter"),
            mono_font_family: SharedString::from("Geist Mono"),
            font_size: 12.0,
            mono_font_size: 11.0,
            rem_size: 16.0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct MoonThemeTokens {
    pub palette: MoonPalette,
    pub scale: MoonScale,
    pub metrics: MoonMetrics,
    pub typography: MoonTypography,
}

impl Default for MoonThemeTokens {
    fn default() -> Self {
        Self {
            palette: MoonPalette::TERMINAL,
            scale: MoonScale::default(),
            metrics: MoonMetrics::TERMINAL,
            typography: MoonTypography::default(),
        }
    }
}

impl MoonThemeTokens {
    pub fn ui(&self, value: f32) -> f32 {
        value * self.scale.ui.max(0.25)
    }

    pub fn font(&self, value: f32) -> f32 {
        (value * self.scale.font.max(0.25) + self.scale.font_delta).max(1.0)
    }

    pub fn line_height(&self, value: f32) -> f32 {
        (value * self.scale.font.max(0.25) + self.scale.font_delta).max(1.0)
    }

    pub fn text(&self, font_size: f32, line_height: f32) -> MoonTextMetrics {
        MoonTextMetrics {
            font_size: self.font(font_size),
            line_height: self.line_height(line_height),
        }
    }

    pub fn font_family(&self, mono: bool) -> SharedString {
        if mono {
            self.typography.mono_font_family.clone()
        } else {
            self.typography.font_family.clone()
        }
    }

    pub fn base_font_size(&self) -> f32 {
        self.font(self.typography.font_size)
    }

    pub fn base_mono_font_size(&self) -> f32 {
        self.font(self.typography.mono_font_size)
    }

    fn theme_colors(&self) -> ThemeColor {
        let p = self.palette;

        ThemeColor {
            background: rgba_from(p.shell, 1.0),
            foreground: rgba_from(p.text, 1.0),
            border: rgba_from(p.border, 1.0),
            accent: rgba_from(p.accent, 1.0),
            accent_foreground: rgba_from(p.text, 1.0),
            accordion: rgba_from(p.panel, 1.0),
            accordion_hover: rgba_from(p.panel_high, 1.0),
            button_primary: rgba_from(p.accent, 1.0),
            button_primary_active: rgba_from(p.orange, 1.0),
            button_primary_foreground: rgba_from(p.text, 1.0),
            button_primary_hover: rgba_from(p.amber, 1.0),
            group_box: rgba_from(p.panel, 1.0),
            group_box_foreground: rgba_from(p.text, 1.0),
            caret: rgba_from(p.text, 1.0),
            chart_1: rgba_from(p.blue, 1.0),
            chart_2: rgba_from(p.green, 1.0),
            chart_3: rgba_from(p.amber, 1.0),
            chart_4: rgba_from(p.orange, 1.0),
            chart_5: rgba_from(p.red, 1.0),
            chart_bullish: rgba_from(p.green, 1.0),
            chart_bearish: rgba_from(p.orange, 1.0),
            danger: rgba_from(p.red, 1.0),
            danger_active: rgba_from(p.red, 0.92),
            danger_foreground: rgba_from(p.text, 1.0),
            danger_hover: rgba_from(p.red, 0.72),
            description_list_label: rgba_from(p.panel, 1.0),
            description_list_label_foreground: rgba_from(p.text_soft, 1.0),
            drag_border: rgba_from(p.accent, 1.0),
            drop_target: rgba_from(p.accent, 0.18),
            info: rgba_from(p.blue, 0.22),
            info_active: rgba_from(p.blue, 0.34),
            info_foreground: rgba_from(p.blue, 1.0),
            info_hover: rgba_from(p.blue, 0.28),
            input: rgba_from(p.panel_high, 1.0),
            link: rgba_from(p.blue, 1.0),
            link_active: rgba_from(p.amber, 1.0),
            link_hover: rgba_from(p.blue, 0.82),
            list: rgba_from(p.panel, 1.0),
            list_active: rgba_from(p.table_selected, 1.0),
            list_active_border: rgba_from(p.accent, 1.0),
            list_even: rgba_from(p.shell_high, 1.0),
            list_head: rgba_from(p.table_head, 1.0),
            list_hover: rgba_from(p.panel_high, 1.0),
            muted: rgba_from(p.panel_high, 1.0),
            muted_foreground: rgba_from(p.text_muted, 1.0),
            popover: rgba_from(p.panel_high, 1.0),
            popover_foreground: rgba_from(p.text, 1.0),
            primary: rgba_from(p.accent, 1.0),
            primary_active: rgba_from(p.orange, 1.0),
            primary_foreground: rgba_from(p.text, 1.0),
            primary_hover: rgba_from(p.amber, 1.0),
            progress_bar: rgba_from(p.green, 1.0),
            ring: rgba_from(p.blue, 1.0),
            scrollbar: rgba_from(p.panel_high, 0.34),
            scrollbar_thumb: rgba_from(p.text_muted, 0.58),
            scrollbar_thumb_hover: rgba_from(p.text_soft, 0.72),
            secondary: rgba_from(p.panel, 1.0),
            secondary_active: rgba_from(p.panel_high, 1.0),
            secondary_foreground: rgba_from(p.text_soft, 1.0),
            secondary_hover: rgba_from(p.panel_high, 1.0),
            selection: rgba_from(p.blue, 0.32),
            sidebar: rgba_from(p.shell_high, 1.0),
            sidebar_accent: rgba_from(p.table_selected, 1.0),
            sidebar_accent_foreground: rgba_from(p.text, 1.0),
            sidebar_border: rgba_from(p.border, 1.0),
            sidebar_foreground: rgba_from(p.text_soft, 1.0),
            sidebar_primary: rgba_from(p.accent, 1.0),
            sidebar_primary_foreground: rgba_from(p.text, 1.0),
            skeleton: rgba_from(p.panel_high, 1.0),
            slider_bar: rgba_from(p.border, 1.0),
            slider_thumb: rgba_from(p.orange, 1.0),
            success: rgba_from(p.green, 1.0),
            success_foreground: rgba_from(p.text, 1.0),
            success_hover: rgba_from(p.green, 0.72),
            success_active: rgba_from(p.green, 0.92),
            switch: rgba_from(p.panel_high, 1.0),
            switch_thumb: rgba_from(p.text_soft, 1.0),
            tab: rgba_from(p.shell_high, 1.0),
            tab_active: rgba_from(p.panel, 1.0),
            tab_active_foreground: rgba_from(p.text, 1.0),
            tab_bar: rgba_from(p.shell_high, 1.0),
            tab_bar_segmented: rgba_from(p.panel, 1.0),
            tab_foreground: rgba_from(p.text_soft, 1.0),
            table: rgba_from(p.table_body, 1.0),
            table_active: rgba_from(p.table_selected, 1.0),
            table_active_border: rgba_from(p.accent, 1.0),
            table_even: rgba_from(p.shell_high, 1.0),
            table_head: rgba_from(p.table_head, 1.0),
            table_head_foreground: rgba_from(p.text_soft, 1.0),
            table_foot: rgba_from(p.table_head, 1.0),
            table_foot_foreground: rgba_from(p.text_soft, 1.0),
            table_hover: rgba_from(p.panel_high, 1.0),
            table_row_border: rgba_from(p.border, 1.0),
            title_bar: rgba_from(p.shell_high, 1.0),
            title_bar_border: rgba_from(p.border, 1.0),
            status_bar: rgba_from(p.shell_high, 1.0),
            status_bar_border: rgba_from(p.border, 1.0),
            tiles: rgba_from(p.panel, 1.0),
            warning: rgba_from(p.amber, 1.0),
            warning_active: rgba_from(p.orange, 1.0),
            warning_hover: rgba_from(p.amber, 0.72),
            warning_foreground: rgba_from(p.text, 1.0),
            overlay: rgba_from(p.shell, 0.72),
            window_border: rgba_from(p.border, 1.0),
            red: rgba_from(p.red, 1.0),
            red_light: rgba_from(p.red, 0.72),
            green: rgba_from(p.green, 1.0),
            green_light: rgba_from(p.green, 0.72),
            blue: rgba_from(p.blue, 1.0),
            blue_light: rgba_from(p.blue, 0.72),
            yellow: rgba_from(p.yellow, 1.0),
            yellow_light: rgba_from(p.yellow, 0.72),
            magenta: rgba_from(p.orange, 1.0),
            magenta_light: rgba_from(p.orange, 0.72),
            cyan: rgba_from(p.blue, 1.0),
            cyan_light: rgba_from(p.blue, 0.72),
        }
    }

    pub fn fit_height(&self, base_height: f32, base_line_height: f32, base_pad_y: f32) -> f32 {
        self.ui(base_height)
            .max(self.line_height(base_line_height) + self.ui(base_pad_y) * 2.0)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct MoonThemeConfig {
    pub mode: ThemeMode,
    pub dark: MoonThemeTokens,
    pub light: MoonThemeTokens,
}

impl Default for MoonThemeConfig {
    fn default() -> Self {
        Self {
            mode: ThemeMode::Dark,
            dark: MoonThemeTokens::default(),
            light: MoonThemeTokens {
                palette: MoonPalette::LIGHT,
                scale: MoonScale::default(),
                metrics: MoonMetrics::TERMINAL,
                typography: MoonTypography::default(),
            },
        }
    }
}

impl MoonThemeConfig {
    pub fn moon_terminal() -> Self {
        toml::from_str(include_str!("../../themes/moon-terminal.toml"))
            .expect("bundled moon-terminal theme must parse")
    }

    pub fn moon_light() -> Self {
        toml::from_str(include_str!("../../themes/moon-light.toml"))
            .expect("bundled moon-light theme must parse")
    }

    pub fn set_font_delta(&mut self, font_delta: f32) {
        self.dark.scale.font_delta = font_delta;
        self.light.scale.font_delta = font_delta;
    }

    pub fn with_font_delta(mut self, font_delta: f32) -> Self {
        self.set_font_delta(font_delta);
        self
    }

    pub fn set_ui_scale(&mut self, ui_scale: f32) {
        self.dark.scale.ui = ui_scale;
        self.light.scale.ui = ui_scale;
    }

    pub fn with_ui_scale(mut self, ui_scale: f32) -> Self {
        self.set_ui_scale(ui_scale);
        self
    }
}

#[derive(Clone, Debug)]
pub struct MoonTheme {
    pub mode: ThemeMode,
    pub palette: MoonPalette,
    pub scale: MoonScale,
    pub metrics: MoonMetrics,
    pub typography: MoonTypography,
    pub config: MoonThemeConfig,
}

impl Global for MoonTheme {}

impl Default for MoonTheme {
    fn default() -> Self {
        Self::from_config(MoonThemeConfig::default())
    }
}

impl MoonTheme {
    pub fn from_config(config: MoonThemeConfig) -> Self {
        let tokens = match config.mode {
            ThemeMode::Light => config.light.clone(),
            ThemeMode::Dark | ThemeMode::System => config.dark.clone(),
        };
        Self {
            mode: config.mode,
            palette: tokens.palette,
            scale: tokens.scale,
            metrics: tokens.metrics,
            typography: tokens.typography,
            config,
        }
    }

    pub fn install(cx: &mut App) {
        if !cx.has_global::<Self>() {
            let theme = Self::default();
            theme.sync_base_theme(cx);
            cx.set_global(theme);
        }
    }

    pub fn install_config(config: MoonThemeConfig, cx: &mut App) {
        let theme = Self::from_config(config);
        theme.sync_base_theme(cx);
        cx.set_global(theme);
    }

    pub fn load_toml(path: impl AsRef<Path>) -> Result<MoonThemeConfig, MoonThemeConfigError> {
        let text = fs::read_to_string(path).map_err(MoonThemeConfigError::Io)?;
        toml::from_str(&text).map_err(MoonThemeConfigError::Toml)
    }

    pub fn install_toml(path: impl AsRef<Path>, cx: &mut App) -> Result<(), MoonThemeConfigError> {
        let config = Self::load_toml(path)?;
        Self::install_config(config, cx);
        Ok(())
    }

    pub fn global(cx: &App) -> Option<&Self> {
        cx.try_global::<Self>()
    }

    pub fn global_mut(cx: &mut App) -> &mut Self {
        if !cx.has_global::<Self>() {
            cx.set_global(Self::default());
        }
        cx.global_mut::<Self>()
    }

    pub fn set_mode(mode: ThemeMode, cx: &mut App) {
        let config = {
            let theme = Self::global_mut(cx);
            theme.config.mode = mode;
            theme.config.clone()
        };
        let next = Self::from_config(config);
        next.sync_base_theme(cx);
        *Self::global_mut(cx) = next;
    }

    pub fn active_tokens(cx: &App) -> MoonThemeTokens {
        cx.try_global::<Self>()
            .map(|theme| MoonThemeTokens {
                palette: theme.palette,
                scale: theme.scale,
                metrics: theme.metrics,
                typography: theme.typography.clone(),
            })
            .unwrap_or_default()
    }

    fn tokens(&self) -> MoonThemeTokens {
        MoonThemeTokens {
            palette: self.palette,
            scale: self.scale,
            metrics: self.metrics,
            typography: self.typography.clone(),
        }
    }

    fn sync_base_theme(&self, cx: &mut App) {
        if !cx.has_global::<BaseTheme>() {
            cx.set_global(BaseTheme::default());
        }
        let tokens = self.tokens();
        let base = BaseTheme::global_mut(cx);

        base.mode = match self.mode {
            ThemeMode::Light => BaseThemeMode::Light,
            ThemeMode::Dark | ThemeMode::System => BaseThemeMode::Dark,
        };
        base.font_family = tokens.typography.font_family.clone();
        base.mono_font_family = tokens.typography.mono_font_family.clone();
        base.font_size = px(tokens.base_font_size());
        base.mono_font_size = px(tokens.base_mono_font_size());
        base.radius = px(tokens.ui(self.metrics.button_radius));
        base.radius_lg = px(tokens.ui(self.metrics.button_radius + 2.0));
        base.colors = tokens.theme_colors();
    }
}

#[derive(Debug)]
pub enum MoonThemeConfigError {
    Io(std::io::Error),
    Toml(toml::de::Error),
}

impl std::fmt::Display for MoonThemeConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "failed to read Moon theme config: {err}"),
            Self::Toml(err) => write!(f, "failed to parse Moon theme config TOML: {err}"),
        }
    }
}

impl std::error::Error for MoonThemeConfigError {}
