//! Regression coverage for MoonCollapsible header interactions.

use super::moon_collapsible_header_click_next;

/// Catches removing the controlled/disabled guard or replacing the open-state inversion in
/// `collapsible.rs:moon_collapsible_header_click_next`, which would let read-only sections change
/// or make enabled sections toggle in only one direction.
#[test]
fn collapsible_header_click_respects_disabled_and_controlled_state() {
    assert_eq!(
        moon_collapsible_header_click_next(false, false, false),
        Some(true)
    );
    assert_eq!(
        moon_collapsible_header_click_next(true, false, false),
        Some(false)
    );
    assert_eq!(moon_collapsible_header_click_next(false, true, false), None);
    assert_eq!(moon_collapsible_header_click_next(false, false, true), None);
}

/// `collapsible.rs:MoonCollapsible::render` must build its header caret from the LIVE `open`
/// value, never a constant.
///
/// This is the bug this component's adoption fixed: the caret used to render unconditionally,
/// pointing the same way whether the section was open or closed. The plausible regression is a
/// refactor that passes a literal — `MoonDisclosure::glyph(true)` or `glyph(false)` — instead of
/// `open`, silently bringing that bug back: the caret then stops indicating anything.
///
/// No functional seam can catch this from outside `collapsible.rs`: `render()`'s `impl
/// IntoElement` return is opaque (the same reason `MoonDisclosure::caret_box` was split out of
/// its own `render()` for testability — see `disclosure.rs`), `MoonCollapsible` builds its header
/// inline with no equivalent split, and `Svg::transformation` — the field the caret's pose lives
/// in — has no public getter even on the concrete type. The header's own SOURCE is therefore the
/// only observable surface. Read as text with comment lines stripped first, so a comment that
/// happens to name the call cannot satisfy this the way a raw substring search could.
///
/// The scan is bounded to the header's own click-handler-and-caret chain, not the whole file: a
/// whole-file search would stay green if the header switched to a literal while the string
/// `MoonDisclosure::glyph(open)` still appeared anywhere else (a second header variant, a future
/// helper) — and it would never ban the two literal poses the regression actually writes, only
/// require the correct string to appear SOMEWHERE. Anchoring on the click handler through the
/// header/content split, and asserting both the positive form and the two literal forms it must
/// exclude, closes both gaps.
#[test]
fn the_header_caret_is_wired_to_the_live_open_value() {
    let source = include_str!("../collapsible.rs");
    let code: String = source
        .lines()
        .map(|line| match line.find("//") {
            Some(at) => &line[..at],
            None => line,
        })
        .collect::<Vec<_>>()
        .join("\n");

    let start = code
        .find(".on_mouse_down(MouseButton::Left, {")
        .expect("expected to find the header's click handler in MoonCollapsible::render");
    let after_handler = &code[start..];
    let end = after_handler
        .find("if self.header.is_empty()")
        .expect("expected the click handler to be followed by the header/content split");
    let chain = &after_handler[..end];

    assert!(
        chain.contains("MoonDisclosure::glyph(open)"),
        "MoonCollapsible's header caret must be built from the live `open` value, not a constant"
    );
    assert!(
        !chain.contains("MoonDisclosure::glyph(true)")
            && !chain.contains("MoonDisclosure::glyph(false)"),
        "the header caret must never be built from a literal pose"
    );
}
