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
