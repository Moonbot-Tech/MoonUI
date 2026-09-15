//! Radio choices with density-selected tiers and optional supporting text.

use gpui::prelude::FluentBuilder;
use gpui::*;

use super::{
    foundation::MoonSize,
    theme::{MoonTheme, MoonThemeTokens},
    tokens::{MoonRect, MoonTone, rgba_from},
};

/// Shared size tier or explicitly scaled legacy radio metrics.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MoonRadioSize {
    /// Supports Sm and Md; other tiers snap to the nearest supported size.
    Tier(MoonSize),
    Custom {
        dot_size: f32,
        font_size: f32,
        line_height: f32,
        gap: f32,
    },
}

impl From<MoonSize> for MoonRadioSize {
    /// Wraps a shared tier; unsupported tiers snap when metrics are resolved.
    fn from(size: MoonSize) -> Self {
        Self::Tier(size)
    }
}

/// Final pixel metrics, with legacy text scaling applied only to Custom.
#[derive(Clone, Copy, Debug)]
struct RadioMetrics {
    outer_size: f32,
    inner_size: f32,
    font_size: f32,
    line_height: f32,
    gap: f32,
    description_gap: f32,
}

fn moon_radio_click_value(disabled: bool) -> Option<bool> {
    if disabled { None } else { Some(true) }
}

/// A controlled radio choice whose omitted size follows the active density.
#[derive(IntoElement)]
pub struct MoonRadio {
    id: SharedString,
    bounds: Option<MoonRect>,
    label: Option<SharedString>,
    description: Option<SharedString>,
    checked: bool,
    disabled: bool,
    size: Option<MoonRadioSize>,
    tone: MoonTone,
    mono: bool,
    on_change: Option<std::rc::Rc<dyn Fn(&bool, &mut Window, &mut App)>>,
}

impl MoonRadio {
    /// Creates an unchecked choice with a density-selected size.
    pub fn new(id: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            bounds: None,
            label: None,
            description: None,
            checked: false,
            disabled: false,
            size: None,
            tone: MoonTone::Info,
            mono: true,
            on_change: None,
        }
    }

    pub fn bounds(mut self, bounds: MoonRect) -> Self {
        self.bounds = Some(bounds);
        self
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets supporting text below the label, keeping the mark centred on its first line.
    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Overrides the density with a tier (via Into) or custom metrics.
    pub fn size(mut self, size: MoonRadioSize) -> Self {
        self.size = Some(size);
        self
    }

    pub fn tone(mut self, tone: MoonTone) -> Self {
        self.tone = tone;
        self
    }

    pub fn mono(mut self, mono: bool) -> Self {
        self.mono = mono;
        self
    }

    pub fn on_change(mut self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(std::rc::Rc::new(handler));
        self
    }

    /// Resolves density, snapping and scaling into final pixels.
    fn metrics(&self, tokens: &MoonThemeTokens) -> RadioMetrics {
        match self.size.unwrap_or(MoonRadioSize::Tier(tokens.tier())) {
            MoonRadioSize::Tier(tier) => {
                let tier = tier.nearest(&[MoonSize::Sm, MoonSize::Md]);
                let control = tier.control_metrics();
                let outer_size: f32 = if tier == MoonSize::Sm { 16.0 } else { 20.0 };
                RadioMetrics {
                    outer_size: tokens.ui(outer_size),
                    inner_size: tokens.ui((outer_size * 0.44).round()),
                    font_size: tokens.ui(control.font_size),
                    line_height: tokens.ui(control.line_height),
                    gap: tokens.ui(control.gap),
                    description_gap: tokens.ui(if tier == MoonSize::Sm { 0.0 } else { 2.0 }),
                }
            }
            MoonRadioSize::Custom {
                dot_size,
                font_size,
                line_height,
                gap,
            } => RadioMetrics {
                outer_size: tokens.ui(dot_size),
                inner_size: tokens.ui((dot_size * 0.44).round()),
                font_size: tokens.font(font_size),
                line_height: tokens.line_height(line_height),
                gap: tokens.ui(gap),
                description_gap: 0.0,
            },
        }
    }
}

