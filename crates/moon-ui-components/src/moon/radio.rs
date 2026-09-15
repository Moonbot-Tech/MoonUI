//! Radio choices with density-selected tiers and checkbox-shared visuals.

use gpui::prelude::FluentBuilder;
use gpui::*;

use crate::checkbox::{
    ChoiceColors, MoonCheckboxMetrics, choice_focus_ring, choice_text_column, fading_mark,
};

use super::{
    checkbox::tier_size,
    foundation::MoonSize,
    theme::{MoonTheme, MoonThemeTokens},
    tokens::{MoonRect, MoonTone},
};

/// Shared size tier or explicitly scaled custom radio metrics.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MoonRadioSize {
    /// A tier of the shared size scale. Radios come in the checkbox's tiers with the same text and
    /// spacing: `Sm` (16px circle) and `Md` (20px circle); `Xs` renders as `Sm`, and `Lg` and above
    /// render as `Md`.
    Tier(MoonSize),
    /// A custom radio whose outer circle is `dot_size` across.
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

/// A radio's geometry: the checkbox metrics of its size, with the circle as the box, plus the dot
/// drawn in a checked circle.
#[derive(Clone, Copy, Debug, PartialEq)]
struct RadioMetrics {
    choice: MoonCheckboxMetrics,
    dot_size: Pixels,
}

impl RadioMetrics {
    /// Resolves checkbox-shared tier geometry while retaining explicit custom radio scaling.
    fn resolve(size: MoonRadioSize, tokens: &MoonThemeTokens) -> Self {
        match size {
            MoonRadioSize::Tier(tier) => {
                let size = tier_size(tier);
                Self {
                    choice: MoonCheckboxMetrics::resolve(size, tokens),
                    // The 6 and 8px dots leave an even 5 and 6px ring inside the 16 and 20px
                    // circles.
                    dot_size: px(tokens.ui(if size == crate::Size::Small { 6. } else { 8. })),
                }
            }
            MoonRadioSize::Custom {
                dot_size,
                font_size,
                line_height,
                gap,
            } => Self {
                choice: MoonCheckboxMetrics {
                    box_size: px(tokens.ui(dot_size)),
                    font_size: px(tokens.font(font_size)),
                    line_height: px(tokens.line_height(line_height)),
                    gap: px(tokens.ui(gap)),
                    ..MoonCheckboxMetrics::resolve(crate::Size::Size(px(dot_size)), tokens)
                },
                dot_size: px(tokens.ui((dot_size * 0.44).round())),
            },
        }
    }
}

fn moon_radio_click_value(disabled: bool) -> Option<bool> {
    if disabled { None } else { Some(true) }
}

/// A controlled choice whose omitted size follows density; a handler enables keyboard focus.
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
            mono: false,
            on_change: None,
        }
    }

    pub fn bounds(mut self, bounds: MoonRect) -> Self {
        self.bounds = Some(bounds);
        self
    }

    /// Sets the label. The circle stays on the label's first line when the label wraps; an empty
    /// label renders no text, so the radio stays exactly its circle.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets supporting text shown under the label in the muted text colour.
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

    /// Sets the size: a tier such as `MoonSize::Sm`, or a `MoonRadioSize::Custom`.
    pub fn size(mut self, size: impl Into<MoonRadioSize>) -> Self {
        self.size = Some(size.into());
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

    /// Installs the selection handler and makes an enabled choice focusable.
    pub fn on_change(mut self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(std::rc::Rc::new(handler));
        self
    }
}

impl MoonRadio {
    /// Resolves omitted size from density and explicit tiers or custom metrics into pixels.
    fn metrics(&self, tokens: &MoonThemeTokens) -> RadioMetrics {
        RadioMetrics::resolve(
            self.size.unwrap_or(MoonRadioSize::Tier(tokens.tier())),
            tokens,
        )
    }
}

impl RenderOnce for MoonRadio {
    /// Renders the shared choice visuals and radio interaction at the resolved density.
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = MoonTheme::active_tokens(cx);
        let metrics = self.metrics(&tokens);
        let choice = metrics.choice;
        let disabled = self.disabled;
        let interactive = !disabled && self.on_change.is_some();
        let checked = self.checked;
        let colors = ChoiceColors::resolve(tokens.palette, self.tone, checked, disabled);
        let state_id = ElementId::from(self.id.clone());
        let focus_handle = window
            .use_keyed_state(state_id.clone(), cx, |_, cx| cx.focus_handle())
            .read(cx)
            .clone();
        let is_focused = focus_handle.is_focused(window);
        // Empty text counts as no text, so a bare radio never gains the text column and its gap.
        let label = self.label.filter(|label| !label.is_empty());
        let description = self
            .description
            .filter(|description| !description.is_empty());
        let has_text = label.is_some() || description.is_some();
        let dot = fading_mark(state_id, checked, window, cx, || {
            div()
                .debug_selector(|| format!("{}:dot", self.id))
                .size(metrics.dot_size)
                .rounded_full()
                .bg(colors.mark)
        });

        let circle = div()
            .debug_selector(|| format!("{}:box", self.id))
            .relative()
            .flex()
            .flex_shrink_0()
            .items_center()
            .justify_center()
            .when(has_text, |this| this.mt(choice.box_offset()))
            .size(choice.box_size)
            .rounded_full()
            .border_1()
            .border_color(colors.border)
            .bg(colors.fill)
            .when(is_focused, |this| {
                this.child(choice_focus_ring(
                    &self.id,
                    choice,
                    choice.box_size * 0.5,
                    colors.focus_ring,
                ))
            })
            .children(dot);

        let mut root = div()
            .id(ElementId::from(SharedString::from(format!(
                "{}:root",
                self.id
            ))))
            .when(interactive, |this| {
                this.track_focus(&focus_handle.clone().tab_stop(true))
            })
            .when(!disabled, |this| this.cursor_pointer())
            .relative()
            .flex()
            .items_center()
            // Rows with text are top-aligned so the circle stays on the first line.
            .when(has_text, |this| this.items_start())
            .gap(choice.gap)
            .text_size(choice.font_size)
            .text_color(colors.label)
            .when(self.mono, |this| this.font_family(tokens.font_family(true)))
            .child(circle)
            .when(has_text, |this| {
                this.child(choice_text_column(
                    &self.id,
                    choice,
                    label,
                    description,
                    colors.description,
                    Vec::new(),
                ))
            });

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
            if !interactive {
                return;
            }
            // Pressing an enabled radio with a handler focuses it, so Tab continues from the clicked
            // control. Focus is set here because this listener stops the press before the
            // element's own focus-on-press listener would see it.
            window.focus(&focus_handle, cx);
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
