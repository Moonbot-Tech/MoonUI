//! Regression coverage for MoonRadio behavior and reviewed geometry.

use super::{MoonRadio, MoonRadioSize, moon_radio_click_value};

/// Catches changing the compact or normal `outer_size`/`inner_size` values in
/// `radio.rs:MoonRadio::metrics` away from the reviewed designer reference, which would render the
/// radio mark at the wrong size.
#[test]
fn radio_metrics_match_designer_reference() {
    let compact = MoonRadio::new("compact").size(MoonRadioSize::Compact);
    assert_eq!(compact.metrics().outer_size, 12.0);
    assert_eq!(compact.metrics().inner_size, 5.0);
    let normal = MoonRadio::new("normal");
    assert_eq!(normal.metrics().outer_size, 14.0);
    assert_eq!(normal.metrics().inner_size, 6.0);
}

/// Catches making `radio.rs:moon_radio_click_value` select disabled radios or ignore enabled
/// clicks, which would let unavailable choices change or leave available choices unselected.
#[test]
fn radio_click_value_respects_disabled_state() {
    assert_eq!(moon_radio_click_value(false), Some(true));
    assert_eq!(moon_radio_click_value(true), None);
}
