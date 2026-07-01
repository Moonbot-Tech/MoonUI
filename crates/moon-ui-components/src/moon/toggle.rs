use gpui::prelude::FluentBuilder;
use gpui::*;

use super::{
    foundation::box_shadow,
    text::MoonText,
    theme::MoonTheme,
    tokens::{rgba_from, MoonPalette, MoonRect, MoonTone},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoonToggleLabelSide {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MoonToggleSize {
    Compact,
    Normal,
    Custom {
        track_width: f32,
        track_height: f32,
        thumb_size: f32,
        font_size: f32,
        line_height: f32,
        gap: f32,
    },
}

#[derive(Clone, Copy, Debug)]
struct ToggleMetrics {
    track_width: f32,
    track_height: f32,
    thumb_size: f32,
    font_size: f32,
    line_height: f32,
    gap: f32,
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
    label_side: MoonToggleLabelSide,
    checked: Option<bool>,
    default_checked: bool,
    disabled: bool,
    size: MoonToggleSize,
    tone: MoonTone,
    mono: bool,
    on_change: Option<std::rc::Rc<dyn Fn(&bool, &mut Window, &mut App)>>,
}

impl MoonToggle {
    pub fn new(id: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            bounds: None,
            label: None,
            label_side: MoonToggleLabelSide::Right,
            checked: None,
            default_checked: false,
            disabled: false,
            size: MoonToggleSize::Normal,
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

    pub fn size(mut self, size: MoonToggleSize) -> Self {
        self.size = size;
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

    fn metrics(&self) -> ToggleMetrics {
        match self.size {
            MoonToggleSize::Compact => ToggleMetrics {
                track_width: 28.0,
                track_height: 16.0,
                thumb_size: 12.0,
                font_size: 9.5,
                line_height: 12.0,
                gap: 7.0,
            },
            MoonToggleSize::Normal => ToggleMetrics {
                track_width: 36.0,
                track_height: 20.0,
                thumb_size: 16.0,
                font_size: 10.5,
                line_height: 13.0,
                gap: 8.0,
            },
            MoonToggleSize::Custom {
                track_width,
                track_height,
                thumb_size,
                font_size,
                line_height,
                gap,
            } => ToggleMetrics {
                track_width,
                track_height,
                thumb_size,
                font_size,
                line_height,
                gap,
            },
        }
    }
}

impl RenderOnce for MoonToggle {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state_id = ElementId::from(self.id.clone());
        let state = window.use_keyed_state(state_id, cx, |_, _| MoonToggleState {
            checked: self.default_checked,
        });

        let tokens = MoonTheme::active_tokens(cx);
        let p = tokens.palette;
        let checked = self.checked.unwrap_or_else(|| state.read(cx).checked);
        let disabled = self.disabled;
        let metrics = self.metrics();
        let accent = self.tone.color(p);
        let control_alpha = if disabled { 0.45 } else { 1.0 };
        let parent_view = window.current_view();
        let track_width = tokens.ui(metrics.track_width);
        let track_height = tokens.ui(metrics.track_height);
        let thumb_size = tokens.ui(metrics.thumb_size);
        let thumb_left = if checked {
            track_width - thumb_size - tokens.ui(2.0)
        } else {
            tokens.ui(2.0)
        };
        let colors = toggle_colors(p, accent, checked);

        let switch = div()
            .relative()
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
            );

        let label = self.label.as_ref().map(|label| {
            let text = tokens.text(metrics.font_size, metrics.line_height);
            MoonText::new(label.clone())
                .color(if disabled { p.text_muted } else { p.text_soft })
                .alpha(if disabled { 0.45 } else { 1.0 })
                .font_size(text.font_size)
                .line_height(text.line_height)
                .weight(400.0)
                .mono(self.mono)
                .uppercase(false)
                .render()
        });

        let mut root = div()
            .id(ElementId::from(SharedString::from(format!(
                "{}:root",
                self.id
            ))))
            .relative()
            .flex()
            .items_center()
            .gap(px(tokens.ui(metrics.gap)))
            .rounded(px(track_height * 0.5))
            .cursor_default()
            .when(!disabled, |this| {
                this.hover(|this| this.bg(rgba_from(p.overlay, 0.025)))
                    .active(|this| this.bg(rgba_from(p.overlay, 0.015)))
            });

        if self.label_side == MoonToggleLabelSide::Left {
            if let Some(label) = label {
                root = root.child(label);
            }
            root = root.child(switch);
        } else {
            root = root.child(switch);
            if let Some(label) = label {
                root = root.child(label);
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

        root = root.on_mouse_down(MouseButton::Left, move |_, window, cx| {
            cx.stop_propagation();
            if disabled {
                return;
            }
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
mod tests {
    use super::{moon_toggle_click_plan, toggle_colors, MoonToggle, MoonToggleSize};
    use crate::moon::MoonPalette;

    #[test]
    fn toggle_metrics_match_designer_reference() {
        let compact = MoonToggle::new("compact").size(MoonToggleSize::Compact);
        assert_eq!(compact.metrics().track_width, 28.0);
        assert_eq!(compact.metrics().track_height, 16.0);
        let normal = MoonToggle::new("normal");
        assert_eq!(normal.metrics().track_width, 36.0);
        assert_eq!(normal.metrics().track_height, 20.0);
    }

    #[test]
    fn toggle_click_plan_respects_disabled_and_controlled_state() {
        assert_eq!(moon_toggle_click_plan(false, false, true), None);

        let uncontrolled = moon_toggle_click_plan(false, false, false).unwrap();
        assert!(uncontrolled.next_checked);
        assert!(uncontrolled.update_internal);

        let controlled = moon_toggle_click_plan(true, true, false).unwrap();
        assert!(!controlled.next_checked);
        assert!(!controlled.update_internal);
    }

    #[test]
    fn light_toggle_uses_soft_knob_when_off() {
        let p = MoonPalette::LIGHT;
        let off = toggle_colors(p, p.accent, false);
        assert_eq!(off.track, 0xEEF9FF);
        assert_eq!(off.border, 0xC5DEEC);
        assert_eq!(off.thumb, 0x6AA6C8);
        assert_ne!(off.thumb, p.text);
        assert_ne!(off.thumb, p.text_soft);
        assert!(off.shadow_alpha < 0.20);

        let on = toggle_colors(p, p.accent, true);
        assert_eq!(on.thumb, p.surface);
        assert!(on.shadow_alpha < 0.20);
    }
}
