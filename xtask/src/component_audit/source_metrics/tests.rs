//! Regression tests for source-metric parsing of underscore-prefixed builder parameters.

use super::{function_body_uses_param, underscore_self_param};

/// Catches losing underscore-prefixed parameter extraction, which would hide no-op builders.
#[test]
fn underscore_self_param_extracts_builder_arg() {
    assert_eq!(
        underscore_self_param("pub fn id(self, _id: impl Into<SharedString>) -> Self {"),
        Some("_id".to_string())
    );
}

/// Catches counting a used builder parameter as a no-op and reporting a false audit regression.
#[test]
fn function_body_usage_distinguishes_real_noop_from_used_arg() {
    let used = [
        "pub fn id(self, _id: impl Into<SharedString>) -> Self {",
        "    let mut this = self;",
        "    this.inner = this.inner.id(ElementId::from(_id.into()));",
        "    this",
        "}",
    ];
    assert!(function_body_uses_param(&used, 0, "_id"));

    let unused = [
        "pub fn id(self, _id: impl Into<SharedString>) -> Self {",
        "    self",
        "}",
    ];
    assert!(!function_body_uses_param(&unused, 0, "_id"));
}
