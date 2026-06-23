use std::rc::Rc;

use gpui::prelude::FluentBuilder as _;
use gpui::*;

use super::{
    kbd::{MoonKbd, MoonKbdSize},
    text::MoonText,
    theme::MoonTheme,
    tokens::{MoonPalette, MoonRect, MoonTone, rgba_from},
};

pub type MoonHotkeyChangeHandler = Rc<dyn Fn(Option<Keystroke>, &mut Window, &mut App)>;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MoonHotkeyInputSize {
    Compact,
    Normal,
    Custom {
        height: f32,
        radius: f32,
        font_size: f32,
        line_height: f32,
        pad_x: f32,
        gap: f32,
    },
}

#[derive(Clone, Copy, Debug)]
struct HotkeyMetrics {
    height: f32,
    radius: f32,
    font_size: f32,
    line_height: f32,
    pad_x: f32,
    gap: f32,
}

#[derive(Clone, Debug, Default)]
struct MoonHotkeyInputState {
    value: Option<Keystroke>,
    recording: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MoonHotkeyCapture {
    StartRecording,
    WaitForKey,
    Cancel,
    Clear,
    Commit(Keystroke),
    Ignore,
}

#[derive(IntoElement)]
pub struct MoonHotkeyInput {
    id: SharedString,
    bounds: Option<MoonRect>,
    width: Option<f32>,
    full_width: bool,
    value: Option<Option<Keystroke>>,
    default_value: Option<Keystroke>,
    recording: Option<bool>,
    placeholder: SharedString,
    recording_placeholder: SharedString,
    disabled: bool,
    invalid: bool,
    conflict: bool,
    conflict_label: Option<SharedString>,
    clearable: bool,
    size: MoonHotkeyInputSize,
    tone: MoonTone,
    mono: bool,
    on_change: Option<MoonHotkeyChangeHandler>,
}

impl MoonHotkeyInput {
    pub fn new(id: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            bounds: None,
            width: None,
            full_width: false,
            value: None,
            default_value: None,
            recording: None,
            placeholder: SharedString::from("Click to record"),
            recording_placeholder: SharedString::from("Press shortcut..."),
            disabled: false,
            invalid: false,
            conflict: false,
            conflict_label: None,
            clearable: true,
            size: MoonHotkeyInputSize::Normal,
            tone: MoonTone::Info,
            mono: true,
            on_change: None,
        }
    }

