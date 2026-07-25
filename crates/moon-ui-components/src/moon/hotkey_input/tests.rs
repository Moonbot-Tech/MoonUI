//! Regression coverage for hotkey-capture state transitions.

use gpui::Keystroke;

use super::{MoonHotkeyCapture, moon_hotkey_capture};

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
