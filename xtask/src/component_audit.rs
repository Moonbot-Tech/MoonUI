use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context as _, Result, bail};
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

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
                if component.contracts.is_empty() {
                    bail!("Mirror component {} has no contracts", component.concept);
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

fn source_metrics(root: &Path) -> Result<Vec<SourceMetric>> {
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

fn metric(id: &str, policy: MetricPolicy, hits: Vec<SourceHit>) -> SourceMetric {
    SourceMetric {
        id: id.to_string(),
        count: hits.len(),
        policy,
        hits,
    }
}

fn contract_checks(root: &Path) -> Result<Vec<ContractCheck>> {
    let components = load_component_manifest(root)?;
    let tests = collect_rust_tests(&root.join("crates/moon-ui-components/src"))?;
    let moon_button = read(root.join("crates/moon-ui-components/src/moon/button.rs"))?;
    let data_table = read(root.join("crates/moon-ui-components/src/moon/data_table.rs"))?;
    let context_menu = read(root.join("crates/moon-ui-components/src/moon/context_menu.rs"))?;
    let root_source = read(root.join("crates/moon-ui-components/src/root.rs"))?;
    let moon_root_source = read(root.join("crates/moon-ui-components/src/moon/root.rs"))?;
    let facade = read(root.join("crates/moon-ui/src/lib.rs"))?;
    let gallery = read(root.join("crates/moon-ui-gallery/src/main.rs"))?;
    let snapshot_tool = read(root.join("tools/capture-gallery-snapshots.ps1"))?;

    let gallery_missing = public_components_missing_from_gallery(&components, &gallery);
    let visual_missing = missing_visual_baselines(root);
    let mirror_drift_without_reason = mirror_drift_without_reason(root, &components)?;
    let mut checks = vec![
        test_contract(
            "button.click",
            ContractSeverity::Critical,
            &["test_button_clickable_logic"],
            &tests,
            "button clickability must be covered by a Rust behavior test",
        ),
        pass_if(
            "button.width_api",
            ContractSeverity::Critical,
            ContractVerifier::StructuralSource,
            moon_button.contains("pub fn width(mut self, width: f32) -> Self")
                && moon_button.contains("pub fn full_width(mut self) -> Self")
                && moon_button.contains("button = button.w(px(width))")
                && moon_button.contains("button = button.w_full()"),
            "MoonButton width/full_width builders must preserve layout intent",
        ),
        pass_if(
            "button.mono_font",
            ContractSeverity::Critical,
            ContractVerifier::VisualGolden,
            visual_missing.is_empty(),
            "MoonButton::mono must affect rendered text font family",
        ),
        pass_if(
            "checkbox.checked_glyph.asset",
            ContractSeverity::Critical,
            ContractVerifier::VisualGolden,
            visual_missing.is_empty(),
            "checked checkbox must render the Moon check SVG asset, not a text glyph",
        ),
        test_contract(
            "checkbox.click_toggles",
            ContractSeverity::Critical,
            &["test_checkbox_handle_click_toggles_and_calls_handler"],
            &tests,
            "checkbox click path must toggle state and call Moon on_change",
        ),
        test_contract(
            "input.utf8_boundary_clamp",
            ContractSeverity::Critical,
            &["test_clamp_to_char_boundary_never_returns_middle_of_utf8_codepoint"],
            &tests,
            "input text slicing must clamp to UTF-8 char boundaries",
        ),
        test_contract(
            "input.tone_accent",
            ContractSeverity::Critical,
            &["test_input_tone_builder_sets_accent_tone"],
            &tests,
            "MoonInput and MoonTextArea tone must affect input selected/focus accent",
        ),
        test_contract(
            "input.mask_contract",
            ContractSeverity::Critical,
            &[
                "test_mask_pattern1",
                "test_mask_pattern2",
                "test_number_input_undo_with_mask",
            ],
            &tests,
            "input mask and number mask behavior must be covered by Rust tests",
        ),
        pass_if(
            "data_table.text_clipping",
            ContractSeverity::Critical,
            ContractVerifier::VisualGolden,
            visual_missing.is_empty(),
            "data table cells must clip text inside their column",
        ),
        pass_if(
            "data_table.scroll_axis",
            ContractSeverity::Critical,
            ContractVerifier::StructuralSource,
            data_table.contains("restrict_scroll_to_axis"),
            "data table x-scroll layer must opt into GPUI axis restriction; GPUI scroll behavior is covered by scroll tests",
        ),
        pass_if(
            "context_menu.root_owned",
            ContractSeverity::Guardrail,
            ContractVerifier::StructuralSource,
            context_menu.contains("open_context_menu")
                || context_menu.contains("MoonContextMenuOverlay"),
            "context menu must be rooted in Moon overlay/window ownership",
        ),
        test_contract(
            "context_menu.edge_clamp",
            ContractSeverity::Guardrail,
            &[
                "context_menu_origin_clamps_to_viewport_edges",
                "context_menu_requested_max_height_limits_vertical_clamp",
            ],
            &tests,
            "context menu origin must clamp to the viewport edges and honor max-height",
        ),
        test_contract(
            "dock.behavior_contracts",
            ContractSeverity::Guardrail,
            &[
                "moon_dock_panel_builder_flags_are_observable",
                "dock_item_add_panel_creates_tabs_and_activates_new_panel",
                "dock_clamps_tile_meta_inside_root_bounds",
                "move_panel_to_tabs_resolves_target_after_take",
                "move_panel_to_tabs_ignores_self_drop_before_take",
            ],
            &tests,
            "dock must preserve panel flags, tab creation, tab-move target resolution, self-drop guards, tile clamping, and cached panel embedding",
        ),
        pass_if(
            "legacy_dock.internal_only",
            ContractSeverity::Guardrail,
            ContractVerifier::StructuralSource,
            components.iter().any(|component| {
                component.concept == "longbridge_dock_legacy"
                    && component.class == ComponentClass::Internal
                    && component.public_path.is_none()
                    && component
                        .upstream_ref
                        .as_deref()
                        .is_some_and(|upstream_ref| upstream_ref.starts_with("Longbridge::dock"))
            }) && !facade.contains("pub mod components")
                && !facade.contains("dock::"),
            "legacy Longbridge dock must stay manifest-owned as Internal and must not be exported through the public moon_ui facade",
        ),
        ContractCheck {
            id: "mirror.donor_drift_requires_reason".to_string(),
            status: if mirror_drift_without_reason.is_empty() {
                ContractStatus::Pass
            } else {
                ContractStatus::Fail
            },
            severity: ContractSeverity::Critical,
            verifier: ContractVerifier::StructuralSource,
            details: if mirror_drift_without_reason.is_empty() {
                "every Mirror component with donor drift has an explicit fork_reason or has been reclassified".to_string()
            } else {
                format!(
                    "Mirror component(s) have donor drift without fork_reason: {}",
                    mirror_drift_without_reason.join("; ")
                )
            },
        },
        pass_if(
            "root.moon_owned_type",
            ContractSeverity::Guardrail,
            ContractVerifier::StructuralSource,
            root_source.contains("pub struct MoonRoot")
                && root_source.contains("pub type Root = MoonRoot")
                && !moon_root_source.contains("pub type MoonRoot = crate::Root"),
            "MoonRoot must be the real root type; Root may remain only as a compatibility alias",
        ),
        test_contract(
            "popover.open_close_lifecycle",
            ContractSeverity::Guardrail,
            &["test_popover_builder_chaining"],
            &tests,
            "popover open/close builder lifecycle must stay covered by Rust tests",
        ),
        test_contract(
            "select.open_select_lifecycle",
            ContractSeverity::Guardrail,
            &[
                "test_select_initial_selection_seeds_cursor",
                "test_select_initial_grouped_selection_seeds_cursor",
            ],
            &tests,
            "select open/select lifecycle must keep initial selection and cursor state covered",
        ),
        pass_if(
            "slider.diffused_visual_state",
            ContractSeverity::Critical,
            ContractVerifier::VisualGolden,
            visual_missing.is_empty(),
            "MoonSlider diffused visual state must be covered by committed gallery golden snapshots",
        ),
        pass_if(
            "window_frame.visual_types",
            ContractSeverity::Critical,
            ContractVerifier::VisualGolden,
            visual_missing.is_empty(),
            "MoonWindowFrame visual chrome types must be covered by committed gallery golden snapshots",
        ),
        pass_if(
            "visual_snapshots",
            ContractSeverity::Guardrail,
            ContractVerifier::VisualGolden,
            gallery.contains("--snapshot-dir")
                && gallery.contains("--theme")
                && gallery.contains("theme_mode_name")
                && snapshot_tool.contains("$Themes = @(\"Dark\", \"Light\")")
                && gallery.contains("snapshot_window_image(")
                && gallery.contains("clear_snapshot_dir")
                && snapshot_tool.contains("cargo run -p moon-ui-gallery --features snapshot")
                && snapshot_tool.contains("Compare-Png")
                && snapshot_tool.contains("elseif ($Compare)")
                && visual_missing.is_empty(),
            if visual_missing.is_empty() {
                "gallery must own page switching, light/dark theme switching, committed golden PNGs, and visual diff comparison"
            } else {
                "gallery visual golden snapshot(s) are missing"
            },
        ),
        ContractCheck {
            id: "gallery.manifest_coverage".to_string(),
            status: if gallery_missing.is_empty() {
                ContractStatus::Pass
            } else {
                ContractStatus::Fail
            },
            severity: ContractSeverity::Critical,
            verifier: ContractVerifier::StructuralSource,
            details: if gallery_missing.is_empty() {
                "gallery coverage must include every public manifest component".to_string()
            } else {
                format!(
                    "gallery coverage is missing public manifest component(s): {}",
                    gallery_missing.join(", ")
                )
            },
        },
        ContractCheck {
            id: "gallery.visual_coverage".to_string(),
            status: if visual_missing.is_empty() {
                ContractStatus::Pass
            } else {
                ContractStatus::Fail
            },
            severity: ContractSeverity::Critical,
            verifier: ContractVerifier::VisualGolden,
            details: if visual_missing.is_empty() {
                "gallery dark/light golden snapshots must be committed for every gallery page"
                    .to_string()
            } else {
                format!(
                    "missing committed gallery golden snapshot(s): {}",
                    visual_missing.join(", ")
                )
            },
        },
    ];
    let produced_contracts = checks
        .iter()
        .map(|check| check.id.as_str())
        .collect::<BTreeSet<_>>();
    let manifest_contracts = components
        .iter()
        .flat_map(|component| component.contracts.iter().map(String::as_str))
        .collect::<BTreeSet<_>>();
    let missing_contract_checks = manifest_contracts
        .difference(&produced_contracts)
        .map(|id| (*id).to_string())
        .collect::<Vec<_>>();
    checks.push(ContractCheck {
        id: "manifest.contracts_have_verifiers".to_string(),
        status: if missing_contract_checks.is_empty() {
            ContractStatus::Pass
        } else {
            ContractStatus::Fail
        },
        severity: ContractSeverity::Critical,
        verifier: ContractVerifier::StructuralSource,
        details: if missing_contract_checks.is_empty() {
            "every contract named by the component manifest has an audit verifier".to_string()
        } else {
            format!(
                "manifest contract(s) without audit verifier: {}",
                missing_contract_checks.join(", ")
            )
        },
    });
    checks.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(checks)
}

fn mirror_drift_without_reason(root: &Path, components: &[ComponentEntry]) -> Result<Vec<String>> {
    let baseline = load_mirror_baseline(root)?;
    let manifest = components
        .iter()
        .map(|component| (component.concept.as_str(), component))
        .collect::<BTreeMap<_, _>>();
    let mut problems = Vec::new();

    if baseline.version != REPORT_VERSION {
        problems.push(format!(
            "mirror baseline version {} != audit version {}",
            baseline.version, REPORT_VERSION
        ));
    }
    if !baseline.donor_root_provided {
        problems.push("mirror baseline was recorded without donor_root".to_string());
    }

    for entry in baseline.entries {
        let changed_files = entry.donor_changed_files.unwrap_or_default();
        if changed_files.is_empty() {
            continue;
        }

        let Some(component) = manifest.get(entry.concept.as_str()) else {
            continue;
        };
        if component.class != ComponentClass::Mirror {
            continue;
        }
        if component
            .fork_reason
            .as_deref()
            .is_some_and(|reason| !reason.trim().is_empty())
        {
            continue;
        }

        problems.push(format!(
            "{} ({} donor-changed file(s): {})",
            entry.concept,
            changed_files.len(),
            changed_files.join(", ")
        ));
    }

    Ok(problems)
}

fn load_mirror_baseline(root: &Path) -> Result<MirrorBaselineFile> {
    let path = root.join("docs/component-mirror-baseline.json");
    let text = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("parse {}", path.display()))
}

fn public_components_missing_from_gallery(
    components: &[ComponentEntry],
    gallery: &str,
) -> Vec<String> {
    let mut missing = Vec::new();
    for component in components {
        let Some(public_path) = &component.public_path else {
            continue;
        };
        let public_name = public_path.rsplit("::").next().unwrap_or(public_path);
        if !gallery.contains(&format!("\"{public_name}\"")) {
            missing.push(format!("{} ({public_path})", component.concept));
        }
    }
    missing
}

#[derive(Debug)]
struct RustTest {
    file: String,
    line: usize,
    ignored: bool,
}

#[derive(Debug)]
struct TestIndex {
    tests: BTreeMap<String, RustTest>,
}

fn collect_rust_tests(src_root: &Path) -> Result<TestIndex> {
    let mut tests = BTreeMap::new();
    for entry in WalkDir::new(src_root).sort_by_file_name() {
        let entry = entry.with_context(|| format!("walk {}", src_root.display()))?;
        if !entry.file_type().is_file() || entry.path().extension().is_none_or(|ext| ext != "rs") {
            continue;
        }

        let rel = entry
            .path()
            .strip_prefix(src_root)
            .with_context(|| {
                format!(
                    "strip {} from {}",
                    src_root.display(),
                    entry.path().display()
                )
            })?
            .to_string_lossy()
            .replace('\\', "/");
        let text = read(entry.path().to_path_buf())?;
        let mut attrs = Vec::<String>::new();
        for (ix, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("#[") {
                attrs.push(trimmed.to_string());
                continue;
            }
            if let Some(name) = parse_rust_fn_name(trimmed) {
                let is_test = attrs.iter().any(|attr| {
                    attr == "#[test]"
                        || attr == "#[gpui::test]"
                        || attr.starts_with("#[gpui::test(")
                });
                if is_test {
                    tests.insert(
                        name.to_string(),
                        RustTest {
                            file: rel.clone(),
                            line: ix + 1,
                            ignored: attrs.iter().any(|attr| attr.starts_with("#[ignore")),
                        },
                    );
                }
            }
            if !trimmed.is_empty() && !trimmed.starts_with("//") {
                attrs.clear();
            }
        }
    }
    Ok(TestIndex { tests })
}

fn parse_rust_fn_name(trimmed: &str) -> Option<&str> {
    let rest = trimmed
        .strip_prefix("fn ")
        .or_else(|| trimmed.strip_prefix("async fn "))?;
    let name = rest
        .split(|c: char| !(c == '_' || c.is_ascii_alphanumeric()))
        .next()?;
    (!name.is_empty()).then_some(name)
}

fn test_contract(
    id: &str,
    severity: ContractSeverity,
    required_tests: &[&str],
    tests: &TestIndex,
    details: &str,
) -> ContractCheck {
    let missing = required_tests
        .iter()
        .filter(|name| !tests.tests.contains_key(**name))
        .copied()
        .collect::<Vec<_>>();
    let ignored = required_tests
        .iter()
        .filter_map(|name| {
            tests
                .tests
                .get(*name)
                .filter(|test| test.ignored)
                .map(|test| format!("{} ({}:{})", name, test.file, test.line))
        })
        .collect::<Vec<_>>();

    let status = if missing.is_empty() && ignored.is_empty() {
        ContractStatus::Pass
    } else {
        ContractStatus::Fail
    };
    let details = if status == ContractStatus::Pass {
        format!(
            "{}; covered by Rust test(s): {}",
            details,
            required_tests.join(", ")
        )
    } else {
        let mut problems = Vec::new();
        if !missing.is_empty() {
            problems.push(format!("missing test(s): {}", missing.join(", ")));
        }
        if !ignored.is_empty() {
            problems.push(format!("ignored test(s): {}", ignored.join(", ")));
        }
        format!("{}; {}", details, problems.join("; "))
    };

    ContractCheck {
        id: id.to_string(),
        status,
        severity,
        verifier: ContractVerifier::BehavioralTest,
        details,
    }
}

fn missing_visual_baselines(root: &Path) -> Vec<String> {
    const THEMES: &[&str] = &["Dark", "Light"];
    const PAGES: &[&str] = &[
        "Controls",
        "Inputs",
        "Data",
        "Overlays",
        "Layout",
        "NewControls",
        "Composites",
        "Stateful",
    ];

    let baseline_root = root.join("crates/moon-ui-gallery/snapshots/baseline");
    let mut missing = Vec::new();
    for theme in THEMES {
        for page in PAGES {
            let name = format!("{theme}-{page}.png");
            if !baseline_root.join(&name).is_file() {
                missing.push(name);
            }
        }
    }
    missing
}

fn pass_if(
    id: &str,
    severity: ContractSeverity,
    verifier: ContractVerifier,
    condition: bool,
    details: &str,
) -> ContractCheck {
    ContractCheck {
        id: id.to_string(),
        status: if condition {
            ContractStatus::Pass
        } else {
            ContractStatus::Fail
        },
        severity,
        verifier,
        details: details.to_string(),
    }
}

fn check_critical_contracts(report: &ComponentAuditReport) -> Vec<String> {
    report
        .contracts
        .iter()
        .filter(|contract| {
            contract.severity == ContractSeverity::Critical
                && contract.status != ContractStatus::Pass
        })
        .map(|contract| {
            format!(
                "critical contract {} is {:?}: {}",
                contract.id, contract.status, contract.details
            )
        })
        .collect()
}

fn compare_with_baseline(
    baseline: &ComponentAuditReport,
    current: &ComponentAuditReport,
    approved_migrations: &BTreeSet<(String, ComponentClass, ComponentClass)>,
) -> Vec<String> {
    let mut failures = Vec::new();
    if baseline.version != current.version {
        failures.push(format!(
            "baseline version {} != current version {}",
            baseline.version, current.version
        ));
    }

    let baseline_components = keyed_components(&baseline.components);
    let current_components = keyed_components(&current.components);
    for concept in baseline_components.keys() {
        if !current_components.contains_key(concept) {
            failures.push(format!("component manifest entry removed: {concept}"));
        }
    }
    for (concept, current_entry) in &current_components {
        if let Some(baseline_entry) = baseline_components.get(concept) {
            if baseline_entry.class != current_entry.class {
                let approved = approved_migrations.contains(&(
                    concept.to_string(),
                    baseline_entry.class,
                    current_entry.class,
                ));
                if !approved {
                    failures.push(format!(
                        "component {concept} class changed {:?} -> {:?}; record an approved migration first",
                        baseline_entry.class, current_entry.class
                    ));
                }
            }
        }
    }

    let baseline_metrics = keyed_metrics(&baseline.source_metrics);
    let current_metrics = keyed_metrics(&current.source_metrics);
    for (id, current_metric) in current_metrics {
        if let Some(baseline_metric) = baseline_metrics.get(id) {
            match current_metric.policy {
                MetricPolicy::MustNotIncrease | MetricPolicy::MustBeZeroEventually => {
                    if current_metric.count > baseline_metric.count {
                        failures.push(format!(
                            "metric {id} regressed: {} -> {}",
                            baseline_metric.count, current_metric.count
                        ));
                    }
                }
                MetricPolicy::Informational => {}
            }
        }
    }

    let baseline_contracts = keyed_contracts(&baseline.contracts);
    let current_contracts = keyed_contracts(&current.contracts);
    for (id, baseline_contract) in baseline_contracts {
        let Some(current_contract) = current_contracts.get(id) else {
            failures.push(format!("contract removed: {id}"));
            continue;
        };
        if baseline_contract.status == ContractStatus::Pass
            && current_contract.status != ContractStatus::Pass
        {
            failures.push(format!(
                "contract {id} regressed: Pass -> {:?}",
                current_contract.status
            ));
        }
    }

    failures
}

fn keyed_components(entries: &[ComponentEntry]) -> BTreeMap<&str, &ComponentEntry> {
    entries
        .iter()
        .map(|entry| (entry.concept.as_str(), entry))
        .collect()
}

fn keyed_metrics(entries: &[SourceMetric]) -> BTreeMap<&str, &SourceMetric> {
    entries
        .iter()
        .map(|entry| (entry.id.as_str(), entry))
        .collect()
}

fn keyed_contracts(entries: &[ContractCheck]) -> BTreeMap<&str, &ContractCheck> {
    entries
        .iter()
        .map(|entry| (entry.id.as_str(), entry))
        .collect()
}

fn print_human_report(report: &ComponentAuditReport, failures: &[String]) {
    let mut classes = BTreeMap::<ComponentClass, usize>::new();
    for entry in &report.components {
        *classes.entry(entry.class).or_default() += 1;
    }
    println!("MoonUI component audit v{}", report.version);
    println!("components:");
    for (class, count) in classes {
        println!("  {class:?}: {count}");
    }
    println!("source metrics:");
    for metric in &report.source_metrics {
        println!("  {} = {} ({:?})", metric.id, metric.count, metric.policy);
    }
    println!("contracts:");
    for contract in &report.contracts {
        println!(
            "  {:?} {:?} {:?} {} - {}",
            contract.status, contract.severity, contract.verifier, contract.id, contract.details
        );
    }
    if failures.is_empty() {
        println!("component audit: PASS");
    } else {
        println!("component audit: FAIL");
        for failure in failures {
            println!("  - {failure}");
        }
    }
}

fn scan_contains(root: &Path, needle: &str) -> Result<Vec<SourceHit>> {
    scan_files(root, |line| line.contains(needle))
}

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

fn underscore_self_param(signature: &str) -> Option<String> {
    let rest = signature.split_once("(self, ")?.1;
    let end = rest
        .find(|ch: char| ch == ':' || ch == ',' || ch == ')')
        .unwrap_or(rest.len());
    let param = rest[..end].trim();
    param.starts_with('_').then(|| param.to_string())
}

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

fn brace_delta(line: &str) -> i32 {
    line.chars().fold(0, |depth, ch| match ch {
        '{' => depth + 1,
        '}' => depth - 1,
        _ => depth,
    })
}

#[cfg(test)]
mod tests {
    use super::{function_body_uses_param, underscore_self_param};

    #[test]
    fn underscore_self_param_extracts_builder_arg() {
        assert_eq!(
            underscore_self_param("pub fn id(self, _id: impl Into<SharedString>) -> Self {"),
            Some("_id".to_string())
        );
    }

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
}

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
