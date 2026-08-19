//! Regression coverage for hotkey-capture state transitions.

use gpui::{Capslock, Keystroke, Modifiers};

use super::{MoonHotkeyCapture, MoonHotkeyModifierWatch, moon_hotkey_capture};

/// Catches handling ordinary chords while idle in `hotkey_input.rs:moon_hotkey_capture`, which
/// would steal global shortcuts before the user activates the field.
#[test]
fn hotkey_input_does_not_steal_global_shortcuts_when_idle() {
    let stroke = Keystroke::parse("ctrl-k").unwrap();
    assert_eq!(
        moon_hotkey_capture(&stroke, false),
        MoonHotkeyCapture::Ignore
    );
}

/// Catches removing Enter or Space activation in `hotkey_input.rs:moon_hotkey_capture`, which
/// would make the field inaccessible to keyboard-only users.
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

/// Catches committing a bare modifier in `hotkey_input.rs:moon_hotkey_capture`, which would store
/// an unusable shortcut before the user presses its non-modifier key.
#[test]
fn hotkey_input_waits_for_non_modifier_key() {
    let control = Keystroke::parse("ctrl").unwrap();
    assert_eq!(
        moon_hotkey_capture(&control, true),
        MoonHotkeyCapture::WaitForKey
    );
}

/// Catches dropping modifiers from the commit branch in `hotkey_input.rs:moon_hotkey_capture`,
/// which would save a different shortcut than the user entered.
#[test]
fn hotkey_input_commits_full_chord_while_recording() {
    let stroke = Keystroke::parse("ctrl-alt-k").unwrap();
    assert_eq!(
        moon_hotkey_capture(&stroke, true),
        MoonHotkeyCapture::Commit(stroke)
    );
}

/// Catches merging cancel and clear handling in `hotkey_input.rs:moon_hotkey_capture`, which
/// would either erase a shortcut on Escape or leave Delete unable to clear it.
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

/// Catches classifying modified control keys as field controls in
/// `hotkey_input.rs:moon_hotkey_capture`, which would prevent users from recording those chords.
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

/// Catches dropping the modifiers-changed half of recording in `hotkey_input.rs`.
///
/// Neither Windows nor macOS reports a bare modifier as a key press, so without this a user cannot
/// put an action on Alt at all: the field waits forever and records nothing.
#[test]
fn hotkey_input_records_a_lone_modifier_on_release() {
    let mut watch = MoonHotkeyModifierWatch::default();
    watch.prime(Modifiers::none(), Capslock { on: false });

    assert_eq!(
        watch.modifiers_changed(Modifiers::alt(), Capslock { on: false }, true),
        MoonHotkeyCapture::WaitForKey,
        "a held modifier may still turn out to be a chord prefix"
    );
    assert_eq!(
        watch.modifiers_changed(Modifiers::none(), Capslock { on: false }, true),
        MoonHotkeyCapture::Commit(Keystroke::parse("alt").unwrap()),
        "releasing it with nothing pressed in between is the binding"
    );
}

/// Catches recording a modifier that was only leading a chord, in `hotkey_input.rs`.
///
/// Both paths would otherwise put "alt" into the field the moment the user let go of Alt+K, or of
/// Ctrl+Alt, silently replacing the shortcut they actually pressed.
#[test]
fn hotkey_input_ignores_a_modifier_that_led_a_chord() {
    let mut watch = MoonHotkeyModifierWatch::default();
    watch.prime(Modifiers::none(), Capslock { on: false });
    watch.modifiers_changed(Modifiers::alt(), Capslock { on: false }, true);
    watch.interrupt();
    assert_eq!(
        watch.modifiers_changed(Modifiers::none(), Capslock { on: false }, true),
        MoonHotkeyCapture::Ignore,
        "a key landed while Alt was held, so Alt is not a binding of its own"
    );

    let mut watch = MoonHotkeyModifierWatch::default();
    watch.prime(Modifiers::none(), Capslock { on: false });
    watch.modifiers_changed(Modifiers::control(), Capslock { on: false }, true);
    // Ctrl+Alt held, then Ctrl released: Alt is alone again, but the user pressed a chord.
    watch.modifiers_changed(
        Modifiers {
            control: true,
            alt: true,
            ..Modifiers::none()
        },
        Capslock { on: false },
        true,
    );
    watch.modifiers_changed(Modifiers::alt(), Capslock { on: false }, true);
    assert_eq!(
        watch.modifiers_changed(Modifiers::none(), Capslock { on: false }, true),
        MoonHotkeyCapture::Ignore
    );
}

