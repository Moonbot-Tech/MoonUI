use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context as _, Result, bail};
use serde::{Deserialize, Serialize};

mod contracts;
mod reporting;
mod source_metrics;

use contracts::{check_critical_contracts, contract_checks};
use reporting::{compare_with_baseline, print_human_report};
use source_metrics::source_metrics;

const REPORT_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ComponentEntry {
    pub concept: String,
    pub class: ComponentClass,
    pub behavior_source: String,
    pub theme_source: String,
    pub public_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub escape_path: Option<String>,
    pub upstream_ref: Option<String>,
    pub fork_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub donor_drift_budget: Option<usize>,
    pub contracts: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ComponentManifestFile {
    version: u32,
    components: Vec<ComponentEntry>,
}

#[derive(Debug, Deserialize)]
struct ApprovedMigrationsFile {
    version: u32,
    migrations: Vec<ApprovedClassMigration>,
}

#[derive(Debug, Deserialize)]
struct ApprovedClassMigration {
    concept: String,
    from: ComponentClass,
    to: ComponentClass,
    reason: String,
}

#[derive(Debug, Deserialize)]
struct MirrorBaselineFile {
    version: u32,
    donor_root_provided: bool,
    entries: Vec<MirrorBaselineEntry>,
}

#[derive(Debug, Deserialize)]
struct MirrorBaselineEntry {
    concept: String,
    #[serde(default)]
    donor_changed_files: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ComponentClass {
    Mirror,
    TrackedFork,
    Forged,
    Domain,
    Internal,
    Forbidden,
    Pending,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct SourceMetric {
    pub id: String,
    pub count: usize,
    pub policy: MetricPolicy,
    pub hits: Vec<SourceHit>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum MetricPolicy {
    MustNotIncrease,
    MustBeZeroEventually,
    Informational,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct SourceHit {
    pub file: String,
    pub line: usize,
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ContractCheck {
    pub id: String,
    pub status: ContractStatus,
    pub severity: ContractSeverity,
    pub verifier: ContractVerifier,
    pub details: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ContractStatus {
    Pass,
    Debt,
    Fail,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum ContractSeverity {
    Critical,
    Guardrail,
    Info,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum ContractVerifier {
    StructuralSource,
    BehavioralTest,
    VisualGolden,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ComponentAuditReport {
    pub version: u32,
    pub components: Vec<ComponentEntry>,
    pub source_metrics: Vec<SourceMetric>,
    pub contracts: Vec<ContractCheck>,
}

#[derive(Debug)]
pub struct AuditOptions {
    pub baseline: PathBuf,
    pub update_baseline: bool,
    pub check_baseline: bool,
    pub json: bool,
}

pub fn run(options: AuditOptions) -> Result<()> {
    let root = std::env::current_dir().context("resolve current dir")?;
    let report = build_report(&root)?;

    if options.update_baseline {
        if let Some(parent) = options.baseline.parent() {
            fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
        }
        let json = serde_json::to_string_pretty(&report)?;
        fs::write(&options.baseline, format!("{json}\n"))
            .with_context(|| format!("write {}", options.baseline.display()))?;
        println!(
            "updated component audit baseline: {}",
            options.baseline.display()
        );
        return Ok(());
    }

    let mut failures = Vec::new();
    if options.check_baseline {
        let baseline = read_baseline(&options.baseline)?;
        let approved = approved_class_migrations(&root)?;
        failures.extend(compare_with_baseline(&baseline, &report, &approved));
    }
    failures.extend(check_critical_contracts(&report));

    if options.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print_human_report(&report, &failures);
    }

    if !failures.is_empty() {
        bail!("component audit failed with {} issue(s)", failures.len());
    }

    Ok(())
}

fn read_baseline(path: &Path) -> Result<ComponentAuditReport> {
    let text = fs::read_to_string(path).with_context(|| {
        format!(
            "read baseline {}; run `cargo xtask component-audit --update-baseline` first",
            path.display()
        )
    })?;
    serde_json::from_str(&text).with_context(|| format!("parse {}", path.display()))
}

fn approved_class_migrations(
    root: &Path,
) -> Result<BTreeSet<(String, ComponentClass, ComponentClass)>> {
    let path = root.join("docs/component-class-migrations.json");
    if !path.exists() {
        return Ok(BTreeSet::new());
    }

    let text = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let file: ApprovedMigrationsFile =
        serde_json::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
    if file.version != REPORT_VERSION {
        bail!(
            "approved migrations version {} != audit version {}",
            file.version,
            REPORT_VERSION
        );
    }

    let mut migrations = BTreeSet::new();
    for migration in file.migrations {
        if migration.concept.trim().is_empty() {
            bail!("approved migration contains an empty concept");
        }
        if migration.reason.trim().is_empty() {
            bail!(
                "approved migration {} {:?}->{:?} has no reason",
                migration.concept,
                migration.from,
                migration.to
            );
        }
        migrations.insert((migration.concept, migration.from, migration.to));
    }
    Ok(migrations)
}

fn build_report(root: &Path) -> Result<ComponentAuditReport> {
    let components = load_component_manifest(root)?;
    validate_component_manifest(&components)?;
    Ok(ComponentAuditReport {
        version: REPORT_VERSION,
        components,
        source_metrics: source_metrics(root)?,
        contracts: contract_checks(root)?,
    })
}

fn load_component_manifest(root: &Path) -> Result<Vec<ComponentEntry>> {
    let path = root.join("crates/moon-ui-components/component-manifest.json");
    let text = read(&path)?;
    let mut manifest: ComponentManifestFile =
        serde_json::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
    if manifest.version != REPORT_VERSION {
        bail!(
            "component manifest version {} != audit version {}",
            manifest.version,
            REPORT_VERSION
        );
    }
    manifest
        .components
        .sort_by(|a, b| a.concept.cmp(&b.concept));
    Ok(manifest.components)
}

fn validate_component_manifest(components: &[ComponentEntry]) -> Result<()> {
    let mut seen_concepts = BTreeSet::new();
    let mut seen_public_paths = BTreeMap::<String, String>::new();
    for component in components {
        if component.concept.trim().is_empty() {
            bail!("component manifest contains an empty concept");
        }
        if !seen_concepts.insert(component.concept.clone()) {
            bail!(
                "duplicate component concept in manifest: {}",
                component.concept
            );
        }

        match component.class {
            ComponentClass::Mirror => {
                if component.public_path.is_none() {
                    bail!("Mirror component {} has no public_path", component.concept);
                }
                if component.upstream_ref.is_none() {
                    bail!("Mirror component {} has no upstream_ref", component.concept);
                }
                if component.fork_reason.is_some() {
                    bail!(
                        "Mirror component {} must not have fork_reason; use TrackedFork for reviewed donor drift",
                        component.concept
                    );
                }
                if component.donor_drift_budget.is_some() {
                    bail!(
                        "Mirror component {} must not have donor_drift_budget; use TrackedFork for reviewed donor drift",
                        component.concept
                    );
                }
                if component.contracts.is_empty() {
                    bail!("Mirror component {} has no contracts", component.concept);
                }
            }
            ComponentClass::TrackedFork => {
                if component.public_path.is_none() {
                    bail!(
                        "TrackedFork component {} has no public_path",
                        component.concept
                    );
                }
                if component.upstream_ref.is_none() {
                    bail!(
                        "TrackedFork component {} has no upstream_ref",
                        component.concept
                    );
                }
                if component
                    .fork_reason
                    .as_deref()
                    .is_none_or(|reason| reason.trim().is_empty())
                {
                    bail!(
                        "TrackedFork component {} has no fork_reason",
                        component.concept
                    );
                }
                if component.donor_drift_budget.unwrap_or(0) == 0 {
                    bail!(
                        "TrackedFork component {} has no positive donor_drift_budget",
                        component.concept
                    );
                }
                if component.contracts.is_empty() {
                    bail!(
                        "TrackedFork component {} has no contracts",
                        component.concept
                    );
                }
            }
            ComponentClass::Forged => {
                if component.public_path.is_none() {
                    bail!("Forged component {} has no public_path", component.concept);
                }
                if component.fork_reason.is_none() {
                    bail!("Forged component {} has no fork_reason", component.concept);
                }
                if component.contracts.is_empty() {
                    bail!("Forged component {} has no contracts", component.concept);
                }
            }
            ComponentClass::Pending | ComponentClass::Forbidden | ComponentClass::Domain => {
                if component.public_path.is_some() {
                    bail!(
                        "{:?} component {} must not expose public_path",
                        component.class,
                        component.concept
                    );
                }
            }
            ComponentClass::Internal => {}
        }

        if let Some(escape_path) = &component.escape_path {
            if !matches!(component.class, ComponentClass::Pending) {
                bail!(
                    "{:?} component {} must not expose escape_path",
                    component.class,
                    component.concept
                );
            }
            if !escape_path.starts_with("moon_ui::components::") {
                bail!(
                    "Pending component {} escape_path must stay under moon_ui::components::*",
                    component.concept
                );
            }
        }

        if let Some(public_path) = &component.public_path {
            if let Some(previous) =
                seen_public_paths.insert(public_path.clone(), component.concept.clone())
            {
                bail!(
                    "public_path {public_path} is used by both {previous} and {}",
                    component.concept
                );
            }
        }
    }
    Ok(())
}

fn read(path: impl AsRef<Path>) -> Result<String> {
    fs::read_to_string(path.as_ref()).with_context(|| format!("read {}", path.as_ref().display()))
}

fn normalize_path(path: &Path) -> String {
    let cwd = std::env::current_dir().ok();
    let relative = cwd
        .as_ref()
        .and_then(|root| path.strip_prefix(root).ok())
        .unwrap_or(path);
    relative.to_string_lossy().replace('\\', "/")
}

#[allow(dead_code)]
fn sorted_set(values: impl IntoIterator<Item = String>) -> Vec<String> {
    values
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}