    pub fn bounds(mut self, bounds: MoonRect) -> Self {
        self.bounds = Some(bounds);
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    pub fn full_width(mut self) -> Self {
        self.full_width = true;
        self
    }

    pub fn value(mut self, value: impl Into<Option<Keystroke>>) -> Self {
        self.value = Some(value.into());
        self
    }

    pub fn empty_value(mut self) -> Self {
        self.value = Some(None);
        self
    }

    pub fn default_value(mut self, value: impl Into<Option<Keystroke>>) -> Self {
        self.default_value = value.into();
        self
    }

    pub fn recording(mut self, recording: bool) -> Self {
        self.recording = Some(recording);
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn recording_placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.recording_placeholder = placeholder.into();
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    pub fn conflict(mut self, conflict: bool) -> Self {
        self.conflict = conflict;
        self
    }

    pub fn conflict_label(mut self, label: impl Into<SharedString>) -> Self {
        self.conflict_label = Some(label.into());
        self.conflict = true;
        self
    }

    pub fn clearable(mut self, clearable: bool) -> Self {
        self.clearable = clearable;
        self
    }

    pub fn size(mut self, size: MoonHotkeyInputSize) -> Self {
        self.size = size;
        self
    }

    pub fn compact(self) -> Self {
        self.size(MoonHotkeyInputSize::Compact)
    }

    pub fn tone(mut self, tone: MoonTone) -> Self {
        self.tone = tone;
        self
    }

    pub fn mono(mut self, mono: bool) -> Self {
        self.mono = mono;
        self
    }

    pub fn on_change(
        mut self,
        handler: impl Fn(Option<Keystroke>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }

    fn metrics(&self) -> HotkeyMetrics {
        match self.size {
            MoonHotkeyInputSize::Compact => HotkeyMetrics {
                height: 24.0,
                radius: 4.0,
                font_size: 10.0,
                line_height: 13.0,
                pad_x: 7.0,
                gap: 6.0,
            },
            MoonHotkeyInputSize::Normal => HotkeyMetrics {
                height: 30.0,
                radius: 4.0,
                font_size: 10.5,
                line_height: 14.0,
                pad_x: 9.0,
                gap: 7.0,
            },
            MoonHotkeyInputSize::Custom {
                height,
                radius,
                font_size,
                line_height,
                pad_x,
                gap,
            } => HotkeyMetrics {
                height,
                radius,
                font_size,
                line_height,
                pad_x,
                gap,
            },
        }
    }
}

impl RenderOnce for MoonHotkeyInput {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = MoonTheme::active_tokens(cx);
        let p = tokens.palette;
        let metrics = self.metrics();
        let state_id = ElementId::from(self.id.clone());
        let focus_id = ElementId::from(SharedString::from(format!("{}:focus", self.id)));
        let default_value = self.default_value.clone();
        let state =
            window.use_keyed_state(state_id.clone(), cx, move |_, _| MoonHotkeyInputState {
                value: default_value,
                recording: false,
            });
        let focus_handle = window
            .use_keyed_state(focus_id, cx, |_, cx| cx.focus_handle().tab_stop(true))
            .read(cx)
            .clone();

        let controlled = self.value.is_some();
        let current_value = self
            .value
            .clone()
            .unwrap_or_else(|| state.read(cx).value.clone());
        let is_focused = focus_handle.is_focused(window);
        let recording = self
            .recording
            .unwrap_or_else(|| state.read(cx).recording && is_focused)
            && !self.disabled;
        let disabled = self.disabled;
        let invalid = self.invalid;
        let conflict = self.conflict;
        let clearable = self.clearable;
        let on_change = self.on_change.clone();
        let capture_state = state.clone();
        let capture_focus = focus_handle.clone();
        let active_tone = self.tone.color(p);

        let alpha = if disabled { 0.45 } else { 1.0 };
        let border_color =
            hotkey_border_color(p, active_tone, is_focused, recording, invalid, conflict);
        let bg_color = if disabled { p.panel } else { p.shell_high };
        let text = tokens.text(metrics.font_size, metrics.line_height);

        let mut root = div()
            .id(state_id)
            .relative()
            .h(px(tokens.ui(metrics.height)))
            .min_w(px(tokens.ui(176.0)))
            .px(px(tokens.ui(metrics.pad_x)))
            .gap(px(tokens.ui(metrics.gap)))
            .rounded(px(tokens.ui(metrics.radius)))
            .border(px(tokens.ui(1.0)))
            .border_color(rgba_from(border_color, if recording { 0.95 } else { 0.86 }))
            .bg(rgba_from(bg_color, if disabled { 0.52 } else { 1.0 }))
            .flex()
            .items_center()
            .overflow_hidden()
            .text_color(rgba_from(p.text_soft, alpha))
            .when(self.mono, |this| this.font_family(tokens.font_family(true)))
            .when(!disabled, |this| {
                this.track_focus(&focus_handle)
                    .cursor_pointer()
                    .hover(|this| this.border_color(rgba_from(p.border_hover, 1.0)))
                    .on_mouse_down(MouseButton::Left, {
                        let state = state.clone();
                        let focus_handle = focus_handle.clone();
                        move |_, window, cx| {
                            cx.stop_propagation();
                            focus_handle.focus(window, cx);
                            state.update(cx, |state, cx| {
                                state.recording = true;
                                cx.notify();
                            });
                        }
                    })
                    .on_key_down(move |event: &KeyDownEvent, window, cx| {
                        let recording = self
                            .recording
                            .unwrap_or_else(|| capture_state.read(cx).recording);
                        match moon_hotkey_capture(&event.keystroke, recording) {
                            MoonHotkeyCapture::Ignore => {}
                            MoonHotkeyCapture::StartRecording => {
                                cx.stop_propagation();
                                capture_focus.focus(window, cx);
                                capture_state.update(cx, |state, cx| {
                                    state.recording = true;
                                    cx.notify();
                                });
                            }
                            MoonHotkeyCapture::WaitForKey => {
                                cx.stop_propagation();
                            }
                            MoonHotkeyCapture::Cancel => {
                                cx.stop_propagation();
                                capture_state.update(cx, |state, cx| {
                                    state.recording = false;
                                    cx.notify();
                                });
                            }
                            MoonHotkeyCapture::Clear => {
                                cx.stop_propagation();
                                capture_state.update(cx, |state, cx| {
                                    state.recording = false;
                                    if !controlled {
                                        state.value = None;
                                    }
                                    cx.notify();
                                });
                                if let Some(on_change) = &on_change {
                                    on_change(None, window, cx);
                                }
                            }
                            MoonHotkeyCapture::Commit(stroke) => {
                                cx.stop_propagation();
                                capture_state.update(cx, |state, cx| {
                                    state.recording = false;
                                    if !controlled {
                                        state.value = Some(stroke.clone());
                                    }
                                    cx.notify();
                                });
                                if let Some(on_change) = &on_change {
                                    on_change(Some(stroke), window, cx);
                                }
                            }
                        }
                    })
            });

        if self.full_width {
            root = root.w_full();
        }
        if let Some(width) = self.width {
            root = root.w(px(tokens.ui(width)));
        }
        if let Some(bounds) = self.bounds {
            root = root
                .absolute()
                .left(px(bounds.x))
                .top(px(bounds.y))
                .w(px(bounds.w))
                .h(px(bounds.h));
        }

        let body = if recording {
            MoonText::new(self.recording_placeholder)
                .uppercase(false)
                .mono(self.mono)
                .font_size(text.font_size)
                .line_height(text.line_height)
                .color(active_tone)
                .render()
                .into_any_element()
        } else if let Some(stroke) = current_value.clone() {
            MoonKbd::from_keystroke(stroke)
                .size(if matches!(self.size, MoonHotkeyInputSize::Compact) {
                    MoonKbdSize::Compact
                } else {
                    MoonKbdSize::Normal
                })
                .into_any_element()
        } else {
            MoonText::new(self.placeholder)
                .uppercase(false)
                .mono(self.mono)
                .font_size(text.font_size)
                .line_height(text.line_height)
                .color(if disabled { p.text_muted } else { p.text_soft })
                .alpha(if disabled { 0.55 } else { 0.74 })
                .render()
                .into_any_element()
        };

        root.child(div().flex_1().overflow_hidden().child(body))
            .when(conflict || invalid, |this| {
                let label = self.conflict_label.clone().unwrap_or_else(|| {
                    SharedString::from(if invalid { "invalid" } else { "conflict" })
                });
                this.child(
                    MoonText::new(label)
                        .uppercase(false)
                        .mono(true)
                        .font_size(tokens.font(9.0))
                        .line_height(tokens.line_height(12.0))
                        .color(if invalid { p.red } else { p.amber })
                        .render(),
                )
            })
            .when(clearable && current_value.is_some() && !disabled, |this| {
                let state = state.clone();
                let on_change = self.on_change.clone();
                this.child(
                    div()
                        .w(px(tokens.ui(18.0)))
                        .h(px(tokens.ui(18.0)))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(tokens.ui(3.0)))
                        .cursor_pointer()
                        .hover(|this| this.bg(rgba_from(p.overlay, 0.045)))
                        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                            cx.stop_propagation();
                            state.update(cx, |state, cx| {
                                state.recording = false;
                                if !controlled {
                                    state.value = None;
                                }
                                cx.notify();
                            });
                            if let Some(on_change) = &on_change {
                                on_change(None, window, cx);
                            }
                        })
                        .child(
                            MoonText::new("×")
                                .uppercase(false)
                                .mono(true)
                                .font_size(tokens.font(10.0))
                                .line_height(tokens.line_height(12.0))
                                .weight(700.0)
                                .color(p.text_soft)
                                .render(),
                        ),
                )
            })
    }
}

