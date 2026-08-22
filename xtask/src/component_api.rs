use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context as _, Result, bail};
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

const API_SNAPSHOT_VERSION: u32 = 1;

#[derive(Debug)]
pub struct ApiOptions {
    pub baseline: PathBuf,
    pub update_baseline: bool,
    pub check_baseline: bool,
    pub json: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ApiSnapshot {
    version: u32,
    items: Vec<ApiItem>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ApiItem {
    file: String,
    signature: String,
}

#[derive(Debug, Deserialize)]
struct ApprovedApiRemovalsFile {
    version: u32,
    removals: Vec<ApprovedApiRemoval>,
}

#[derive(Debug, Deserialize)]
struct ApprovedApiRemoval {
    file: String,
    signature: String,
    reason: String,
}

pub fn run(options: ApiOptions) -> Result<()> {
    let root = std::env::current_dir().context("resolve current dir")?;
    let snapshot = build_snapshot(&root)?;

    if options.update_baseline {
        if let Some(parent) = options.baseline.parent() {
            fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
        }
        fs::write(
            &options.baseline,
            format!("{}\n", serde_json::to_string_pretty(&snapshot)?),
        )
        .with_context(|| format!("write {}", options.baseline.display()))?;
        println!(
            "updated component API baseline: {}",
            options.baseline.display()
        );
        return Ok(());
    }

    let mut failures = Vec::new();
    if options.check_baseline {
        let baseline = read_baseline(&options.baseline)?;
        let approved_removals = approved_api_removals(&root)?;
        failures.extend(compare_api(&baseline, &snapshot, &approved_removals));
    }

    if options.json {
        println!("{}", serde_json::to_string_pretty(&snapshot)?);
    } else {
        println!("MoonUI component API snapshot v{}", snapshot.version);
        println!("public signatures: {}", snapshot.items.len());
        if failures.is_empty() {
            println!("component API snapshot: PASS");
        } else {
            println!("component API snapshot: FAIL");
            for failure in &failures {
                println!("  - {failure}");
            }
            println!("  fix: record an intended change with");
            println!("       cargo xtask component-api --update-baseline");
            println!("  a removal is declared in docs/component-api-removals.json with its reason");
        }
    }

    if !failures.is_empty() {
        bail!(
            "component API snapshot failed with {} issue(s)",
            failures.len()
        );
    }
    Ok(())
}

fn read_baseline(path: &Path) -> Result<ApiSnapshot> {
    let text = fs::read_to_string(path).with_context(|| {
        format!(
            "read API baseline {}; run `cargo xtask component-api --update-baseline` first",
            path.display()
        )
    })?;
    serde_json::from_str(&text).with_context(|| format!("parse {}", path.display()))
}

fn approved_api_removals(root: &Path) -> Result<BTreeSet<ApiItem>> {
    let path = root.join("docs/component-api-removals.json");
    if !path.exists() {
        return Ok(BTreeSet::new());
    }
    let text = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let file: ApprovedApiRemovalsFile =
        serde_json::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
    if file.version != API_SNAPSHOT_VERSION {
        bail!(
            "approved API removals version {} != API snapshot version {}",
            file.version,
            API_SNAPSHOT_VERSION
        );
    }

    let mut removals = BTreeSet::new();
    for removal in file.removals {
        if removal.file.trim().is_empty() || removal.signature.trim().is_empty() {
            bail!("approved API removal contains an empty file or signature");
        }
        if removal.reason.trim().is_empty() {
            bail!(
                "approved API removal {} :: {} has no reason",
                removal.file,
                removal.signature
            );
        }
        removals.insert(ApiItem {
            file: removal.file,
            signature: removal.signature,
        });
    }
    Ok(removals)
}

fn build_snapshot(root: &Path) -> Result<ApiSnapshot> {
    let mut items = Vec::new();
    collect_public_api(
        &root.join("crates/moon-ui-components/src/moon"),
        root,
        &mut items,
    )?;
    collect_public_api(&root.join("crates/moon-ui/src"), root, &mut items)?;
    items.sort();
    items.dedup();
    Ok(ApiSnapshot {
        version: API_SNAPSHOT_VERSION,
        items,
    })
}

fn compare_api(
    baseline: &ApiSnapshot,
    current: &ApiSnapshot,
    approved_removals: &BTreeSet<ApiItem>,
) -> Vec<String> {
    let mut failures = Vec::new();
    if baseline.version != current.version {
        failures.push(format!(
            "API baseline version {} != current version {}",
            baseline.version, current.version
        ));
    }
    let current_set = current.items.iter().collect::<BTreeSet<_>>();
    for item in &baseline.items {
        if !current_set.contains(item) && !approved_removals.contains(item) {
            failures.push(format!(
                "public API removed/changed: {} :: {}",
                item.file, item.signature
            ));
        }
    }
    // Additions are reported too, so the file keeps DESCRIBING the surface instead of only
    // guarding the part of it somebody once wrote down. An addition left unrecorded is not a
    // removal today, but it is unguarded against one tomorrow, and it makes the next legitimate
    // signature change arrive with a stranger's backlog attached to it.
    let baseline_set = baseline.items.iter().collect::<BTreeSet<_>>();
    for item in &current.items {
        if !baseline_set.contains(item) {
            failures.push(format!(
                "public API added and not recorded: {} :: {}",
                item.file, item.signature
            ));
        }
    }
    failures
}

fn collect_public_api(dir: &Path, root: &Path, items: &mut Vec<ApiItem>) -> Result<()> {
    for entry in WalkDir::new(dir) {
        let entry = entry?;
        if !entry.file_type().is_file()
            || entry.path().extension().and_then(|e| e.to_str()) != Some("rs")
        {
            continue;
        }
        let text = fs::read_to_string(entry.path())
            .with_context(|| format!("read {}", entry.path().display()))?;
        let lines = text.lines().collect::<Vec<_>>();
        let mut ix = 0;
        while ix < lines.len() {
            let trimmed = lines[ix].trim();
            if starts_public_api(trimmed) {
                let (signature, next_ix) = collect_signature(&lines, ix);
                if !is_private_child_reexport(entry.path(), &signature) {
                    items.push(ApiItem {
                        file: normalize_path(root, &logical_api_file(root, entry.path())),
                        signature,
                    });
                }
                ix = next_ix;
            } else {
                ix += 1;
            }
        }
    }
    Ok(())
}

fn starts_public_api(line: &str) -> bool {
    let line = line.trim_start();
    line.starts_with("pub struct ")
        || line.starts_with("pub enum ")
        || line.starts_with("pub type ")
        || line.starts_with("pub const ")
        || line.starts_with("pub static ")
        || line.starts_with("pub fn ")
        || line.starts_with("pub use ")
        || line.starts_with("pub mod ")
}

fn collect_signature(lines: &[&str], start: usize) -> (String, usize) {
    let mut ix = start;
    let mut parts = Vec::new();
    let mut paren_depth = 0i32;
    let mut angle_depth = 0i32;
    let first_line = lines[start].trim_start();
    let can_end_on_open_brace = !first_line.starts_with("pub use ");
    while ix < lines.len() {
        let line = lines[ix].trim();
        let mut visible = line
            .split("//")
            .next()
            .unwrap_or(line)
            .trim()
            .trim()
            .to_string();
        if can_end_on_open_brace {
            visible = visible.trim_end_matches('{').trim().to_string();
        }
        if !visible.is_empty() {
            paren_depth += visible.matches('(').count() as i32;
            paren_depth -= visible.matches(')').count() as i32;
            angle_depth += visible.matches('<').count() as i32;
            angle_depth -= visible.matches('>').count() as i32;
            parts.push(visible);
        }
        ix += 1;
        let joined = parts.join(" ");
        if paren_depth <= 0
            && angle_depth <= 0
            && (joined.ends_with(';')
                || joined.ends_with('}')
                || (can_end_on_open_brace && lines[ix.saturating_sub(1)].contains('{')))
        {
            return (normalize_signature(&joined), ix);
        }
        if ix - start > 24 {
            return (normalize_signature(&joined), ix);
        }
    }
    (normalize_signature(&parts.join(" ")), ix)
}

#[cfg(test)]
mod tests;

/// Returns the public component source that owns an implementation file.
///
/// Files below `moon/<component>/` are implementation details of the sibling
/// `moon/<component>.rs` module, so moving an item between those files does not change its API.
fn logical_api_file(root: &Path, path: &Path) -> PathBuf {
    let moon_root = root.join("crates/moon-ui-components/src/moon");
    let Ok(relative) = path.strip_prefix(&moon_root) else {
        return path.to_path_buf();
    };
    let mut components = relative.components();
    let Some(component) = components.next() else {
        return path.to_path_buf();
    };
    if components.next().is_none() {
        return path.to_path_buf();
    }

    moon_root.join(component).with_extension("rs")
}

/// Returns whether a signature only re-exports a private sibling implementation module.
///
/// The declarations in the child module are recorded against the owning component file, while
/// the externally reachable names remain guarded by `moon/mod.rs` and the compile gate.
fn is_private_child_reexport(source: &Path, signature: &str) -> bool {
    let Some(target) = signature.strip_prefix("pub use ") else {
        return false;
    };
    let Some(module) = target.split("::").next() else {
        return false;
    };

    let is_local_name = !matches!(module, "crate" | "self" | "super")
        && !module.is_empty()
        && module
            .chars()
            .all(|character| character == '_' || character.is_ascii_alphanumeric());
    if !is_local_name {
        return false;
    }

    let module_dir = source.with_extension("");
    module_dir.join(format!("{module}.rs")).is_file()
        || module_dir.join(module).join("mod.rs").is_file()
}

fn normalize_signature(signature: &str) -> String {
    signature
        .trim()
        .trim_end_matches('{')
        .trim()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn normalize_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}
