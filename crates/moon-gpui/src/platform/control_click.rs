//! Whether a macOS Control+left click is delivered as a right click.
//!
//! Upstream Zed rewrites that press unconditionally: an editor wants the platform's secondary-click
//! convention, and nothing in it binds Control+left itself. The rewrite is lossy — besides the
//! button it also CLEARS the Control flag and forces `click_count` to 1 — so an application that
//! does bind Control+left cannot recover the press downstream: it sees an ordinary right click and
//! runs whatever the right button does.
//!
//! MoonUI therefore passes the press through by default and lets an application opt into the editor
//! convention. The macOS window handler is the only reader; every other platform ignores this.

use std::sync::atomic::{AtomicBool, Ordering};

use crate::{Modifiers, MouseButton};

/// Off by default: a Control+left press reaches the application as Left carrying Control, the same
/// event Windows and Linux deliver.
static CONTROL_CLICK_AS_SECONDARY: AtomicBool = AtomicBool::new(false);

/// Opt into (or back out of) the macOS convention where Control+left click acts as a right click.
///
/// Turn it on for an application that relies on Control-click to open context menus and binds
/// nothing to Control+left itself; leave it off when Control+left is a gesture of the application's
/// own. Takes effect on the next press, so it may be flipped at any time; a no-op off macOS.
pub fn set_macos_control_click_as_secondary(enabled: bool) {
    CONTROL_CLICK_AS_SECONDARY.store(enabled, Ordering::Relaxed);
}

/// Whether Control+left click is currently delivered as a right click on macOS.
pub fn macos_control_click_as_secondary() -> bool {
    CONTROL_CLICK_AS_SECONDARY.load(Ordering::Relaxed)
}

/// The button and modifiers this press has to be delivered with, or `None` to leave it untouched.
///
/// Applies to a mouse-down and its paired mouse-up alike: rewriting only one of the two would leave
/// the button that went down never coming back up.
pub fn macos_control_click_rewrite(
    button: MouseButton,
    modifiers: Modifiers,
) -> Option<(MouseButton, Modifiers)> {
    if !macos_control_click_as_secondary() || button != MouseButton::Left || !modifiers.control {
        return None;
    }
    // Control is cleared because that is what the convention means: the application is told a right
    // click happened, not that Control was held during one.
    Some((
        MouseButton::Right,
        Modifiers {
            control: false,
            ..modifiers
        },
    ))
}

#[cfg(test)]
mod tests;