fn hotkey_border_color(
    p: MoonPalette,
    active_tone: u32,
    focused: bool,
    recording: bool,
    invalid: bool,
    conflict: bool,
) -> u32 {
    if invalid {
        p.red
    } else if conflict {
        p.amber
    } else if recording || focused {
        active_tone
    } else {
        p.border
    }
}

pub fn moon_hotkey_capture(stroke: &Keystroke, recording: bool) -> MoonHotkeyCapture {
    let key = stroke.key.as_str();
    let plain = !stroke.modifiers.modified();
    if !recording {
        return match key {
            "enter" | "space" if plain => MoonHotkeyCapture::StartRecording,
            "backspace" | "delete" if plain => MoonHotkeyCapture::Clear,
            _ => MoonHotkeyCapture::Ignore,
        };
    }

    match key {
        "escape" if plain => MoonHotkeyCapture::Cancel,
        "backspace" | "delete" if plain => MoonHotkeyCapture::Clear,
        key if is_modifier_key_name(key) => MoonHotkeyCapture::WaitForKey,
        _ => MoonHotkeyCapture::Commit(stroke.clone()),
    }
}

fn is_modifier_key_name(key: &str) -> bool {
    matches!(
        key,
        "ctrl"
            | "control"
            | "alt"
            | "shift"
            | "cmd"
            | "super"
            | "win"
            | "platform"
            | "fn"
            | "function"
    )
}

