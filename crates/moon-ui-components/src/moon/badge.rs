//! Text-height badges that follow density tiers, with explicit legacy custom sizing.

use gpui::prelude::FluentBuilder;
use gpui::*;

use super::{
    foundation::MoonSize,
    icons::moon_icon,
    theme::{MoonTheme, MoonThemeTokens},
    tokens::{MoonPalette, MoonRect, MoonTone, rgba_from},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoonBadgeVariant {
    Solid,
    Soft,
    Outline,
}

/// Shared density tier or explicit badge geometry and legacy-scaled text.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MoonBadgeSize {
    /// Xs, Sm and Md; larger requests snap to Md.
    Tier(MoonSize),
    Custom {
        height: f32,
        radius: f32,
        font_size: f32,
        line_height: f32,
        pad_x: f32,
        min_width: f32,
    },
}

impl From<MoonSize> for MoonBadgeSize {
    /// Convert a shared tier; unsupported tiers snap when metrics are resolved.
    fn from(size: MoonSize) -> Self {
        Self::Tier(size)
    }
}

/// Badge geometry and text metrics in design-reference or resolved pixels.
#[derive(Clone, Copy, Debug)]
struct BadgeMetrics {
    height: f32,
    radius: f32,
    font_size: f32,
    line_height: f32,
    pad_x: f32,
    min_width: f32,
}