impl RenderOnce for MoonRadio {
    /// Renders one selectable row, stacking supporting text beneath the label.
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = MoonTheme::active_tokens(cx);
        let metrics = self.metrics(&tokens);
        let has_description = self.description.is_some();
        let p = tokens.palette;
        let accent = self.tone.color(p);
        let alpha = if self.disabled { 0.45 } else { 1.0 };
        let disabled = self.disabled;
        let checked = self.checked;
        let outer_size = metrics.outer_size;
        let inner_size = metrics.inner_size;
        let mut mark = div()
            .debug_selector(|| format!("{}:mark", self.id))
            .flex_shrink_0()
            .when(has_description, |this| {
                this.mt(px(((metrics.line_height - outer_size) * 0.5).max(0.0)))
            })
            .relative()
            .w(px(outer_size))
            .h(px(outer_size))
            .rounded(px(outer_size * 0.5))
            .border(px(tokens.ui(1.0)))
            .border_color(rgba_from(
                if checked { accent } else { p.border },
                0.82 * alpha,
            ))
            .bg(rgba_from(
                if checked { accent } else { p.shell_high },
                if checked { 0.14 } else { 0.95 } * alpha,
            ));

        if checked {
            mark = mark.child(
                div()
                    .absolute()
                    .left(px((outer_size - inner_size) * 0.5 - tokens.ui(1.0)))
                    .top(px((outer_size - inner_size) * 0.5 - tokens.ui(1.0)))
                    .w(px(inner_size))
                    .h(px(inner_size))
                    .rounded(px(inner_size * 0.5))
                    .bg(rgba_from(accent, alpha)),
            );
        }

        let mut root = div()
            .id(ElementId::from(SharedString::from(format!(
                "{}:root",
                self.id
            ))))
            .relative()
            .flex()
            .map(|this| {
                if has_description {
                    this.items_start()
                } else {
                    this.items_center()
                }
            })
            .gap(px(metrics.gap))
            .rounded(px(tokens.ui(4.0)))
            .cursor_default()
            .when(!disabled, |this| {
                this.hover(|this| this.bg(rgba_from(p.overlay, 0.025)))
                    .active(|this| this.bg(rgba_from(p.overlay, 0.015)))
            })
            .child(mark);

        if self.label.is_some() || has_description {
            let mut text = div()
                .flex()
                .flex_col()
                .gap(px(metrics.description_gap))
                .font_family(tokens.font_family(self.mono))
                .text_size(px(metrics.font_size))
                .line_height(px(metrics.line_height))
                .when(has_description, |this| {
                    this.mt(px(((outer_size - metrics.line_height) * 0.5).max(0.0)))
                });
            if let Some(label) = self.label {
                text = text.child(
                    div()
                        .debug_selector(|| format!("{}:label", self.id))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(rgba_from(
                            if disabled { p.text_muted } else { p.text_soft },
                            alpha,
                        ))
                        .child(label),
                );
            }
            if let Some(description) = self.description {
                text = text.child(
                    div()
                        .debug_selector(|| format!("{}:description", self.id))
                        .font_weight(FontWeight::NORMAL)
                        .text_color(rgba_from(p.text_muted, alpha))
                        .child(description),
                );
            }
            root = root.child(text);
        }

        if let Some(bounds) = self.bounds {
            root = root
                .absolute()
                .left(px(bounds.x))
                .top(px(bounds.y))
                .w(px(bounds.w))
                .h(px(bounds.h));
        }

        root = root.on_mouse_down(MouseButton::Left, move |_, window, cx| {
            cx.stop_propagation();
            if disabled {
                return;
            }
            window.prevent_default();
        });

        if let Some(on_change) = self.on_change {
            root = root.on_click(move |_, window, cx| {
                let Some(value) = moon_radio_click_value(disabled) else {
                    cx.stop_propagation();
                    return;
                };
                on_change(&value, window, cx);
            });
        }

        root
    }
}

#[cfg(test)]
mod tests;
