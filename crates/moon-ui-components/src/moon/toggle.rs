use gpui::prelude::FluentBuilder;
use gpui::*;

use super::{
    foundation::{MoonSize, box_shadow, v_flex},
    theme::{MoonTheme, MoonThemeTokens},
    tokens::{MoonPalette, MoonRect, MoonTone, rgba_from},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoonToggleLabelSide {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MoonToggleSize {
    /// A tier of the shared size scale. Toggles come in `Sm` (28x16 track) or `Md` (36x20);
    /// `Xs` renders as `Sm`, `Lg` and above as `Md`.
    Tier(MoonSize),
    Custom {
        track_width: f32,
        track_height: f32,
        thumb_size: f32,
        font_size: f32,
        line_height: f32,
        gap: f32,
    },
}

/// Former enum variants kept as associated consts so expression-position call sites still compile.
///
/// They cannot be used in a pattern (`match`, `if let`, `matches!`): `MoonToggleSize` cannot
/// derive `Eq` because `Custom` holds `f32` fields, and Rust requires `Eq` for a constant in
/// pattern position.
#[allow(non_upper_case_globals)]
impl MoonToggleSize {
    #[deprecated(note = "use `MoonSize::Sm`")]
    pub const Compact: Self = Self::Tier(MoonSize::Sm);
    #[deprecated(note = "use `MoonSize::Md`")]
    pub const Normal: Self = Self::Tier(MoonSize::Md);
}

impl From<MoonSize> for MoonToggleSize {
    fn from(size: MoonSize) -> Self {
        Self::Tier(size)
    }
}

/// Metrics for a `MoonToggle` size, in unscaled design-reference pixels until `resolve`.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct MoonToggleMetrics {
    pub track_width: f32,
    pub track_height: f32,
    pub thumb_size: f32,
    pub font_size: f32,
    pub line_height: f32,
    pub label_weight: f32,
    pub description_weight: f32,
    pub gap: f32,
    /// Space between the label and the description under it.
    pub description_gap: f32,
    /// Gap from the track edge to the focus ring's outer edge; the ring's stroke lies inside it.
    pub focus_ring_distance: f32,
    pub focus_ring_width: f32,
}

impl MoonToggleSize {
    pub const SUPPORTED_TIERS: [MoonSize; 2] = [MoonSize::Sm, MoonSize::Md];

    /// The app's density tier, snapped to a tier this component supports.
    pub fn density_default(tokens: &MoonThemeTokens) -> Self {
        Self::Tier(tokens.tier().nearest(&Self::SUPPORTED_TIERS))
    }

    /// This size's metrics in unscaled design-reference pixels.
    pub fn reference_metrics(self) -> MoonToggleMetrics {
        match self {
            Self::Tier(t) => {
                let t = t.nearest(&Self::SUPPORTED_TIERS);
                let c = t.control_metrics();
                let (track_width, track_height, thumb_size, description_gap) = match t {
                    MoonSize::Sm => (28.0, 16.0, 12.0, 0.0),
                    _ => (36.0, 20.0, 16.0, 2.0),
                };
                MoonToggleMetrics {
                    track_width,
                    track_height,
                    thumb_size,
                    font_size: c.font_size,
                    line_height: c.line_height,
                    label_weight: 500.0,
                    description_weight: 400.0,
                    gap: c.gap,
                    description_gap,
                    focus_ring_distance: 4.0,
                    focus_ring_width: 2.0,
                }
            }
            Self::Custom {
                track_width,
                track_height,
                thumb_size,
                font_size,
                line_height,
                gap,
            } => MoonToggleMetrics {
                track_width,
                track_height,
                thumb_size,
                font_size,
                line_height,
                label_weight: 400.0,
                description_weight: 400.0,
                gap,
                description_gap: 0.0,
                focus_ring_distance: 4.0,
                focus_ring_width: 2.0,
            },
        }
    }

    /// Metrics as rendered: a tier follows the UI zoom only, `Custom` also follows text scaling.
    fn resolve(self, tokens: &MoonThemeTokens) -> MoonToggleMetrics {
        match self {
            Self::Tier(_) => self.reference_metrics().zoomed(tokens),
            Self::Custom { .. } => self.reference_metrics().scaled(tokens),
        }
    }
}

impl MoonToggleMetrics {
    /// Scales every length by the UI zoom alone, text included.
    fn zoomed(self, tokens: &MoonThemeTokens) -> Self {
        let ui = |value: f32| tokens.ui(value);
        Self {
            track_width: ui(self.track_width),
            track_height: ui(self.track_height),
            thumb_size: ui(self.thumb_size),
            font_size: ui(self.font_size),
            line_height: ui(self.line_height),
            label_weight: self.label_weight,
            description_weight: self.description_weight,
            gap: ui(self.gap),
            description_gap: ui(self.description_gap),
            focus_ring_distance: ui(self.focus_ring_distance),
            focus_ring_width: ui(self.focus_ring_width),
        }
    }

    /// Scales geometry by the UI zoom and text by the theme's text scaling.
    ///
    /// Unlike the checkbox's `scaled`, this does not grow the track with `font_delta` — a track is
    /// not text-bound.
    fn scaled(self, tokens: &MoonThemeTokens) -> Self {
        Self {
            track_width: tokens.ui(self.track_width),
            track_height: tokens.ui(self.track_height),
            thumb_size: tokens.ui(self.thumb_size),
            font_size: tokens.font(self.font_size),
            line_height: tokens.line_height(self.line_height),
            label_weight: self.label_weight,
            description_weight: self.description_weight,
            gap: tokens.ui(self.gap),
            description_gap: tokens.ui(self.description_gap),
            focus_ring_distance: tokens.ui(self.focus_ring_distance),
            focus_ring_width: tokens.ui(self.focus_ring_width),
        }
    }
}

#[derive(Default)]
struct MoonToggleState {
    checked: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MoonToggleClickPlan {
    next_checked: bool,
    update_internal: bool,
}

fn moon_toggle_click_plan(
    checked: bool,
    controlled: bool,
    disabled: bool,
) -> Option<MoonToggleClickPlan> {
    if disabled {
        None
    } else {
        Some(MoonToggleClickPlan {
            next_checked: !checked,
            update_internal: !controlled,
        })
    }
}

#[derive(IntoElement)]
pub struct MoonToggle {
    id: SharedString,
    bounds: Option<MoonRect>,
    label: Option<SharedString>,
    description: Option<SharedString>,
    label_side: MoonToggleLabelSide,
    checked: Option<bool>,
    default_checked: bool,
    disabled: bool,
    size: Option<MoonToggleSize>,
    tone: MoonTone,
    mono: bool,
    label_color: Option<u32>,
    on_change: Option<std::rc::Rc<dyn Fn(&bool, &mut Window, &mut App)>>,
}

impl MoonToggle {
    pub fn new(id: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            bounds: None,
            label: None,
            description: None,
            label_side: MoonToggleLabelSide::Right,
            checked: None,
            default_checked: false,
            disabled: false,
            size: None,
            tone: MoonTone::Info,
            mono: true,
            label_color: None,
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

    /// Sets supporting text shown under the label in the muted text colour.
    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn label_side(mut self, side: MoonToggleLabelSide) -> Self {
        self.label_side = side;
        self
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = Some(checked);
        self
    }

    pub fn default_checked(mut self, checked: bool) -> Self {
        self.default_checked = checked;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Sets the size: a tier such as `MoonSize::Sm`, or a `MoonToggleSize::Custom`.
    pub fn size(mut self, size: impl Into<MoonToggleSize>) -> Self {
        self.size = Some(size.into());
        self
    }

    /// Overrides the label's ink. The description keeps the muted colour; disabled alpha still applies.
    pub fn label_color(mut self, color: u32) -> Self {
        self.label_color = Some(color);
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
}

impl RenderOnce for MoonToggle {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state_id = ElementId::from(self.id.clone());
        let state = window.use_keyed_state(state_id, cx, |_, _| MoonToggleState {
            checked: self.default_checked,
        });

        let tokens = MoonTheme::active_tokens(cx);
        let size = self
            .size
            .unwrap_or_else(|| MoonToggleSize::density_default(&tokens));
        let m = size.resolve(&tokens);
        let has_description = self.description.is_some();
        let track_offset = ((m.line_height - m.track_height) * 0.5).max(0.0);
        let text_offset = ((m.track_height - m.line_height) * 0.5).max(0.0);
        let inset = -(m.focus_ring_distance + tokens.ui(1.0));
        let p = tokens.palette;
        let checked = self.checked.unwrap_or_else(|| state.read(cx).checked);
        let disabled = self.disabled;
        let accent = self.tone.color(p);
        let control_alpha = if disabled { 0.45 } else { 1.0 };
        let parent_view = window.current_view();
        let track_width = m.track_width;
        let track_height = m.track_height;
        let thumb_size = m.thumb_size;
        let thumb_left = if checked {
            track_width - thumb_size - tokens.ui(2.0)
        } else {
            tokens.ui(2.0)
        };
        let colors = toggle_colors(p, accent, checked);
        let focus_handle = window
            .use_keyed_state(
                ElementId::from(SharedString::from(format!("{}:focus", self.id))),
                cx,
                |_, cx| cx.focus_handle().tab_stop(true),
            )
            .read(cx)
            .clone();
        let is_focused = focus_handle.is_focused(window);
        let label_ink =
            self.label_color
                .unwrap_or(if disabled { p.text_muted } else { p.text_soft });
        let text_alpha = if disabled { 0.45 } else { 1.0 };
        let label_selector = format!("{}:label", self.id);
        let description_selector = format!("{}:description", self.id);
        let track_selector = format!("{}:track", self.id);
        let focus_ring_selector = format!("{}:focus-ring", self.id);
        let has_text = self.label.is_some() || has_description;

        let switch = div()
            .relative()
            .debug_selector(|| track_selector)
            .flex_shrink_0()
            .when(has_description, |this| this.mt(px(track_offset)))
            .w(px(track_width))
            .h(px(track_height))
            .rounded(px(track_height * 0.5))
            .border(px(tokens.ui(1.0)))
            .border_color(rgba_from(colors.border, 0.72 * control_alpha))
            .bg(rgba_from(colors.track, colors.track_alpha * control_alpha))
            .child(
                div()
                    .absolute()
                    .left(px(thumb_left))
                    .top(px((track_height - thumb_size) * 0.5))
                    .w(px(thumb_size))
                    .h(px(thumb_size))
                    .rounded(px(thumb_size * 0.5))
                    .bg(rgba_from(colors.thumb, control_alpha))
                    .shadow(vec![box_shadow(
                        px(0.0),
                        px(tokens.ui(1.0)),
                        px(tokens.ui(4.0)),
                        px(0.0),
                        rgba_from(p.shadow, colors.shadow_alpha * control_alpha),
                    )]),
            )
            .when(is_focused, |this| {
                this.child(
                    div()
                        .debug_selector(|| focus_ring_selector)
                        .absolute()
                        .top(px(inset))
                        .left(px(inset))
                        .right(px(inset))
                        .bottom(px(inset))
                        .border(px(m.focus_ring_width))
                        .border_color(rgba_from(accent, 1.0))
                        .rounded(px(m.track_height * 0.5
                            + m.focus_ring_distance
                            + tokens.ui(1.0))),
                )
            });

        let column = v_flex()
            .gap(px(m.description_gap))
            .when(has_description, |this| this.mt(px(text_offset)))
            .when_some(self.label, |this, label| {
                this.child(
                    div()
                        .font_family(tokens.font_family(self.mono))
                        .text_size(px(m.font_size))
                        .line_height(px(m.line_height))
                        .font_weight(FontWeight(m.label_weight))
                        .text_color(rgba_from(label_ink, text_alpha))
                        .debug_selector(|| label_selector)
                        .child(label),
                )
            })
            .when_some(self.description, |this, description| {
                this.child(
                    div()
                        .font_family(tokens.font_family(self.mono))
                        .text_size(px(m.font_size))
                        .line_height(px(m.line_height))
                        .font_weight(FontWeight(m.description_weight))
                        .text_color(rgba_from(p.text_muted, text_alpha))
                        .debug_selector(|| description_selector)
                        .child(description),
                )
            });

        let mut root = div()
            .id(ElementId::from(SharedString::from(format!(
                "{}:root",
                self.id
            ))))
            .relative()
            .flex()
            .when(has_description, |this| this.items_start())
            .when(!has_description, |this| this.items_center())
            .gap(px(m.gap))
            .rounded(px(track_height * 0.5))
            .when(disabled, |this| this.cursor_default())
            .when(!disabled, |this| {
                this.track_focus(&focus_handle)
                    .cursor_pointer()
                    .hover(|this| this.bg(rgba_from(p.overlay, 0.025)))
                    .active(|this| this.bg(rgba_from(p.overlay, 0.015)))
            });

        if self.label_side == MoonToggleLabelSide::Left {
            if has_text {
                root = root.child(column);
            }
            root = root.child(switch);
        } else {
            root = root.child(switch);
            if has_text {
                root = root.child(column);
            }
        }

        if let Some(bounds) = self.bounds {
            root = root
                .absolute()
                .left(px(bounds.x))
                .top(px(bounds.y))
                .w(px(bounds.w))
                .h(px(bounds.h));
        }

        let focus_handle_for_click = focus_handle.clone();
        root = root.on_mouse_down(MouseButton::Left, move |_, window, cx| {
            cx.stop_propagation();
            if disabled {
                return;
            }
            focus_handle_for_click.focus(window, cx);
            window.prevent_default();
        });

        if !disabled {
            let controlled = self.checked.is_some();
            let on_change = self.on_change.clone();
            root = root.on_click(move |_, window, cx| {
                let Some(plan) = moon_toggle_click_plan(checked, controlled, disabled) else {
                    return;
                };
                if plan.update_internal {
                    state.update(cx, |state, _| {
                        state.checked = plan.next_checked;
                    });
                }
                if let Some(on_change) = on_change.as_ref() {
                    on_change(&plan.next_checked, window, cx);
                }
                cx.notify(parent_view);
            });
        }

        root
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ToggleColors {
    track: u32,
    track_alpha: f32,
    border: u32,
    thumb: u32,
    shadow_alpha: f32,
}

fn toggle_colors(p: MoonPalette, accent: u32, checked: bool) -> ToggleColors {
    if p.is_light() {
        if checked {
            ToggleColors {
                track: accent,
                track_alpha: 0.58,
                border: accent,
                thumb: p.surface,
                shadow_alpha: 0.14,
            }
        } else {
            ToggleColors {
                track: 0xEEF9FF,
                track_alpha: 1.0,
                border: 0xC5DEEC,
                thumb: 0x6AA6C8,
                shadow_alpha: 0.12,
            }
        }
    } else {
        ToggleColors {
            track: if checked { accent } else { p.panel },
            track_alpha: if checked { 0.55 } else { 1.0 },
            border: if checked { accent } else { p.border },
            thumb: if checked { p.text } else { p.text_soft },
            shadow_alpha: 0.38,
        }
    }
}

#[cfg(test)]
mod tests;
