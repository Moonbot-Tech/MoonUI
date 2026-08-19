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
    /// Caps Lock and lone-modifier presses arrive as modifier changes rather than key presses, so
    /// reading them takes state that outlives one event.
    watch: MoonHotkeyModifierWatch,
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
                watch: MoonHotkeyModifierWatch::default(),
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
        let modifier_state = state.clone();
        let modifier_on_change = self.on_change.clone();
        let controlled_modifiers = controlled;
        let recording_override = self.recording;
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
                            let modifiers = window.modifiers();
                            let capslock = window.capslock();
                            state.update(cx, |state, cx| {
                                state.recording = true;
                                // Adopt the keyboard's current state, or the first Caps Lock press
                                // of this session reads as the first observation and is swallowed.
                                state.watch.prime(modifiers, capslock);
                                cx.notify();
                            });
                        }
                    })
                    .on_key_down(move |event: &KeyDownEvent, window, cx| {
                        // A real key landing while a modifier is held makes that modifier a chord
                        // prefix, so releasing it afterwards must not record it on its own.
                        if !is_modifier_key_name(event.keystroke.key.as_str()) {
                            capture_state.update(cx, |state, _| state.watch.interrupt());
                        }
                        let recording = self
                            .recording
                            .unwrap_or_else(|| capture_state.read(cx).recording);
                        match moon_hotkey_capture(&event.keystroke, recording) {
                            MoonHotkeyCapture::Ignore => {}
                            MoonHotkeyCapture::StartRecording => {
                                cx.stop_propagation();
                                capture_focus.focus(window, cx);
                                let modifiers = window.modifiers();
                                let capslock = window.capslock();
                                capture_state.update(cx, |state, cx| {
                                    state.recording = true;
                                    state.watch.prime(modifiers, capslock);
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
                    .on_modifiers_changed(move |event: &ModifiersChangedEvent, window, cx| {
                        // The second half of recording: Caps Lock and the bare modifiers never
                        // arrive as key presses, so `on_key_down` alone cannot record them.
                        let recording =
                            recording_override.unwrap_or_else(|| modifier_state.read(cx).recording);
                        let capture = modifier_state.update(cx, |state, _| {
                            state.watch.modifiers_changed(
                                event.modifiers,
                                event.capslock,
                                recording,
                            )
                        });
                        if let MoonHotkeyCapture::Commit(stroke) = capture {
                            cx.stop_propagation();
                            modifier_state.update(cx, |state, cx| {
                                state.recording = false;
                                if !controlled_modifiers {
                                    state.value = Some(stroke.clone());
                                }
                                cx.notify();
                            });
                            if let Some(on_change) = &modifier_on_change {
                                on_change(Some(stroke), window, cx);
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

/// Cross-event state a recorder needs to read Caps Lock and lone-modifier presses.
///
/// Neither of those reaches an application as a key press. Windows turns `VK_CAPITAL` and the bare
/// modifier keys into `ModifiersChanged` instead of a keystroke, and macOS reports the same through
/// `NSFlagsChanged`, so a control that listens only to `on_key_down` can never record them however
/// permissive its key filter is. Reading them takes state, because a lone modifier is only a
/// binding of its own when it goes down and comes back up with nothing in between — while it is
/// held it may still turn out to be the prefix of a chord.
///
/// An application that dispatches such a binding at runtime drives the same type, so the key it
/// recorded and the key it later matches are decided by one implementation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MoonHotkeyModifierWatch {
    /// Modifier state as of the previous event; a tap can only start from nothing held.
    modifiers: Modifiers,
    /// Caps Lock as last observed, or `None` until an event first names it. The first observation
    /// is not a press: it is only the state the keyboard was already in.
    capslock: Option<bool>,
    /// The lone modifier held right now with nothing pressed since, and therefore still able to
    /// become a binding of its own when it is released.
    tap: Option<&'static str>,
}

impl MoonHotkeyModifierWatch {
    /// Adopt the state the window is already in, so the next event is read as a change.
    ///
    /// Without this the first Caps Lock press of a session is swallowed as the first observation.
    pub fn prime(&mut self, modifiers: Modifiers, capslock: Capslock) {
        self.modifiers = modifiers;
        self.capslock = Some(capslock.on);
        self.tap = None;
    }

    /// Give up on the lone-modifier candidate because something else happened while it was held.
    ///
    /// The modifier turned out to be a prefix — of a chord, or of a mouse gesture — and releasing
    /// it must not record it on its own.
    pub fn interrupt(&mut self) {
        self.tap = None;
    }

    /// Forget the keyboard state, so the next event is read as a snapshot and not as a press.
    ///
    /// A window that loses focus stops being told about key-ups, so the platform re-announces the
    /// whole modifier state when it comes back — on Windows by synthesizing a modifiers-changed
    /// event from `WM_ACTIVATE`. Without this, Caps Lock flipped in another application reads as a
    /// press here, and a modifier still held from the Alt+Tab that brought the window back arms a
    /// tap that fires the moment the user lets go.
    pub fn forget(&mut self) {
        *self = Self::default();
    }

    /// Fold one modifiers-changed event into the watch and report what it means.
    ///
    /// `active` is the recorder's recording flag; an application dispatching a live binding passes
    /// `true`. State is tracked either way, so the watch stays primed while it is `false`.
    pub fn modifiers_changed(
        &mut self,
        modifiers: Modifiers,
        capslock: Capslock,
        active: bool,
    ) -> MoonHotkeyCapture {
        let primed = self.capslock.is_some();
        let previous = std::mem::replace(&mut self.modifiers, modifiers);
        let capslock_changed = self.capslock.replace(capslock.on) == Some(!capslock.on);

        // An unprimed watch is only learning the state the keyboard is already in — it must not
        // read that as a Caps Lock press, nor arm a tap for a modifier that went down elsewhere.
        if !active || !primed {
            self.tap = None;
            return MoonHotkeyCapture::Ignore;
        }

        // Caps Lock has no press event to wait for: the flip of its state IS the press. It is
        // reported with whatever modifiers are held, so ctrl-capslock is recordable too.
        if capslock_changed {
            self.tap = None;
            return MoonHotkeyCapture::Commit(Keystroke {
                modifiers,
                key: "capslock".to_string(),
                key_char: None,
            });
        }

        if !modifiers.modified() {
            // Everything came back up. A modifier that was held alone the whole time is the
            // binding; anything else releases into nothing.
            return match self.tap.take() {
                Some(key) => MoonHotkeyCapture::Commit(Keystroke {
                    modifiers: Modifiers::none(),
                    key: key.to_string(),
                    key_char: None,
                }),
                None => MoonHotkeyCapture::Ignore,
            };
        }

        // Arm a tap only when the press started from nothing held. Coming down from a chord —
        // releasing Control while Alt stays down — leaves a lone modifier that the user pressed as
        // part of something else, and recording it would be recording an accident.
        self.tap = if previous.modified() {
            None
        } else {
            lone_modifier_key(modifiers)
        };
        MoonHotkeyCapture::WaitForKey
    }
}

/// The name GPUI parses a modifier-only shortcut back into, when exactly one modifier is held.
///
/// The names are `Keystroke::parse`'s own (`control` and `platform`, not `ctrl` and `cmd`), so a
/// recorded binding survives the round trip through a settings file.
fn lone_modifier_key(modifiers: Modifiers) -> Option<&'static str> {
    let held = [
        (modifiers.control, "control"),
        (modifiers.alt, "alt"),
        (modifiers.shift, "shift"),
        (modifiers.platform, "platform"),
        (modifiers.function, "function"),
    ];
    let mut only = None;
    for (down, name) in held {
        if down {
            if only.is_some() {
                return None;
            }
            only = Some(name);
        }
    }
    only
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
mod tests;
