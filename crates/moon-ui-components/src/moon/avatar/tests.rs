//! Regression coverage for MoonAvatar initials.

use super::initials;

/// Catches removing `avatar.rs:initials` single-word fallback, which would render one-letter
/// initials for usernames while preserving the expected two-letter result for full names.
#[test]
fn avatar_initials_match_common_names() {
    assert_eq!(initials("Jason Lee"), "JL");
    assert_eq!(initials("huacnlee"), "HU");
}
