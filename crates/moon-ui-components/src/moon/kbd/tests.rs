//! Regression coverage for MoonKbd geometry and platform shortcut formatting.

use super::{MoonKbd, MoonKbdSize};
use gpui::Keystroke;

/// Catches changing the compact or normal height in `kbd.rs:MoonKbd::metrics` away from the
/// reviewed designer reference, which would make shortcut tags misalign with neighboring controls.
#[test]
fn kbd_metrics_match_designer_reference() {
    let compact = MoonKbd::new("Esc").size(MoonKbdSize::Compact);
    assert_eq!(compact.metrics().height, 17.0);
    let normal = MoonKbd::new("Ctrl+K");
    assert_eq!(normal.metrics().height, 20.0);
}

/// Catches changing modifier ordering, separators, or special-key labels in
/// `kbd.rs:MoonKbd::format_keystroke`, which would make displayed shortcuts diverge from the
/// platform-specific Longbridge convention.
#[test]
fn kbd_formats_keystrokes_like_longbridge() {
    #[cfg(target_os = "macos")]
    {
        assert_eq!(
            MoonKbd::format_keystroke(&Keystroke::parse("cmd-enter").unwrap()),
            "⌘⏎"
        );
        assert_eq!(
            MoonKbd::format_keystroke(&Keystroke::parse("cmd-ctrl-shift-a").unwrap()),
            "⌃⇧⌘A"
        );
    }

    #[cfg(not(target_os = "macos"))]
    {
        assert_eq!(
            MoonKbd::format_keystroke(&Keystroke::parse("ctrl-a").unwrap()),
            "Ctrl+A"
        );
        assert_eq!(
            MoonKbd::format_keystroke(&Keystroke::parse("ctrl-alt-shift-a").unwrap()),
            "Ctrl+Alt+Shift+A"
        );
    }
}
