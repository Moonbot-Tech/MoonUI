use std::{fs, path::Path};

use gpui::{App, Global, SharedString, px};
use serde::{Deserialize, Serialize};

use super::{
    foundation::ThemeMode,
    tokens::{MoonMetrics, MoonPalette},
};
use crate::theme::Theme as BaseTheme;

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
        base.font_family = tokens.typography.font_family.clone();
        base.mono_font_family = tokens.typography.mono_font_family.clone();
        base.font_size = px(tokens.base_font_size());
        base.mono_font_size = px(tokens.base_mono_font_size());
        base.radius = px(tokens.ui(self.metrics.button_radius));
        base.radius_lg = px(tokens.ui(self.metrics.button_radius + 2.0));
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
