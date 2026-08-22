//! Source-hygiene metric collection and Rust source scanning helpers.

use std::path::Path;

use anyhow::Result;
use walkdir::WalkDir;

use super::{MetricPolicy, SourceHit, SourceMetric, normalize_path, read};

#[cfg(test)]
mod tests;

/// Collects the sorted source-hygiene metrics for the audited workspace.
pub(super) fn source_metrics(root: &Path) -> Result<Vec<SourceMetric>> {
    let src = root.join("crates/moon-ui-components/src");
    let moon_src = src.join("moon");
    let facade = root.join("crates/moon-ui/src/lib.rs");

    let mut metrics = vec![
        metric(
            "moon_skin_palette_usages",
            MetricPolicy::MustNotIncrease,
            scan_contains(&src, "MoonSkinPalette")?,
        ),
        metric(
            "moon_color_usages",
            MetricPolicy::MustNotIncrease,
            scan_contains(&src, "moon_color(")?,
        ),
        metric(
            "facade_public_component_slurp",
            MetricPolicy::MustNotIncrease,
            scan_file_contains(&facade, "pub use gpui_component::*")?,
        ),
        metric(
            "facade_components_escape_hatch",
            MetricPolicy::MustNotIncrease,
            scan_file_contains(&facade, "pub mod components")?,
        ),
        metric(
            "facade_raw_gpui_exports",
            MetricPolicy::MustNotIncrease,
            scan_facade_raw_gpui_exports(&facade)?,
        ),
        metric(
            "raw_hex_in_moon_runtime",
            MetricPolicy::MustNotIncrease,
            scan_raw_hex_in_moon(&moon_src)?,
        ),
        metric(
            "raw_hex_in_moon_base_runtime",
            MetricPolicy::MustNotIncrease,
            scan_raw_hex_in_moon_base(&src)?,
        ),
        metric(
            "noop_public_api_markers",
            MetricPolicy::MustNotIncrease,
            scan_noop_markers(&src)?,
        ),
    ];
    metrics.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(metrics)
}

/// Construct one source metric from its policy and sorted hits.
fn metric(id: &str, policy: MetricPolicy, hits: Vec<SourceHit>) -> SourceMetric {
    SourceMetric {
        id: id.to_string(),
        count: hits.len(),
        policy,
        hits,
    }
}

/// Scan Rust sources below a root for lines containing one literal needle.
fn scan_contains(root: &Path, needle: &str) -> Result<Vec<SourceHit>> {
    scan_files(root, |line| line.contains(needle))
}

/// Scan one UTF-8 source file for lines containing one literal needle.
fn scan_file_contains(path: &Path, needle: &str) -> Result<Vec<SourceHit>> {
    let text = read(path)?;
    Ok(text
        .lines()
        .enumerate()
        .filter(|(_, line)| line.contains(needle))
        .map(|(ix, line)| SourceHit {
            file: normalize_path(path),
            line: ix + 1,
            text: line.trim().to_string(),
        })
        .collect())
}

/// Find facade re-exports that expose raw base-component APIs.
fn scan_facade_raw_gpui_exports(path: &Path) -> Result<Vec<SourceHit>> {
    let text = read(path)?;
    Ok(text
        .lines()
        .enumerate()
        .filter_map(|(ix, line)| {
            let trimmed = line.trim();
            if !trimmed.starts_with("pub use gpui_component::") {
                return None;
            }
            let allowed = trimmed == "pub use gpui_component::moon::*;"
                || trimmed == "pub use gpui_component::moon::foundation::*;";
            (!allowed).then(|| SourceHit {
                file: normalize_path(path),
                line: ix + 1,
                text: trimmed.to_string(),
            })
        })
        .collect())
}

/// Find unapproved raw color literals in Moon-owned component sources.
fn scan_raw_hex_in_moon(root: &Path) -> Result<Vec<SourceHit>> {
    scan_files(root, |line| {
        let trimmed = line.trim();
        if trimmed.starts_with("//") {
            return false;
        }
        let is_token_file = trimmed.contains("MoonPalette") || trimmed.contains("MoonTone");
        !is_token_file
            && (trimmed.contains("rgb(0x")
                || trimmed.contains("rgba(0x")
                || trimmed.contains("rgba_from(0x")
                || trimmed.contains("hsla(0x"))
            && !trimmed.contains("MOON-ONEOFF")
    })
}

