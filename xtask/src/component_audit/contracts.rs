//! Semantic contract discovery and validation for component audits.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

use anyhow::{Context as _, Result};
use walkdir::WalkDir;

use super::{
    ComponentAuditReport, ComponentClass, ComponentEntry, ContractCheck, ContractSeverity,
    ContractStatus, ContractVerifier, MirrorBaselineFile, REPORT_VERSION, load_component_manifest,
    read,
};

/// Builds the semantic component checks from the manifest, sources, and registered Rust tests.
///
/// Returns an error when an input cannot be read or parsed.
pub(super) fn contract_checks(root: &Path) -> Result<Vec<ContractCheck>> {
    let components = load_component_manifest(root)?;
    let tests = collect_rust_tests(&root.join("crates/moon-ui-components/src"))?;
    let data_table = read(root.join("crates/moon-ui-components/src/moon/data_table.rs"))?;
    let context_menu = read(root.join("crates/moon-ui-components/src/moon/context_menu.rs"))?;
    let root_source = read(root.join("crates/moon-ui-components/src/root.rs"))?;
    let moon_root_source = read(root.join("crates/moon-ui-components/src/moon/root.rs"))?;
    let facade = read(root.join("crates/moon-ui/src/lib.rs"))?;
    let gallery = read(root.join("crates/moon-ui-gallery/src/main.rs"))?;
    let snapshot_tool = read(root.join("tools/capture-gallery-snapshots.ps1"))?;

    let gallery_missing = public_components_missing_from_gallery(&components, &gallery);
    let visual_missing = missing_visual_baselines(root);
    let mirror_class_drift_issues = mirror_class_drift_issues(root, &components)?;
    let mut checks = vec![
        test_contract(
            "button.click",
            ContractSeverity::Critical,
            &["test_button_clickable_logic"],
            &tests,
            "button clickability must be covered by a Rust behavior test",
        ),
        test_contract(
            "button.width_api",
            ContractSeverity::Critical,
            &["moon_button_width_builders_preserve_layout_intent"],
            &tests,
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
            "collapsible.click_behavior",
            ContractSeverity::Guardrail,
            &["collapsible_header_click_respects_disabled_and_controlled_state"],
            &tests,
            "collapsible header click must respect disabled and controlled state",
        ),
        test_contract(
            "disclosure.click_behavior",
            ContractSeverity::Guardrail,
            &["disclosure_click_is_inert_without_an_id_or_when_disabled"],
            &tests,
            "a passive disclosure caret must never consume its parent row's click",
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
        test_contract(
            "kbd.keystroke_format",
            ContractSeverity::Guardrail,
            &["kbd_formats_keystrokes_like_longbridge"],
            &tests,
            "MoonKbd must preserve Longbridge-style platform keystroke formatting",
        ),
        test_contract(
            "hotkey_input.capture_behavior",
            ContractSeverity::Guardrail,
            &[
                "hotkey_input_does_not_steal_global_shortcuts_when_idle",
                "hotkey_input_waits_for_non_modifier_key",
                "hotkey_input_commits_full_chord_while_recording",
                "hotkey_input_records_a_lone_modifier_on_release",
                "hotkey_input_ignores_a_modifier_that_led_a_chord",
                "hotkey_input_records_capslock_from_its_state_flip",
                "hotkey_input_modifier_watch_is_silent_while_idle",
                "a_state_snapshot_after_losing_focus_is_not_a_press",
            ],
            &tests,
            "MoonHotkeyInput must record complete shortcuts, including the keys that arrive as modifier changes, without stealing idle app keybindings",
        ),
        test_contract(
            "rating.click_behavior",
            ContractSeverity::Guardrail,
            &[
                "rating_value_and_max_are_clamped",
                "rating_click_value_respects_disabled_and_range",
            ],
            &tests,
            "rating value/click handling must clamp and ignore disabled clicks",
        ),
        test_contract(
            "radio.click_behavior",
            ContractSeverity::Guardrail,
            &["radio_click_value_respects_disabled_state"],
            &tests,
            "radio click handling must select true and ignore disabled clicks",
        ),
        test_contract(
            "status_bar.click_behavior",
            ContractSeverity::Guardrail,
            &["interactive_text_item_dispatches_click_without_external_hitbox"],
            &tests,
            "status-bar text actions must dispatch from their rendered bounds while static labels and separators remain inert",
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
                "fitted_root_context_menu_grows_and_stays_inside_scaled_viewports",
                "fitted_root_context_menu_dismisses_once_per_escape_or_outside_click",
            ],
            &tests,
            "context menu origin must clamp fitted width and height to the viewport while the Root-owned route preserves one-shot dismissal",
        ),
        test_contract(
            "dropdown.select_behavior",
            ContractSeverity::Guardrail,
            &[
                "menu_item_clickability_respects_kind_and_disabled_state",
                "rendered_action_label_dispatches_while_static_label_stays_inert",
                "dropdown_select_plan_respects_close_and_controlled_state",
            ],
            &tests,
            "dropdown/menu selection must accept enabled item and action-label rows, ignore disabled/static rows, and close only when configured",
        ),
        test_contract(
            "dropdown.menu_placement",
            ContractSeverity::Guardrail,
            &[
                "open_menu_hangs_just_below_its_trigger",
                "supplied_bounds_menu_also_hangs_just_below_its_trigger",
            ],
            &tests,
            "an open dropdown menu must hug its trigger's bottom edge on both the in-flow and the caller-supplied-bounds path; the gallery snapshots render popups closed and cannot see this",
        ),
        test_contract(
            "dropdown.fitted_width",
            ContractSeverity::Guardrail,
            &[
                "fitted_trigger_preserves_caret_at_independent_scale_extremes",
                "scaled_trigger_uses_font_width_without_clipping_component_chrome",
                "fitted_dropdown_stays_inside_both_viewport_edges_at_independent_scales",
            ],
            &tests,
            "fitted and scaled dropdowns must preserve trigger chrome and keep their open menus inside the viewport across independent UI and font scales",
        ),
        test_contract(
            "popup_menu.fitted_width",
            ContractSeverity::Guardrail,
            &[
                "fitted_menu_accounts_for_right_label_and_its_gap",
                "fitted_submenu_resolves_width_from_its_own_items",
                "scaled_menu_width_retains_fitted_rows_at_independent_scale_extremes",
                "menu_max_height_distinguishes_ui_scaled_and_rendered_values",
                "palette_only_menu_render_rejects_measured_width_policies",
            ],
            &tests,
            "fitted and scaled popup menus must reserve scaled row chrome, trailing-label geometry, maximum height, and per-level submenu sizing",
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
        test_contract(
            "tree.strategy_tree_capabilities",
            ContractSeverity::Guardrail,
            &[
                "empty_folder_is_visible_and_expandable_by_explicit_flag",
                "selected_ids_survive_rebuild_by_id",
                "force_expanded_does_not_mutate_expanded_ids",
                "multi_selection_supports_shift_range_and_secondary_toggle",
                "tree_typed_dnd_builders_are_composable",
            ],
            &tests,
            "tree must preserve strategy-tree requirements: empty folders, id-based state, temporary force-open search state, multi-selection, and typed row DnD hooks",
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
            id: "mirror.class_matches_donor_drift".to_string(),
            status: if mirror_class_drift_issues.is_empty() {
                ContractStatus::Pass
            } else {
                ContractStatus::Fail
            },
            severity: ContractSeverity::Critical,
            verifier: ContractVerifier::StructuralSource,
            details: if mirror_class_drift_issues.is_empty() {
                "Mirror components have zero donor drift; TrackedFork components have reviewed donor drift within manifest budget".to_string()
            } else {
                format!(
                    "component class and donor drift are inconsistent: {}",
                    mirror_class_drift_issues.join("; ")
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
            "popover.content_width",
            ContractSeverity::Guardrail,
            &[
                "content_width_policies_reserve_scaled_popup_chrome",
                "intrinsic_popover_shrink_wraps_its_rendered_child",
            ],
            &tests,
            "popover content-width policies must add component-owned chrome and intrinsic popovers must shrink-wrap rendered content",
        ),
        test_contract(
            "segmented_control.fitted_interaction",
            ContractSeverity::Guardrail,
            &[
                "fitted_item_preserves_the_boundary_and_ellipsizes_one_past_it",
                "disabled_and_replaced_cells_expose_no_native_interactions",
                "replaced_cell_releases_its_full_width_to_the_inline_editor",
                "rendered_cells_preserve_width_and_gate_native_interactions",
                "fitted_segment_width_survives_high_ui_low_font_render",
            ],
            &tests,
            "fitted segmented cells must keep viable pre-render widths, reserve replacement width for inline content, and gate rendered native interactions",
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
        test_contract(
            "skeleton.longbridge_capabilities",
            ContractSeverity::Guardrail,
            &["skeleton_keeps_longbridge_secondary_and_animation_controls"],
            &tests,
            "MoonSkeleton must preserve secondary and animation controls from Longbridge while keeping Moon styling",
        ),
        pass_if(
            "slider.diffused_visual_state",
            ContractSeverity::Critical,
            ContractVerifier::VisualGolden,
            visual_missing.is_empty(),
            "MoonSlider diffused visual state must be covered by committed gallery golden snapshots",
        ),
        test_contract(
            "date_time_picker.time_behavior",
            ContractSeverity::Guardrail,
            &[
                "picking_a_day_keeps_the_popup_open_for_the_time_drums",
                "spinning_the_time_keeps_the_day",
                "the_drums_wrap_without_disturbing_the_day",
                "clearing_resets_both_halves_of_the_value",
            ],
            &tests,
            "picking a day must keep the popup open for the time drums, spinning the clock must keep the day and never roll it, and clearing must reset both halves",
        ),
        test_contract(
            "time_picker.wheel_behavior",
            ContractSeverity::Guardrail,
            &[
                "a_drum_wraps_in_both_directions",
                "scrolling_up_walks_the_drum_backwards",
                "one_wheel_notch_moves_exactly_one_row",
                "one_wheel_notch_changes_exactly_one_minute",
                "sub_row_movement_accumulates_instead_of_vanishing",
                "the_minute_drum_walks_whole_steps",
                "the_value_reads_back_as_zero_padded_hh_mm",
            ],
            &tests,
            "time drums must wrap in both directions, follow the platform scroll direction, move exactly one value per wheel notch regardless of the system lines-per-scroll setting, accumulate sub-row trackpad movement, walk whole minute steps, and read back as zero-padded hh:mm",
        ),
        test_contract(
            "stepper.value_behavior",
            ContractSeverity::Guardrail,
            &["stepper_next_value_clamps_to_range_and_positive_step"],
            &tests,
            "stepper value changes must clamp to min/max and normalize non-positive steps",
        ),
        test_contract(
            "toggle.click_behavior",
            ContractSeverity::Guardrail,
            &["toggle_click_plan_respects_disabled_and_controlled_state"],
            &tests,
            "toggle click handling must respect disabled and controlled state",
        ),
        test_contract(
            "virtual_list.visible_range_reporting",
            ContractSeverity::Guardrail,
            &[
                "visible_range_observer_never_sees_the_measured_row",
                "flipped_list_reports_the_range_it_renders",
                "scrolled_list_reports_the_rows_under_the_offset",
                "emptied_list_still_reports_its_empty_range",
                "collapsed_list_reports_nothing_at_all",
                "padded_list_still_reports_the_row_it_draws",
            ],
            &tests,
            "the virtual list must report only ranges it renders, never the row it measured, and must still report once it holds no rows",
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

/// Return donor drift issues for mirror and tracked-fork components.
fn mirror_class_drift_issues(root: &Path, components: &[ComponentEntry]) -> Result<Vec<String>> {
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
        let Some(component) = manifest.get(entry.concept.as_str()) else {
            problems.push(format!(
                "{} appears in mirror baseline but not in component manifest",
                entry.concept
            ));
            continue;
        };

        match component.class {
            ComponentClass::Mirror => {
                if !changed_files.is_empty() {
                    problems.push(format!(
                        "{} is Mirror but has {} donor-changed file(s): {}",
                        entry.concept,
                        changed_files.len(),
                        changed_files.join(", ")
                    ));
                }
            }
            ComponentClass::TrackedFork => {
                if changed_files.is_empty() {
                    problems.push(format!(
                        "{} is TrackedFork but currently has no donor drift",
                        entry.concept
                    ));
                    continue;
                }
                let budget = component.donor_drift_budget.unwrap_or(0);
                if changed_files.len() > budget {
                    problems.push(format!(
                        "{} donor drift exceeds budget: {} > {} ({})",
                        entry.concept,
                        changed_files.len(),
                        budget,
                        changed_files.join(", ")
                    ));
                }
                if component
                    .fork_reason
                    .as_deref()
                    .is_none_or(|reason| reason.trim().is_empty())
                {
                    problems.push(format!(
                        "{} is TrackedFork without fork_reason",
                        entry.concept
                    ));
                }
            }
            _ => {
                if !changed_files.is_empty() {
                    problems.push(format!(
                        "{} appears in mirror baseline with donor drift but manifest class is {:?}",
                        entry.concept, component.class
                    ));
                }
            }
        }
    }

    Ok(problems)
}

/// Load the pinned donor-mirror baseline from the workspace documentation.
fn load_mirror_baseline(root: &Path) -> Result<MirrorBaselineFile> {
    let path = root.join("docs/component-mirror-baseline.json");
    let text = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("parse {}", path.display()))
}

/// Return public manifest components that have no gallery coverage entry.
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

/// Indexed metadata for one discovered Rust test function.
#[derive(Debug)]
struct RustTest {
    file: String,
    line: usize,
    ignored: bool,
}

/// Lookup tables for enabled and ignored Rust tests discovered in component sources.
#[derive(Debug)]
struct TestIndex {
    tests: BTreeMap<String, RustTest>,
}

/// Index named Rust tests under the supplied component source root.
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

/// Parse a bare Rust function name from a trimmed function declaration.
fn parse_rust_fn_name(trimmed: &str) -> Option<&str> {
    let rest = trimmed
        .strip_prefix("fn ")
        .or_else(|| trimmed.strip_prefix("async fn "))?;
    let name = rest
        .split(|c: char| !(c == '_' || c.is_ascii_alphanumeric()))
        .next()?;
    (!name.is_empty()).then_some(name)
}

/// Evaluate a behavioral-test contract against the discovered test index.
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

/// Return missing dark/light visual baselines for gallery pages that require them.
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

/// Build a contract result whose status reflects the supplied condition.
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

/// Return human-readable failures for every non-passing critical contract.
pub(super) fn check_critical_contracts(report: &ComponentAuditReport) -> Vec<String> {
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
