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
                items.push(ApiItem {
                    file: normalize_path(root, entry.path()),
                    signature,
                });
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
mod tests {
    use super::{ApiItem, ApiSnapshot, collect_signature, compare_api};
    use std::collections::BTreeSet;

    fn item(signature: &str) -> ApiItem {
        ApiItem {
            file: "crates/moon-ui-components/src/moon/button.rs".to_string(),
            signature: signature.to_string(),
        }
    }

    fn snapshot(signatures: &[&str]) -> ApiSnapshot {
        ApiSnapshot {
            version: 1,
            items: signatures.iter().map(|s| item(s)).collect(),
        }
    }

    /// Catches letting an addition pass unrecorded, which is how the baseline drifted 87 items
    /// behind the code: only removals were compared, so a new public builder never touched the
    /// file. Everything it does not list is also unguarded against a later removal, and the next
    /// legitimate signature change has to sweep the whole backlog into its own diff.
    #[test]
    fn an_unrecorded_addition_fails_the_check() {
        let baseline = snapshot(&["pub fn caret(self) -> Self"]);
        let current = snapshot(&[
            "pub fn caret(self) -> Self",
            "pub fn rotation(self) -> Self",
        ]);

        let failures = compare_api(&baseline, &current, &BTreeSet::new());

        assert_eq!(failures.len(), 1, "{failures:?}");
        assert!(
            failures[0].contains("added and not recorded")
                && failures[0].contains("pub fn rotation(self) -> Self"),
            "{failures:?}"
        );
        assert!(
            compare_api(&current, &current, &BTreeSet::new()).is_empty(),
            "a recorded surface must pass"
        );
    }

    /// Catches losing the removal half while adding the addition half: a removed builder breaks
    /// MoonTerminal's next `cargo update`, which is the reason this guardrail exists at all.
    #[test]
    fn a_removal_still_fails_unless_it_is_declared() {
        let baseline = snapshot(&["pub fn caret(self) -> Self"]);
        let current = snapshot(&[]);

        let failures = compare_api(&baseline, &current, &BTreeSet::new());
        assert_eq!(failures.len(), 1, "{failures:?}");
        assert!(failures[0].contains("removed/changed"), "{failures:?}");

        let approved = BTreeSet::from([item("pub fn caret(self) -> Self")]);
        assert!(compare_api(&baseline, &current, &approved).is_empty());
    }

    #[test]
    fn collect_signature_keeps_multiline_pub_use_until_semicolon() {
        let lines = [
            "    pub use gpui_component::{",
            "        Theme, WindowExt,",
            "    };",
        ];

        let (signature, next) = collect_signature(&lines, 0);

        assert_eq!(next, 3);
        assert_eq!(signature, "pub use gpui_component::{ Theme, WindowExt, };");
    }
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
