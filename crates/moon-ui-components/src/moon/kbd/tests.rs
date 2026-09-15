//! Regression coverage for MoonKbd geometry and platform shortcut formatting.

use super::{MoonKbd, MoonKbdSize};
use gpui::Keystroke;

/// Catches using pointer-target height or legacy font scaling, which enlarges shortcut chips.
#[test]
fn kbd_metrics_match_designer_reference() {
    use crate::moon::{MoonSize, MoonThemeTokens};
    let mut tokens = MoonThemeTokens::default();
    tokens.scale.ui = 1.5;
    tokens.scale.font = 3.0;
    tokens.scale.font_delta = 6.0;
    for (tier, height, font, radius, pad) in [
        (MoonSize::Xs, 24.0, 16.5, 4.0, 6.0),
        (MoonSize::Sm, 30.0, 18.0, 4.0, 8.0),
        (MoonSize::Md, 36.0, 21.0, 6.0, 12.0),
        (MoonSize::Lg, 36.0, 21.0, 6.0, 12.0),
        (MoonSize::Xl, 36.0, 21.0, 6.0, 12.0),
        (MoonSize::Xxl, 36.0, 21.0, 6.0, 12.0),
    ] {
        let metrics = MoonKbd::new("Esc").size(tier.into()).metrics(&tokens);
        assert_eq!(
            (metrics.height, metrics.line_height, metrics.font_size),
            (height, height, font)
        );
        assert_eq!((metrics.radius, metrics.pad_x), (radius, pad));
        tokens.scale.tier = tier;
        assert_eq!(MoonKbd::new("density").metrics(&tokens).height, height);
    }
}

/// Catches ignoring explicit tiers or legacy Custom text scaling, resizing pinned shortcut chips.
#[test]
fn kbd_tiers_and_custom_scaling_survive() {
    use crate::moon::{MoonSize, MoonThemeTokens};
    let mut tokens = MoonThemeTokens::default();
    tokens.scale.ui = 2.0;
    tokens.scale.font = 3.0;
    tokens.scale.font_delta = 6.0;
    assert_eq!(
        MoonKbd::new("Esc")
            .size(MoonKbdSize::Tier(MoonSize::Xs))
            .metrics(&tokens)
            .height,
        32.0
    );
    assert_eq!(
        MoonKbd::new("Esc")
            .size(MoonKbdSize::Tier(MoonSize::Sm))
            .metrics(&tokens)
            .height,
        40.0
    );
    let metrics = MoonKbd::new("Esc")
        .size(MoonKbdSize::Custom {
            height: 20.0,
            font_size: 10.0,
            line_height: 12.0,
            radius: 4.0,
            pad_x: 6.0,
        })
        .metrics(&tokens);
    assert_eq!(
        (metrics.height, metrics.font_size, metrics.line_height),
        (40.0, 36.0, 42.0)
    );
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

/// Catches dropping the modifier-only and Caps Lock labels from `kbd.rs:MoonKbd::format_keystroke`.
///
/// A shortcut that is one modifier parses back under GPUI's own name for it, so without these the
/// hotkey field shows "Control", "Platform" and "Capslock" — three labels no keyboard is printed
/// with, and the last one wrong for the key it names.
#[test]
fn kbd_labels_modifier_only_and_capslock_shortcuts() {
    #[cfg(target_os = "macos")]
    {
        assert_eq!(
            MoonKbd::format_keystroke(&Keystroke::parse("ctrl").unwrap()),
            "⌃"
        );
        assert_eq!(
            MoonKbd::format_keystroke(&Keystroke::parse("capslock").unwrap()),
            "⇪"
        );
    }
    #[cfg(not(target_os = "macos"))]
    {
        // `Keystroke::parse` reports a lone Control as the key "control", not as the modifier.
        assert_eq!(
            MoonKbd::format_keystroke(&Keystroke::parse("ctrl").unwrap()),
            "Ctrl"
        );
        assert_eq!(
            MoonKbd::format_keystroke(&Keystroke::parse("alt").unwrap()),
            "Alt"
        );
        assert_eq!(
            MoonKbd::format_keystroke(&Keystroke::parse("cmd").unwrap()),
            "Win"
        );
        assert_eq!(
            MoonKbd::format_keystroke(&Keystroke::parse("capslock").unwrap()),
            "Caps Lock"
        );
        assert_eq!(
            MoonKbd::format_keystroke(&Keystroke::parse("ctrl-capslock").unwrap()),
            "Ctrl+Caps Lock"
        );
    }
}
