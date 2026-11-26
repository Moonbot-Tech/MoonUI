//! Regression tests for source-metric parsing of underscore-prefixed builder parameters and the
//! raw-hex scan of Moon runtime sources.

use std::fs;

use super::{
    function_body_uses_param, normalize_path, scan_raw_hex_in_moon, scan_raw_hex_in_moon_base,
    underscore_self_param,
};

/// Lines exercising every branch of the raw-hex rule: the four literal constructors are hits, a
/// whole-line `//` comment and a `MOON-ONEOFF` tag are not, a plain line is not, and a literal
/// before a trailing comment still is. Hits keep the trimmed line text.
const RAW_HEX_RULE_PROBE: &str = concat!(
    "    .bg(rgb(0x112233))\n",
    "    .bg(rgba(0x11223344))\n",
    "    .bg(Hsla::from(rgba_from(0x112233, 0.5)))\n",
    "    .bg(hsla(0x1))\n",
    "    // .bg(rgb(0x445566))\n",
    "    .bg(rgb(0x778899)) // MOON-ONEOFF: reviewed\n",
    "    .bg(cx.theme().background)\n",
    "    .bg(rgb(0xAABBCC)) // trailing note\n",
);

/// The `(line, text)` hits [`RAW_HEX_RULE_PROBE`] must produce under the raw-hex rule.
const RAW_HEX_RULE_PROBE_HITS: &[(usize, &str)] = &[
    (1, ".bg(rgb(0x112233))"),
    (2, ".bg(rgba(0x11223344))"),
    (3, ".bg(Hsla::from(rgba_from(0x112233, 0.5)))"),
    (4, ".bg(hsla(0x1))"),
    (8, ".bg(rgb(0xAABBCC)) // trailing note"),
];

/// Pins `source_metrics.rs:scan_raw_hex_in_moon`'s line rule on every literal form and exemption,
/// so the `raw_hex_in_moon_runtime` metric cannot silently stop counting a constructor or start
/// counting comments and `MOON-ONEOFF` lines.
#[test]
fn raw_hex_scan_in_moon_counts_every_literal_form_and_skips_exemptions() -> anyhow::Result<()> {
    let moon = tempfile::tempdir()?;
    fs::write(moon.path().join("widget.rs"), RAW_HEX_RULE_PROBE)?;

    let hits: Vec<(usize, String)> = scan_raw_hex_in_moon(moon.path())?
        .into_iter()
        .map(|hit| (hit.line, hit.text))
        .collect();

    let expected: Vec<(usize, String)> = RAW_HEX_RULE_PROBE_HITS
        .iter()
        .map(|(line, text)| (*line, text.to_string()))
        .collect();
    assert_eq!(hits, expected);
    Ok(())
}

/// Pins `source_metrics.rs:scan_raw_hex_in_moon_base`: the same line rule as the Moon scan, applied
/// to exactly the six audited base files in their listed order, with a missing file an error rather
/// than zero hits, so the `raw_hex_in_moon_base_runtime` metric cannot drift from the Moon one or
/// pass on an unreadable tree.
#[test]
fn raw_hex_scan_in_moon_base_applies_the_same_rule_to_the_listed_files() -> anyhow::Result<()> {
    let src = tempfile::tempdir()?;
    let files = [
        "checkbox.rs",
        "input/input.rs",
        "label.rs",
        "radio.rs",
        "slider.rs",
        "table/table.rs",
    ];
    fs::create_dir(src.path().join("input"))?;
    fs::create_dir(src.path().join("table"))?;
    for file in files {
        fs::write(src.path().join(file), "")?;
    }
    fs::write(src.path().join("radio.rs"), RAW_HEX_RULE_PROBE)?;
    fs::write(src.path().join("checkbox.rs"), "    .bg(rgb(0x010203))\n")?;
    fs::write(src.path().join("button.rs"), "    .bg(rgb(0x040506))\n")?;

    let prefix = format!("{}/", normalize_path(src.path()));
    let hits: Vec<(String, usize, String)> = scan_raw_hex_in_moon_base(src.path())?
        .into_iter()
        .map(|hit| {
            let file = hit
                .file
                .strip_prefix(&prefix)
                .unwrap_or(&hit.file)
                .to_string();
            (file, hit.line, hit.text)
        })
        .collect();

    let mut expected = vec![(
        "checkbox.rs".to_string(),
        1,
        ".bg(rgb(0x010203))".to_string(),
    )];
    expected.extend(
        RAW_HEX_RULE_PROBE_HITS
            .iter()
            .map(|(line, text)| ("radio.rs".to_string(), *line, text.to_string())),
    );
    assert_eq!(hits, expected);

    fs::remove_file(src.path().join("slider.rs"))?;
    assert!(scan_raw_hex_in_moon_base(src.path()).is_err());
    Ok(())
}

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

/// Catches matching a builder parameter as a substring, which hides a no-op builder whose body
/// mentions a longer identifier that starts with the parameter's name.
#[test]
fn function_body_usage_matches_whole_identifiers_only() {
    let unused = [
        "pub fn id(self, _id: usize) -> Self {",
        "    self.with_identity()",
        "}",
    ];
    assert!(!function_body_uses_param(&unused, 0, "_id"));
}

/// Catches reading a line comment that names the parameter as a use of it, which hides the very
/// no-op builder that documents why it ignores its argument from `noop_public_api_markers`. A
/// comment does not change the component, so the body below still returns `self` untouched.
#[test]
fn function_body_usage_ignores_line_comments_naming_the_param() {
    let unused = [
        "pub fn tone(self, _tone: MoonTone) -> Self {",
        "    // `_tone` is accepted for API parity and ignored for now.",
        "    self",
        "}",
    ];
    assert!(!function_body_uses_param(&unused, 0, "_tone"));
}

/// Catches counting the braces of a skipped line comment toward the body's depth: the `}` quoted
/// in the comment would close the body early, so the real use on the next line is never read and
/// a builder that does use its argument is reported as a no-op.
#[test]
fn function_body_usage_ignores_braces_in_line_comments() {
    let used = [
        "pub fn tone(self, _tone: MoonTone) -> Self {",
        "    // `_tone` replaces the old `}` branch.",
        "    Self { tone: _tone, ..self }",
        "}",
    ];
    assert!(function_body_uses_param(&used, 0, "_tone"));
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

/// Catches `source_metrics.rs:scan_raw_hex_in_moon` exempting any line that merely names
/// `MoonTone` or `MoonPalette`, which lets a component paint a raw hex colour inside a tone branch
/// without moving the `raw_hex_in_moon_runtime` metric. Only the token-source paths and a
/// `MOON-ONEOFF` tag approve a literal, so each of the two component lines below is one hit.
#[test]
fn raw_hex_scan_flags_a_literal_on_a_line_naming_a_tone_or_palette() -> anyhow::Result<()> {
    let moon = tempfile::tempdir()?;
    fs::write(
        moon.path().join("badge.rs"),
        concat!(
            "    .bg(if tone == MoonTone::Danger { rgb(0xFF0000) } else { p.shell })\n",
            "    let fill = if palette == MoonPalette::LIGHT { rgba(0xFFFFFF80) } else { base };\n",
            "    .border_color(rgb(0x00FF00)) // MOON-ONEOFF: reviewed brand outline\n",
        ),
    )?;

    let lines: Vec<usize> = scan_raw_hex_in_moon(moon.path())?
        .into_iter()
        .map(|hit| hit.line)
        .collect();

    assert_eq!(lines, [1, 2]);
    Ok(())
}