#[cfg(test)]
mod tests {
    use gpui::Keystroke;

    use super::{MoonHotkeyCapture, moon_hotkey_capture};

    #[test]
    fn hotkey_input_does_not_steal_global_shortcuts_when_idle() {
        let stroke = Keystroke::parse("ctrl-k").unwrap();
        assert_eq!(
            moon_hotkey_capture(&stroke, false),
            MoonHotkeyCapture::Ignore
        );
    }

    #[test]
    fn hotkey_input_starts_recording_from_keyboard_activation() {
        let enter = Keystroke::parse("enter").unwrap();
        let space = Keystroke::parse("space").unwrap();
        assert_eq!(
            moon_hotkey_capture(&enter, false),
            MoonHotkeyCapture::StartRecording
        );
        assert_eq!(
            moon_hotkey_capture(&space, false),
            MoonHotkeyCapture::StartRecording
        );
    }

    #[test]
    fn hotkey_input_waits_for_non_modifier_key() {
        let control = Keystroke::parse("ctrl").unwrap();
        assert_eq!(
            moon_hotkey_capture(&control, true),
            MoonHotkeyCapture::WaitForKey
        );
    }

    #[test]
    fn hotkey_input_commits_full_chord_while_recording() {
        let stroke = Keystroke::parse("ctrl-alt-k").unwrap();
        assert_eq!(
            moon_hotkey_capture(&stroke, true),
            MoonHotkeyCapture::Commit(stroke)
        );
    }

    #[test]
    fn hotkey_input_escape_cancels_and_delete_clears() {
        let escape = Keystroke::parse("escape").unwrap();
        let delete = Keystroke::parse("delete").unwrap();
        let backspace = Keystroke::parse("backspace").unwrap();
        assert_eq!(
            moon_hotkey_capture(&escape, true),
            MoonHotkeyCapture::Cancel
        );
        assert_eq!(moon_hotkey_capture(&delete, true), MoonHotkeyCapture::Clear);
        assert_eq!(
            moon_hotkey_capture(&backspace, false),
            MoonHotkeyCapture::Clear
        );
    }

    #[test]
    fn hotkey_input_can_record_modified_control_keys() {
        for source in ["ctrl-delete", "ctrl-backspace", "ctrl-enter", "ctrl-space"] {
            let stroke = Keystroke::parse(source).unwrap();
            assert_eq!(
                moon_hotkey_capture(&stroke, true),
                MoonHotkeyCapture::Commit(stroke.clone()),
                "{source} must be recordable"
            );
            assert_eq!(
                moon_hotkey_capture(&stroke, false),
                MoonHotkeyCapture::Ignore,
                "{source} must not control the field while idle"
            );
        }
    }
}
