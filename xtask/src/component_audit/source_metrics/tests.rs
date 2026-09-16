//! Regression tests for source-metric parsing of underscore-prefixed builder parameters and the
//! raw-hex scan of Moon runtime sources.

use std::fs;

use super::{
    function_body_uses_param, normalize_path, scan_raw_hex_in_moon, underscore_self_param,
};

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

/// Catches `source_metrics.rs:scan_raw_hex_in_moon` exempting token sources by file name instead
/// of by path, which would let any component module named `primitives.rs` paint raw hex without
/// moving the `raw_hex_in_moon_runtime` metric, or dropping the exemption, which fails the audit
/// on the primitive colour scales themselves.
#[test]
fn raw_hex_scan_skips_only_the_top_level_primitive_scales() -> anyhow::Result<()> {
    let moon = tempfile::tempdir()?;
    fs::create_dir(moon.path().join("dock"))?;
    fs::write(
        moon.path().join("primitives.rs"),
        "    c50: MoonColor::rgb(0xFAFAFA),\n",
    )?;
    fs::write(
        moon.path().join("dock/primitives.rs"),
        "    .bg(rgb(0x123456))\n",
    )?;
    fs::write(moon.path().join("button.rs"), "    .bg(rgba(0x12345678))\n")?;

    let prefix = format!("{}/", normalize_path(moon.path()));
    let mut flagged: Vec<String> = scan_raw_hex_in_moon(moon.path())?
        .into_iter()
        .map(|hit| {
            hit.file
                .strip_prefix(&prefix)
                .unwrap_or(&hit.file)
                .to_string()
        })
        .collect();
    flagged.sort();

    assert_eq!(flagged, ["button.rs", "dock/primitives.rs"]);
    Ok(())
}
