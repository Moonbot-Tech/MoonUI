//! Regression coverage for MoonRating range and click behavior.

use super::{moon_rating_click_value, moon_rating_max, moon_rating_value};

/// Catches removing the lower bound from `rating.rs:moon_rating_max` or the upper clamp from
/// `moon_rating_value`, which would render an empty scale or more selected stars than the control
/// contains.
#[test]
fn rating_value_and_max_are_clamped() {
    assert_eq!(moon_rating_max(0), 1);
    assert_eq!(moon_rating_value(7, 5), 5);
    assert_eq!(moon_rating_value(0, 0), 0);
}

/// Catches letting `rating.rs:moon_rating_click_value` accept zero/disabled clicks or return a
/// value above the configured maximum, which would emit invalid or unavailable selections.
#[test]
fn rating_click_value_respects_disabled_and_range() {
    assert_eq!(moon_rating_click_value(3, 5, false), Some(3));
    assert_eq!(moon_rating_click_value(8, 5, false), Some(5));
    assert_eq!(moon_rating_click_value(0, 5, false), None);
    assert_eq!(moon_rating_click_value(3, 5, true), None);
}