impl BadgeMetrics {
    /// Resolve custom geometry and legacy text scaling, expanding the box to fit its line.
    fn scaled(self, tokens: &MoonThemeTokens) -> Self {
        let line_height = tokens.line_height(self.line_height);
        let pad_y = ((self.height - self.line_height) * 0.5).max(0.0);
        Self {
            height: tokens
                .ui(self.height)
                .max(line_height + tokens.ui(pad_y) * 2.0),
            radius: tokens.ui(self.radius),
            font_size: tokens.font(self.font_size),
            line_height,
            pad_x: tokens.ui(self.pad_x),
            min_width: tokens.ui(self.min_width),
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct BadgeStyle {
    bg: u32,
    bg_alpha: f32,
    border: u32,
    border_alpha: f32,
    fg: u32,
    fg_alpha: f32,
}

#[derive(Clone, Debug)]
pub enum MoonBadgeContent {
    Text(SharedString),
    Dot,
    Count { value: usize, max: Option<usize> },
    Icon(&'static str),
}

/// Theme-aware text, count, dot or icon badge with density-driven chrome.
#[derive(IntoElement)]
pub struct MoonBadge {
    bounds: Option<MoonRect>,
    content: MoonBadgeContent,
    tone: MoonTone,
    variant: MoonBadgeVariant,
    size: Option<MoonBadgeSize>,
    disabled: bool,
    mono: bool,
    weight: f32,
    margin_top: f32,
    bg: Option<u32>,
    bg_alpha: Option<f32>,
    border: Option<u32>,
    border_alpha: Option<f32>,
    fg: Option<u32>,
    fg_alpha: Option<f32>,
}

impl MoonBadge {
    /// Create a badge using the active density unless an explicit size is supplied.
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            bounds: None,
            content: MoonBadgeContent::Text(label.into()),
            tone: MoonTone::Default,
            variant: MoonBadgeVariant::Soft,
            size: None,
            disabled: false,
            mono: true,
            weight: 500.0,
            margin_top: 0.0,
            bg: None,
            bg_alpha: None,
            border: None,
            border_alpha: None,
            fg: None,
            fg_alpha: None,
        }
    }

    pub fn bounds(mut self, bounds: MoonRect) -> Self {
        self.bounds = Some(bounds);
        self
    }

    pub fn content(mut self, content: MoonBadgeContent) -> Self {
        self.content = content;
        self
    }

    pub fn dot(mut self) -> Self {
        self.content = MoonBadgeContent::Dot;
        self
    }

    pub fn count(mut self, value: usize) -> Self {
        self.content = MoonBadgeContent::Count { value, max: None };
        self
    }

    pub fn count_max(mut self, value: usize, max: usize) -> Self {
        self.content = MoonBadgeContent::Count {
            value,
            max: Some(max),
        };
        self
    }

    pub fn icon(mut self, path: &'static str) -> Self {
        self.content = MoonBadgeContent::Icon(path);
        self
    }

    pub fn tone(mut self, tone: MoonTone) -> Self {
        self.tone = tone;
        self
    }

    pub fn variant(mut self, variant: MoonBadgeVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Override density using a shared tier or custom design-reference metrics.
    pub fn size(mut self, size: MoonBadgeSize) -> Self {
        self.size = Some(size);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn mono(mut self, mono: bool) -> Self {
        self.mono = mono;
        self
    }

    pub fn weight(mut self, weight: f32) -> Self {
        self.weight = weight;
        self
    }

    pub fn margin_top(mut self, margin_top: f32) -> Self {
        self.margin_top = margin_top;
        self
    }

    pub fn bg_color(mut self, bg: u32) -> Self {
        self.bg = Some(bg);
        self
    }

    pub fn bg_alpha(mut self, alpha: f32) -> Self {
        self.bg_alpha = Some(alpha);
        self
    }

    pub fn border_color(mut self, border: u32) -> Self {
        self.border = Some(border);
        self
    }

    pub fn border_alpha(mut self, alpha: f32) -> Self {
        self.border_alpha = Some(alpha);
        self
    }

    pub fn text_color(mut self, fg: u32) -> Self {
        self.fg = Some(fg);
        self
    }

    pub fn text_alpha(mut self, alpha: f32) -> Self {
        self.fg_alpha = Some(alpha);
        self
    }

    pub fn render(self) -> impl IntoElement {
        self
    }

    pub fn render_with_palette(self, p: MoonPalette) -> impl IntoElement {
        self.render_with_theme(p, MoonThemeTokens::default())
    }

    /// Render with the supplied palette and tokens, scaling text and chrome exactly once.
    pub fn render_with_theme(self, p: MoonPalette, tokens: MoonThemeTokens) -> impl IntoElement {
        let metrics = self.metrics(&tokens);
        let style = self.style(p);
        let disabled_scale = if self.disabled { 0.5 } else { 1.0 };
        let border_alpha = style.border_alpha * disabled_scale;

        let is_dot = matches!(self.content, MoonBadgeContent::Dot);
        let mut badge = div()
            .relative()
            .h(px(metrics.height))
            .min_w(px(if is_dot {
                metrics.height
            } else {
                metrics.min_width
            }))
            .when(!is_dot, |this| this.px(px(metrics.pad_x)))
            .mt(px(tokens.ui(self.margin_top)))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(metrics.radius))
            .bg(rgba_from(style.bg, style.bg_alpha * disabled_scale))
            .whitespace_nowrap()
            .when(border_alpha > 0.0, |this| {
                this.border(px(1.0))
                    .border_color(rgba_from(style.border, border_alpha))
            });

        if let Some(bounds) = self.bounds {
            badge = badge
                .absolute()
                .left(px(bounds.x))
                .top(px(bounds.y))
                .w(px(bounds.w))
                .h(px(bounds.h));
        }

        match self.content {
            MoonBadgeContent::Dot => badge,
            MoonBadgeContent::Text(text) => badge.child(
                div()
                    .text_color(rgba_from(style.fg, style.fg_alpha * disabled_scale))
                    .text_size(px(metrics.font_size))
                    .line_height(px(metrics.line_height))
                    .font_weight(FontWeight(self.weight))
                    .font_family(tokens.font_family(self.mono))
                    .child(text),
            ),
            MoonBadgeContent::Count { value, max } => {
                let text = max
                    .filter(|max| value > *max)
                    .map(|max| format!("{max}+"))
                    .unwrap_or_else(|| value.to_string());
                badge.child(
                    div()
                        .text_color(rgba_from(style.fg, style.fg_alpha * disabled_scale))
                        .text_size(px(metrics.font_size))
                        .line_height(px(metrics.line_height))
                        .font_weight(FontWeight(self.weight))
                        .font_family(tokens.font_family(self.mono))
                        .child(text),
                )
            }
            MoonBadgeContent::Icon(path) => badge.child(moon_icon(
                path,
                metrics.font_size + tokens.ui(2.0),
                style.fg,
                style.fg_alpha,
            )),
        }
    }

    /// Return rendered pixels; tiers use UI zoom only and custom text keeps font scaling.
    fn metrics(&self, tokens: &MoonThemeTokens) -> BadgeMetrics {
        match self.size.unwrap_or(MoonBadgeSize::Tier(tokens.tier())) {
            MoonBadgeSize::Tier(size) => {
                let tier = size.nearest(&[MoonSize::Xs, MoonSize::Sm, MoonSize::Md]);
                let m = tier.control_metrics();
                let font_size = match tier {
                    MoonSize::Xs => 11.0,
                    MoonSize::Sm => 12.0,
                    _ => 14.0,
                };
                // Preserve the old Tiny padding ratio at Xs and Status ratio above it.
                let ratio = if tier == MoonSize::Xs {
                    4.0 / 13.0
                } else {
                    7.0 / 17.0
                };
                BadgeMetrics {
                    height: tokens.ui(m.line_height),
                    radius: tokens.ui(m.radius),
                    font_size: tokens.ui(font_size),
                    line_height: tokens.ui(m.line_height),
                    pad_x: tokens.ui(m.line_height * ratio),
                    min_width: tokens.ui(m.line_height),
                }
            }
            MoonBadgeSize::Custom {
                height,
                radius,
                font_size,
                line_height,
                pad_x,
                min_width,
            } => BadgeMetrics {
                height,
                radius,
                font_size,
                line_height,
                pad_x,
                min_width,
            }
            .scaled(tokens),
        }
    }

    fn style(&self, p: MoonPalette) -> BadgeStyle {
        let tone = self.tone.color(p);
        let defaults = match self.variant {
            MoonBadgeVariant::Solid => BadgeStyle {
                bg: tone,
                bg_alpha: 0.8,
                border: tone,
                border_alpha: 0.0,
                fg: p.shell,
                fg_alpha: 1.0,
            },
            MoonBadgeVariant::Soft => BadgeStyle {
                bg: tone,
                bg_alpha: 0.14,
                border: tone,
                border_alpha: 0.0,
                fg: tone,
                fg_alpha: 1.0,
            },
            MoonBadgeVariant::Outline => BadgeStyle {
                bg: tone,
                bg_alpha: 0.08,
                border: tone,
                border_alpha: 0.27,
                fg: tone,
                fg_alpha: 1.0,
            },
        };

        BadgeStyle {
            bg: self.bg.unwrap_or(defaults.bg),
            bg_alpha: self.bg_alpha.unwrap_or(defaults.bg_alpha),
            border: self.border.unwrap_or(defaults.border),
            border_alpha: self.border_alpha.unwrap_or(defaults.border_alpha),
            fg: self.fg.unwrap_or(defaults.fg),
            fg_alpha: self.fg_alpha.unwrap_or(defaults.fg_alpha),
        }
    }
}

impl RenderOnce for MoonBadge {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = MoonTheme::active_tokens(cx);
        self.render_with_theme(MoonPalette::active(cx), tokens)
    }
}

#[cfg(test)]
mod tests;