/// Catches treating the first observed Caps Lock state as a press in `hotkey_input.rs`, and
/// dropping the flip that IS one.
///
/// The keyboard is simply in some Caps Lock state when a field opens; reading that as a press
/// would record Caps Lock without the user touching it, and ignoring the flip would leave the key
/// unrecordable.
#[test]
fn hotkey_input_records_capslock_from_its_state_flip() {
    let mut watch = MoonHotkeyModifierWatch::default();
    assert_eq!(
        watch.modifiers_changed(Modifiers::none(), Capslock { on: true }, true),
        MoonHotkeyCapture::Ignore,
        "the first event only names the state the keyboard was already in"
    );
    assert_eq!(
        watch.modifiers_changed(Modifiers::none(), Capslock { on: false }, true),
        MoonHotkeyCapture::Commit(Keystroke::parse("capslock").unwrap()),
        "turning it back off is a press, exactly like turning it on"
    );
    assert_eq!(
        watch.modifiers_changed(Modifiers::control(), Capslock { on: true }, true),
        MoonHotkeyCapture::Commit(Keystroke::parse("ctrl-capslock").unwrap()),
        "held modifiers belong to the recorded shortcut"
    );
}

/// Catches committing from the watch while the field is not recording, in `hotkey_input.rs`.
///
/// Every modifier press in the application reaches a focused field; acting on them outside a
/// recording session would rewrite a shortcut the user never opened.
#[test]
fn hotkey_input_modifier_watch_is_silent_while_idle() {
    let mut watch = MoonHotkeyModifierWatch::default();
    watch.prime(Modifiers::none(), Capslock { on: false });
    assert_eq!(
        watch.modifiers_changed(Modifiers::alt(), Capslock { on: false }, false),
        MoonHotkeyCapture::Ignore
    );
    assert_eq!(
        watch.modifiers_changed(Modifiers::none(), Capslock { on: true }, false),
        MoonHotkeyCapture::Ignore,
        "not even a Caps Lock flip records while idle"
    );
}

/// Catches reading the state a window is re-told on activation as a press.
///
/// Windows re-announces the whole modifier state when a window comes back (`WM_ACTIVATE`
/// synthesizes a modifiers-changed event), because a window without focus is not told about
/// key-ups. Without [`MoonHotkeyModifierWatch::forget`] and the unprimed guard, Caps Lock flipped
/// in another application fires the binding the moment the user clicks back in, and Alt still held
/// from the Alt+Tab that brought them there fires on release — a keypress nobody made.
#[test]
fn a_state_snapshot_after_losing_focus_is_not_a_press() {
    let mut watch = MoonHotkeyModifierWatch::default();
    watch.prime(Modifiers::none(), Capslock { on: false });
    watch.forget();

    // Coming back with Caps Lock flipped somewhere else, and with Alt still held from Alt+Tab.
    assert_eq!(
        watch.modifiers_changed(Modifiers::alt(), Capslock { on: true }, true),
        MoonHotkeyCapture::Ignore,
        "the re-announcement is the state, not a press"
    );
    assert_eq!(
        watch.modifiers_changed(Modifiers::none(), Capslock { on: true }, true),
        MoonHotkeyCapture::Ignore,
        "releasing a modifier that was never pressed here records nothing"
    );
    // The watch is primed again, so an ordinary press right after still reads as one.
    assert_eq!(
        watch.modifiers_changed(Modifiers::none(), Capslock { on: false }, true),
        MoonHotkeyCapture::Commit(Keystroke::parse("capslock").unwrap())
    );
}
