use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context as _, Result, bail};
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

const REPORT_VERSION: u32 = 1;

#[derive(Debug)]
pub struct MirrorOptions {
    pub baseline: PathBuf,
    pub donor_root: Option<PathBuf>,
    pub update_baseline: bool,
    pub check_baseline: bool,
    pub json: bool,
}

#[derive(Debug, Deserialize)]
struct ComponentManifestFile {
    version: u32,
    components: Vec<ComponentEntry>,
}

#[derive(Debug, Deserialize)]
struct ComponentEntry {
    concept: String,
    class: String,
    public_path: Option<String>,
    upstream_ref: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct MirrorReport {
    version: u32,
    donor_root_provided: bool,
    entries: Vec<MirrorEntry>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct MirrorEntry {
    concept: String,
    class: String,
    public_path: String,
    upstream_ref: String,
    local_paths: Vec<String>,
    local_hash: String,
    local_files: Vec<MirrorFileHash>,
    donor_hash: Option<String>,
    donor_files: Option<Vec<MirrorFileHash>>,
    donor_changed_files: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct MirrorFileHash {
    path: String,
    hash: String,
    bytes: u64,
}

pub fn run(options: MirrorOptions) -> Result<()> {
    let root = std::env::current_dir().context("resolve current dir")?;
    let report = build_report(&root, options.donor_root.as_deref())?;

    if options.update_baseline {
        if let Some(parent) = options.baseline.parent() {
            fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
        }
        let json = serde_json::to_string_pretty(&report)?;
        fs::write(&options.baseline, format!("{json}\n"))
            .with_context(|| format!("write {}", options.baseline.display()))?;
        println!(
            "updated component mirror baseline: {}",
            options.baseline.display()
        );
        return Ok(());
    }

    let mut failures = Vec::new();
    if options.check_baseline {
        let baseline = read_baseline(&options.baseline)?;
        failures.extend(compare_with_baseline(&baseline, &report));
    }

    if options.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print_human_report(&report, &failures);
    }

    if !failures.is_empty() {
        bail!(
            "component mirror check failed with {} issue(s)",
            failures.len()
        );
    }
    Ok(())
}

fn read_baseline(path: &Path) -> Result<MirrorReport> {
    let text = fs::read_to_string(path).with_context(|| {
        format!(
            "read baseline {}; run `cargo xtask component-mirror --update-baseline` first",
            path.display()
        )
    })?;
    serde_json::from_str(&text).with_context(|| format!("parse {}", path.display()))
}

fn build_report(root: &Path, donor_root: Option<&Path>) -> Result<MirrorReport> {
    let source_root = root.join("crates/moon-ui-components/src");
    let components = load_manifest(root)?;
    let mut entries = Vec::new();

    for component in components
        .into_iter()
        .filter(|component| component.class == "Mirror" || component.class == "TrackedFork")
    {
        let public_path = component.public_path.with_context(|| {
            format!(
                "{} component {} has no public_path",
                component.class, component.concept
            )
        })?;
        let upstream_ref = component.upstream_ref.with_context(|| {
            format!(
                "{} component {} has no upstream_ref",
                component.class, component.concept
            )
        })?;
        let paths = mirror_source_paths(&upstream_ref)
            .with_context(|| format!("no mirror source path mapping for {upstream_ref}"))?;
        let local_paths: Vec<String> = paths.iter().map(|path| path.to_string()).collect();
        let local_files = hash_paths(&source_root, &paths)
            .with_context(|| format!("hash local mirror paths for {}", component.concept))?;
        let local_hash = combined_hash(&local_files);

        let (donor_hash, donor_files, donor_changed_files) = if let Some(donor_root) = donor_root {
            let donor_files = hash_paths(donor_root, &paths)
                .with_context(|| format!("hash donor mirror paths for {}", component.concept))?;
            let donor_hash = combined_hash(&donor_files);
            let changed = changed_files(&local_files, &donor_files);
            (Some(donor_hash), Some(donor_files), Some(changed))
        } else {
            (None, None, None)
        };

        entries.push(MirrorEntry {
            concept: component.concept,
            class: component.class,
            public_path,
            upstream_ref,
            local_paths,
            local_hash,
            local_files,
            donor_hash,
            donor_files,
            donor_changed_files,
        });
    }

    entries.sort_by(|a, b| a.concept.cmp(&b.concept));
    Ok(MirrorReport {
        version: REPORT_VERSION,
        donor_root_provided: donor_root.is_some(),
        entries,
    })
}

fn load_manifest(root: &Path) -> Result<Vec<ComponentEntry>> {
    let path = root.join("crates/moon-ui-components/component-manifest.json");
    let text = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let file: ComponentManifestFile =
        serde_json::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
    if file.version != REPORT_VERSION {
        bail!(
            "component manifest version {} != mirror report version {}",
            file.version,
            REPORT_VERSION
        );
    }
    Ok(file.components)
}

fn mirror_source_paths(upstream_ref: &str) -> Option<Vec<&'static str>> {
    let upstream_ref = upstream_symbol(upstream_ref);
    Some(match upstream_ref {
        "Longbridge::accordion" => vec!["accordion.rs"],
        "Longbridge::alert" => vec!["alert.rs"],
        "Longbridge::button" => vec!["button"],
        "Longbridge::checkbox" => vec!["checkbox.rs"],
        "Longbridge::color_picker" => vec!["color_picker.rs"],
        "Longbridge::combobox" => vec!["combobox.rs"],
        "Longbridge::time::calendar" => vec!["time/calendar.rs"],
        "Longbridge::time::date_picker" => vec!["time/date_picker.rs"],
        "Longbridge::description_list" => vec!["description_list.rs"],
        "Longbridge::hover_card" => vec!["hover_card.rs"],
        "Longbridge::input" => vec!["input"],
        "Longbridge::list" => vec!["list"],
        "Longbridge::popover" => vec!["popover.rs"],
        "Longbridge::pagination" => vec!["pagination.rs"],
        "Longbridge::select" => vec!["select.rs"],
        "Longbridge::setting" => vec!["setting"],
        "Longbridge::sheet" => vec!["sheet.rs"],
        "Longbridge::sidebar" => vec!["sidebar"],
        "Longbridge::slider" => vec!["slider.rs"],
        "Longbridge::progress" | "Longbridge::progress::ProgressCircle" => vec!["progress"],
        "Longbridge::resizable" => vec!["resizable"],
        "Longbridge::switch" => vec!["switch.rs"],
        "Longbridge::dialog" => vec!["dialog"],
        "Longbridge::native_menu" => vec!["native_menu"],
        "Longbridge::notification" => vec!["notification.rs"],
        "Longbridge::tree" => vec!["tree.rs"],
        _ => return None,
    })
}

fn upstream_symbol(upstream_ref: &str) -> &str {
    upstream_ref
        .split_once('@')
        .map(|(symbol, _)| symbol)
        .unwrap_or(upstream_ref)
}

fn hash_paths(root: &Path, paths: &[&str]) -> Result<Vec<MirrorFileHash>> {
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    for rel in paths {
        let path = root.join(rel);
        if path.is_file() {
            push_hash(root, &path, &mut seen, &mut out)?;
        } else if path.is_dir() {
            for entry in WalkDir::new(&path).sort_by_file_name() {
                let entry = entry.with_context(|| format!("walk {}", path.display()))?;
                if entry.file_type().is_file() {
                    push_hash(root, entry.path(), &mut seen, &mut out)?;
                }
            }
        } else {
            bail!("mirror path missing: {}", path.display());
        }
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

fn push_hash(
    root: &Path,
    path: &Path,
    seen: &mut BTreeSet<String>,
    out: &mut Vec<MirrorFileHash>,
) -> Result<()> {
    let rel = path
        .strip_prefix(root)
        .with_context(|| format!("strip {} from {}", root.display(), path.display()))?
        .to_string_lossy()
        .replace('\\', "/");
    if !seen.insert(rel.clone()) {
        return Ok(());
    }
    let bytes =
        canonical_source_bytes(fs::read(path).with_context(|| format!("read {}", path.display()))?);
    out.push(MirrorFileHash {
        path: rel,
        hash: fnv_hex(&bytes),
        bytes: bytes.len() as u64,
    });
    Ok(())
}

fn canonical_source_bytes(bytes: Vec<u8>) -> Vec<u8> {
    if !bytes.contains(&b'\r') {
        return bytes;
    }

    let mut normalized = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'\r' {
            normalized.push(b'\n');
            index += if bytes.get(index + 1) == Some(&b'\n') {
                2
            } else {
                1
            };
        } else {
            normalized.push(bytes[index]);
            index += 1;
        }
    }
    normalized
}

fn combined_hash(files: &[MirrorFileHash]) -> String {
    let mut hash = FNV_OFFSET;
    for file in files {
        hash = fnv_update(hash, file.path.as_bytes());
        hash = fnv_update(hash, &[0]);
        hash = fnv_update(hash, file.hash.as_bytes());
        hash = fnv_update(hash, &[0]);
        hash = fnv_update(hash, file.bytes.to_string().as_bytes());
        hash = fnv_update(hash, &[0xFF]);
    }
    format!("{hash:016x}")
}

fn changed_files(local_files: &[MirrorFileHash], donor_files: &[MirrorFileHash]) -> Vec<String> {
    let mut changed = BTreeSet::new();
    for local in local_files {
        match donor_files.iter().find(|donor| donor.path == local.path) {
            Some(donor) if donor.hash == local.hash && donor.bytes == local.bytes => {}
            _ => {
                changed.insert(local.path.clone());
            }
        }
    }
    for donor in donor_files {
        if !local_files.iter().any(|local| local.path == donor.path) {
            changed.insert(donor.path.clone());
        }
    }
    changed.into_iter().collect()
}

const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

fn fnv_hex(bytes: &[u8]) -> String {
    format!("{:016x}", fnv_update(FNV_OFFSET, bytes))
}

fn fnv_update(mut hash: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

fn compare_with_baseline(baseline: &MirrorReport, current: &MirrorReport) -> Vec<String> {
    let mut failures = Vec::new();
    if baseline.version != current.version {
        failures.push(format!(
            "mirror baseline version {} != current version {}",
            baseline.version, current.version
        ));
        return failures;
    }
    if baseline.donor_root_provided && !current.donor_root_provided {
        failures.push(
            "mirror baseline was recorded with donor_root, but current check has no donor_root"
                .to_string(),
        );
    }

    for current_entry in &current.entries {
        let Some(baseline_entry) = baseline
            .entries
            .iter()
            .find(|entry| entry.concept == current_entry.concept)
        else {
            failures.push(format!(
                "new mirror-tracked component without baseline: {}",
                current_entry.concept
            ));
            continue;
        };
        if baseline_entry.class != current_entry.class {
            failures.push(format!(
                "mirror-tracked {} class changed: {} -> {}",
                current_entry.concept, baseline_entry.class, current_entry.class
            ));
        }
        if baseline_entry.upstream_ref != current_entry.upstream_ref {
            failures.push(format!(
                "mirror-tracked {} upstream_ref changed: {} -> {}",
                current_entry.concept, baseline_entry.upstream_ref, current_entry.upstream_ref
            ));
        }
        if baseline_entry.local_paths != current_entry.local_paths {
            failures.push(format!(
                "mirror-tracked {} source paths changed: {:?} -> {:?}",
                current_entry.concept, baseline_entry.local_paths, current_entry.local_paths
            ));
        }
        if baseline_entry.local_hash != current_entry.local_hash {
            failures.push(format!(
                "mirror-tracked {} local hash changed: {} -> {}",
                current_entry.concept, baseline_entry.local_hash, current_entry.local_hash
            ));
        }
        if let (Some(expected), Some(actual)) =
            (&baseline_entry.donor_hash, &current_entry.donor_hash)
        {
            if expected != actual {
                failures.push(format!(
                    "mirror-tracked {} donor hash changed: {} -> {}",
                    current_entry.concept, expected, actual
                ));
            }
        }
    }

    for baseline_entry in &baseline.entries {
        if !current
            .entries
            .iter()
            .any(|entry| entry.concept == baseline_entry.concept)
        {
            failures.push(format!(
                "mirror-tracked component removed from current report: {}",
                baseline_entry.concept
            ));
        }
    }

    failures
}

fn print_human_report(report: &MirrorReport, failures: &[String]) {
    println!("MoonUI component mirror report v{}", report.version);
    let mut by_class = BTreeMap::<&str, usize>::new();
    for entry in &report.entries {
        *by_class.entry(entry.class.as_str()).or_default() += 1;
    }
    println!(
        "mirror-tracked components: {} ({})",
        report.entries.len(),
        by_class
            .into_iter()
            .map(|(class, count)| format!("{class}: {count}"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!("donor root provided: {}", report.donor_root_provided);
    for entry in &report.entries {
        let changed = entry
            .donor_changed_files
            .as_ref()
            .map(|files| files.len().to_string())
            .unwrap_or_else(|| "n/a".to_string());
        println!(
            "  {:<18} {:<11} local={} donor_changed_files={}",
            entry.concept, entry.class, entry.local_hash, changed
        );
    }
    if failures.is_empty() {
        println!("component mirror: PASS");
    } else {
        println!("component mirror: FAIL");
        for failure in failures {
            println!("  - {failure}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::canonical_source_bytes;

    #[test]
    fn canonical_source_bytes_normalizes_windows_and_legacy_newlines() {
        assert_eq!(
            canonical_source_bytes(b"a\r\nb\rc\n".to_vec()),
            b"a\nb\nc\n".to_vec()
        );
    }
}
