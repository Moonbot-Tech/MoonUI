use std::{fs, path::Path};

use gpui::{App, Global, SharedString};
use serde::{Deserialize, Serialize};

use super::{
    foundation::ThemeMode,
    tokens::{MoonMetrics, MoonPalette},
};

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
    pub metrics: MoonMetrics,
    pub typography: MoonTypography,
}

impl Default for MoonThemeTokens {
    fn default() -> Self {
        Self {
            palette: MoonPalette::TERMINAL,
            metrics: MoonMetrics::TERMINAL,
            typography: MoonTypography::default(),
        }
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
}

#[derive(Clone, Debug)]
pub struct MoonTheme {
    pub mode: ThemeMode,
    pub palette: MoonPalette,
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
            metrics: tokens.metrics,
            typography: tokens.typography,
            config,
        }
    }

    pub fn install(cx: &mut App) {
        if !cx.has_global::<Self>() {
            cx.set_global(Self::default());
        }
    }

    pub fn install_config(config: MoonThemeConfig, cx: &mut App) {
        cx.set_global(Self::from_config(config));
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
        let theme = Self::global_mut(cx);
        theme.config.mode = mode;
        *theme = Self::from_config(theme.config.clone());
    }

    pub fn active_tokens(cx: &App) -> MoonThemeTokens {
        cx.try_global::<Self>()
            .map(|theme| MoonThemeTokens {
                palette: theme.palette,
                metrics: theme.metrics,
                typography: theme.typography.clone(),
            })
            .unwrap_or_default()
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