/// Find unapproved raw color literals in the audited inherited base components.
fn scan_raw_hex_in_moon_base(src: &Path) -> Result<Vec<SourceHit>> {
    let files = [
        "checkbox.rs",
        "input/input.rs",
        "label.rs",
        "radio.rs",
        "slider.rs",
        "table/table.rs",
    ];
    let mut hits = Vec::new();
    for file in files {
        let path = src.join(file);
        let text = read(&path)?;
        hits.extend(text.lines().enumerate().filter_map(|(ix, line)| {
            let trimmed = line.trim();
            if trimmed.starts_with("//") || trimmed.contains("MOON-ONEOFF") {
                return None;
            }
            (trimmed.contains("rgb(0x")
                || trimmed.contains("rgba(0x")
                || trimmed.contains("rgba_from(0x")
                || trimmed.contains("hsla(0x"))
            .then(|| SourceHit {
                file: normalize_path(&path),
                line: ix + 1,
                text: trimmed.to_string(),
            })
        }));
    }
    Ok(hits)
}

/// Find builder methods that accept parameters but leave the component unchanged.
fn scan_noop_markers(root: &Path) -> Result<Vec<SourceHit>> {
    let mut hits = scan_files(root, |line| {
        let trimmed = line.trim();
        trimmed.contains("let _ = self.")
            || trimmed.contains("let _tone = self.")
            || trimmed.contains("let _variant = self.")
            || trimmed.contains("let _input_id = self.")
    })?;

    for entry in WalkDir::new(root).into_iter().filter_entry(|entry| {
        let name = entry.file_name().to_string_lossy();
        name != "target" && name != ".git"
    }) {
        let entry = entry?;
        if !entry.file_type().is_file()
            || entry.path().extension().and_then(|e| e.to_str()) != Some("rs")
        {
            continue;
        }

        let text = read(entry.path())?;
        let lines = text.lines().collect::<Vec<_>>();
        for (ix, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if !trimmed.starts_with("pub fn ")
                || !trimmed.contains("(self, _")
                || !trimmed.ends_with("-> Self {")
            {
                continue;
            }

            let Some(param) = underscore_self_param(trimmed) else {
                continue;
            };
            if !function_body_uses_param(&lines, ix, &param) {
                hits.push(SourceHit {
                    file: normalize_path(entry.path()),
                    line: ix + 1,
                    text: trimmed.to_string(),
                });
            }
        }
    }

    Ok(hits)
}

/// Extract the underscore-prefixed parameter that follows a builder's `self` argument.
fn underscore_self_param(signature: &str) -> Option<String> {
    let rest = signature.split_once("(self, ")?.1;
    let end = rest
        .find(|ch: char| ch == ':' || ch == ',' || ch == ')')
        .unwrap_or(rest.len());
    let param = rest[..end].trim();
    param.starts_with('_').then(|| param.to_string())
}

/// Return whether a function body refers to the supplied parameter before its closing brace.
fn function_body_uses_param(lines: &[&str], signature_ix: usize, param: &str) -> bool {
    let mut depth = brace_delta(lines[signature_ix]);
    let after_open = lines[signature_ix].split_once('{').map(|(_, body)| body);
    if after_open.is_some_and(|body| body.contains(param)) {
        return true;
    }

    for line in lines.iter().skip(signature_ix + 1) {
        if line.contains(param) {
            return true;
        }
        depth += brace_delta(line);
        if depth <= 0 {
            return false;
        }
    }

    false
}

/// Return the net curly-brace depth change contributed by one source line.
fn brace_delta(line: &str) -> i32 {
    line.chars().fold(0, |depth, ch| match ch {
        '{' => depth + 1,
        '}' => depth - 1,
        _ => depth,
    })
}

/// Scan sorted Rust files below a root and retain lines accepted by a predicate.
fn scan_files(root: &Path, predicate: impl Fn(&str) -> bool) -> Result<Vec<SourceHit>> {
    let mut hits = Vec::new();
    for entry in WalkDir::new(root).into_iter().filter_entry(|entry| {
        let name = entry.file_name().to_string_lossy();
        name != "target" && name != ".git"
    }) {
        let entry = entry?;
        if !entry.file_type().is_file()
            || entry.path().extension().and_then(|e| e.to_str()) != Some("rs")
        {
            continue;
        }
        let text = read(entry.path())?;
        for (ix, line) in text.lines().enumerate() {
            if predicate(line) {
                hits.push(SourceHit {
                    file: normalize_path(entry.path()),
                    line: ix + 1,
                    text: line.trim().to_string(),
                });
            }
        }
    }
    hits.sort_by(|a, b| (&a.file, a.line).cmp(&(&b.file, b.line)));
    Ok(hits)
}
