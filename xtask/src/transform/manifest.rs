//! Cargo manifest and dependency policy for generated crates.

use anyhow::{Context, Result, bail};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;
use toml_edit::{DocumentMut, Item, Value};

use super::{CRATE_PUBLISH_ORDER, crate_name_from_path, is_internal_crate, moon_package_name};

#[cfg(test)]
mod tests;

/// Workspace dependency definitions copied from Zed's root manifest.
pub(super) type WorkspaceDependencies = HashMap<String, Item>;

/// Parse Zed's root workspace dependency table.
pub(super) fn parse_workspace_deps(zed_dir: &Path) -> Result<WorkspaceDependencies> {
    let cargo_toml_path = zed_dir.join("Cargo.toml");
    let content = fs::read_to_string(&cargo_toml_path)
        .with_context(|| format!("Failed to read {}", cargo_toml_path.display()))?;
    let doc: DocumentMut = content.parse()?;

    let mut deps = HashMap::new();

    // Extract [workspace.dependencies]
    if let Some(workspace) = doc.get("workspace") {
        if let Some(workspace_deps) = workspace.get("dependencies") {
            if let Some(table) = workspace_deps.as_table_like() {
                for (name, value) in table.iter() {
                    deps.insert(name.to_string(), value.clone());
                }
            }
        }
    }

    Ok(deps)
}

/// Rewrite one copied crate manifest for Moon package and dependency policy.
pub(super) fn transform_cargo_toml(
    crate_dir: &Path,
    output_dir: &Path,
    original_name: &str,
    workspace_deps: &WorkspaceDependencies,
    zed_tag: &str,
    use_local_deps: bool,
) -> Result<()> {
    let cargo_toml_path = crate_dir.join("Cargo.toml");
    let content = fs::read_to_string(&cargo_toml_path)?;
    let mut doc: DocumentMut = content.parse()?;

    let package_name = moon_package_name(original_name);
    let version = zed_tag_to_version(zed_tag);

    // Update [package] section
    if let Some(package) = doc.get_mut("package") {
        if let Some(table) = package.as_table_like_mut() {
            // Rename package
            table.insert("name", toml_edit::value(&package_name));

            // Set version
            table.insert("version", toml_edit::value(&version));

            // Remove workspace inheritance for edition, use explicit
            if table
                .get("edition")
                .is_some_and(|v| v.as_table_like().is_some())
            {
                table.insert("edition", toml_edit::value("2024"));
            }

            // Set repository
            table.insert(
                "repository",
                toml_edit::value("https://github.com/Moonbot-Tech/MoonUI"),
            );

            // MoonUI is consumed as a git/path workspace for now.
            table.insert("publish", toml_edit::value(false));

            // Ensure license is set
            if !table.contains_key("license") {
                table.insert("license", toml_edit::value("Apache-2.0"));
            }

            // Ensure description is set.
            if !table.contains_key("description") {
                table.insert(
                    "description",
                    toml_edit::value(format!(
                        "Moon standalone build of Zed's {original_name} crate"
                    )),
                );
            }
        }
    }

    // For gpui, set lib name to "gpui" so users can `use gpui::...`
    // even though the package is named "moon-gpui".
    if original_name == "gpui" {
        // Update existing [lib] section or create new one
        if let Some(lib) = doc.get_mut("lib") {
            if let Some(table) = lib.as_table_like_mut() {
                table.insert("name", toml_edit::value("gpui"));
            }
        } else {
            let mut lib_table = toml_edit::Table::new();
            lib_table.insert("name", toml_edit::value("gpui"));
            lib_table.insert("path", toml_edit::value("src/lib.rs"));
            doc.insert("lib", Item::Table(lib_table));
        }

        // Add dev-dependency alias for gpui_platform so users can `use gpui_platform::...` in tests
        if let Some(dev_deps) = doc.get_mut("dev-dependencies") {
            if let Some(table) = dev_deps.as_table_like_mut() {
                let mut dep = toml_edit::InlineTable::new();
                dep.insert("package", "moon-gpui-platform".into());
                if use_local_deps {
                    dep.insert("path", "../moon-gpui-platform".into());
                } else {
                    dep.insert("version", version.clone().into());
                }
                table.insert("gpui_platform", Item::Value(Value::InlineTable(dep)));
            }
        }
    }

    // Transform dependencies, collecting any optional deps that get removed (git-only, no crates.io equiv)
    let mut removed_optionals: Vec<String> = Vec::new();
    if original_name == "sum_tree" {
        remove_dependency(&mut doc, "dependencies", "ztracing");
        remove_dependency(&mut doc, "dev-dependencies", "zlog");
        remove_dependency(&mut doc, "dev-dependencies", "ctor");
    }
    transform_dependencies(
        &mut doc,
        "dependencies",
        workspace_deps,
        &version,
        output_dir,
        use_local_deps,
        &mut removed_optionals,
    )?;
    transform_dependencies(
        &mut doc,
        "dev-dependencies",
        workspace_deps,
        &version,
        output_dir,
        use_local_deps,
        &mut removed_optionals,
    )?;
    transform_dependencies(
        &mut doc,
        "build-dependencies",
        workspace_deps,
        &version,
        output_dir,
        use_local_deps,
        &mut removed_optionals,
    )?;

    // Handle target-specific dependencies
    if let Some(target) = doc.get_mut("target") {
        if let Some(target_table) = target.as_table_like_mut() {
            let targets: Vec<_> = target_table.iter().map(|(k, _)| k.to_string()).collect();
            for target_name in targets {
                if let Some(target_section) =
                    doc.get_mut("target").and_then(|t| t.get_mut(&target_name))
                {
                    if let Some(table) = target_section.as_table_like_mut() {
                        for dep_section in
                            ["dependencies", "dev-dependencies", "build-dependencies"]
                        {
                            if table.contains_key(dep_section) {
                                let mut temp_doc = DocumentMut::new();
                                if let Some(deps) = table.get(dep_section).cloned() {
                                    temp_doc.insert(dep_section, deps);
                                    transform_dependencies(
                                        &mut temp_doc,
                                        dep_section,
                                        workspace_deps,
                                        &version,
                                        output_dir,
                                        use_local_deps,
                                        &mut removed_optionals,
                                    )?;
                                    if let Some(new_deps) = temp_doc.get(dep_section).cloned() {
                                        table.insert(dep_section, new_deps);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Clean up [features] entries that referenced removed optional deps
    for dep_name in &removed_optionals {
        super::remove_dep_from_features(&mut doc, dep_name);
    }

    // Remove inspector feature from gpui_macros and gpui
    if original_name == "gpui_macros" || original_name == "gpui" {
        remove_inspector_feature(&mut doc);
    }

    // Add proptest dependency to crates that need it for tests
    if original_name == "gpui" || original_name == "sum_tree" {
        add_proptest_dependency(&mut doc);
    }

    // Remove workspace lints (not supported for standalone crates)
    doc.remove("lints");

    // Add custom cfg lints for crates that need them
    add_custom_cfg_lints(&mut doc, original_name);

    // Write back
    fs::write(&cargo_toml_path, doc.to_string())?;

    Ok(())
}

/// Rewrite one dependency table, preserving Moon aliases and supported workspace policy.
fn transform_dependencies(
    doc: &mut DocumentMut,
    section: &str,
    workspace_deps: &WorkspaceDependencies,
    version: &str,
    _output_dir: &Path,
    use_local_deps: bool,
    removed_optionals: &mut Vec<String>,
) -> Result<()> {
    let Some(deps) = doc.get_mut(section) else {
        return Ok(());
    };

    let Some(deps_table) = deps.as_table_like_mut() else {
        return Ok(());
    };

    let dep_names: Vec<_> = deps_table.iter().map(|(k, _)| k.to_string()).collect();
    let mut deps_to_remove: Vec<String> = Vec::new();

    for dep_name in dep_names {
        let is_internal = is_internal_crate(&dep_name);

        if let Some(dep) = deps_table.get_mut(&dep_name) {
            // Check if it's a workspace dependency
            let is_workspace = dep.as_table_like().is_some_and(|t| {
                t.get("workspace")
                    .is_some_and(|v| v.as_bool() == Some(true))
            }) || dep.as_str() == Some("workspace = true");

            if is_workspace || dep.get("workspace").is_some() {
                if is_internal {
                    // Internal crate - use package alias so code can keep using original name
                    let package_name = moon_package_name(&dep_name);
                    let mut new_dep = toml_edit::InlineTable::new();
                    new_dep.insert("package", package_name.as_str().into());

                    if use_local_deps {
                        // Use path dependency for local testing (relative to sibling crate)
                        let relative_path = format!("../{package_name}");
                        new_dep.insert("path", relative_path.into());
                    } else {
                        // Use version dependency for non-local generated crates.
                        new_dep.insert("version", version.into());
                    }

                    // Preserve features if any
                    if let Some(table) = dep.as_table_like() {
                        if let Some(features) = table.get("features") {
                            if let Some(arr) = features.as_array() {
                                let mut feat_arr = toml_edit::Array::new();
                                for f in arr.iter() {
                                    // Skip inspector feature
                                    if f.as_str() != Some("inspector") {
                                        feat_arr.push(f.clone());
                                    }
                                }
                                if !feat_arr.is_empty() {
                                    new_dep.insert("features", toml_edit::Value::Array(feat_arr));
                                }
                            }
                        }
                        if let Some(optional) = table.get("optional") {
                            if let Some(b) = optional.as_bool() {
                                new_dep.insert("optional", b.into());
                            }
                        }
                    }

                    // Keep the original name as the key (for aliasing)
                    deps_table.insert(&dep_name, Item::Value(Value::InlineTable(new_dep)));
                } else {
                    // External crate - resolve from workspace
                    if let Some(workspace_dep) = workspace_deps.get(&dep_name) {
                        // Check optional before passing dep to resolve (borrow ends after call)
                        let is_optional = dep
                            .as_table_like()
                            .and_then(|t| t.get("optional"))
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);
                        match resolve_workspace_dep(workspace_dep, dep)? {
                            Some(resolved) => {
                                deps_table.insert(&dep_name, resolved);
                            }
                            None => {
                                // Git-only dep with no version field.
                                // For non-optional [dependencies], try to find the official crates.io
                                // version (e.g. the zed-industries/wgpu fork tracks wgpu 29.x on crates.io).
                                let resolved_via_lookup = if !is_optional
                                    && section == "dependencies"
                                {
                                    let pkg = workspace_dep
                                        .as_table_like()
                                        .and_then(|t| t.get("package"))
                                        .and_then(|v| v.as_str())
                                        .unwrap_or(&dep_name)
                                        .to_string();
                                    // Try known fallbacks first, then lookup
                                    let version = known_git_dep_version(&pkg)
                                        .or_else(|| super::lookup_crates_io_version(&pkg));
                                    version.map(|ver| {
                                        println!(
                                            "  Resolved git-only dep '{dep_name}' to crates.io {pkg}@{ver}"
                                        );
                                        let mut t = toml_edit::InlineTable::new();
                                        t.insert("version", ver.into());
                                        Item::Value(Value::InlineTable(t))
                                    })
                                } else {
                                    None
                                };
                                if let Some(resolved) = resolved_via_lookup {
                                    deps_table.insert(&dep_name, resolved);
                                } else {
                                    if is_optional {
                                        removed_optionals.push(dep_name.clone());
                                    }
                                    deps_to_remove.push(dep_name.clone());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Remove git-only deps after the loop (borrow of individual deps has ended)
    let Some(deps) = doc.get_mut(section) else {
        return Ok(());
    };
    let Some(deps_table) = deps.as_table_like_mut() else {
        return Ok(());
    };
    for dep_name in deps_to_remove {
        deps_table.remove(&dep_name);
    }

    Ok(())
}

/// Resolve a workspace dependency into a standalone crates.io-compatible item.
fn resolve_workspace_dep(workspace_def: &Item, usage: &Item) -> Result<Option<Item>> {
    // Get the base definition from workspace.
    // Git fields (git/rev/branch/tag) are intentionally NOT copied - crates.io rejects them.
    // For git+version deps the version alone is sufficient.
    // For git-only deps (no version), we return None so the caller removes the dep.
    let mut result = if let Some(version) = workspace_def.as_str() {
        // Simple version string
        let mut table = toml_edit::InlineTable::new();
        table.insert("version", version.into());
        Item::Value(Value::InlineTable(table))
    } else if let Some(table) = workspace_def.as_table_like() {
        // Table with version and/or git fields
        let mut new_table = toml_edit::InlineTable::new();

        // Copy version if present (git fields intentionally omitted)
        if let Some(version) = table.get("version").and_then(|v| v.as_str()) {
            new_table.insert("version", version.into());
        }

        // Copy package rename if present
        if let Some(pkg) = table.get("package").and_then(|v| v.as_str()) {
            new_table.insert("package", pkg.into());
        }

        // Copy default-features if present
        if let Some(default_features) = table.get("default-features") {
            if let Some(b) = default_features.as_bool() {
                new_table.insert("default-features", b.into());
            }
        }

        // Copy features from workspace definition
        if let Some(features) = table.get("features") {
            if let Some(arr) = features.as_array() {
                let mut feat_arr = toml_edit::Array::new();
                for f in arr.iter() {
                    feat_arr.push(f.clone());
                }
                new_table.insert("features", toml_edit::Value::Array(feat_arr));
            }
        }

        // If there's no version and no path, this is a git-only dep - not publishable to crates.io
        if !new_table.contains_key("version") && !new_table.contains_key("path") {
            return Ok(None);
        }

        Item::Value(Value::InlineTable(new_table))
    } else {
        workspace_def.clone()
    };

    // Merge features from usage
    if let Some(usage_table) = usage.as_table_like() {
        if let Some(result_table) = result.as_table_like_mut() {
            if let Some(features) = usage_table.get("features") {
                if let Some(arr) = features.as_array() {
                    let mut feat_arr = toml_edit::Array::new();
                    for f in arr.iter() {
                        feat_arr.push(f.clone());
                    }
                    result_table.insert("features", Item::Value(Value::Array(feat_arr)));
                }
            }
            if let Some(optional) = usage_table.get("optional") {
                if let Some(b) = optional.as_bool() {
                    result_table.insert("optional", Item::Value(Value::from(b)));
                }
            }
        }
    }

    Ok(Some(result))
}

/// Remove all references to a dep from the `[features]` section.
/// Handles both bare `"dep_name"` activations and `"dep_name/feature"` entries.
pub(crate) fn remove_dep_from_features(doc: &mut DocumentMut, dep_name: &str) {
    // Phase 1: collect which features need a new array
    let mut modifications: Vec<(String, toml_edit::Array)> = Vec::new();
    if let Some(features) = doc.get("features") {
        if let Some(table) = features.as_table_like() {
            for (feat_name, feat_val) in table.iter() {
                let arr = feat_val
                    .as_value()
                    .and_then(|v| v.as_array())
                    .or_else(|| feat_val.as_array());
                if let Some(arr) = arr {
                    let mut new_arr = toml_edit::Array::new();
                    let mut changed = false;
                    for v in arr.iter() {
                        if let Some(s) = v.as_str() {
                            if s == dep_name || s.starts_with(&format!("{dep_name}/")) {
                                changed = true;
                                continue;
                            }
                        }
                        new_arr.push(v.clone());
                    }
                    if changed {
                        modifications.push((feat_name.to_string(), new_arr));
                    }
                }
            }
        }
    }
    // Phase 2: apply modifications
    if let Some(features) = doc.get_mut("features") {
        if let Some(table) = features.as_table_like_mut() {
            for (feat_name, new_arr) in modifications {
                table.insert(&feat_name, Item::Value(Value::Array(new_arr)));
            }
        }
    }
}

/// Remove one named dependency from a manifest table when present.
fn remove_dependency(doc: &mut DocumentMut, section: &str, dep_name: &str) {
    if let Some(deps) = doc.get_mut(section)
        && let Some(table) = deps.as_table_like_mut()
    {
        table.remove(dep_name);
    }
}

// Features don't need transformation since we use package aliasing
// e.g., `collections/test-support` still works because the dependency key is
// `collections`, even though the actual package is `moon-collections`.

/// Remove inspector feature declarations and inspector-only dependencies.
fn remove_inspector_feature(doc: &mut DocumentMut) {
    // Remove from [features]
    if let Some(features) = doc.get_mut("features") {
        if let Some(table) = features.as_table_like_mut() {
            table.remove("inspector");
        }
    }

    // Remove from dependencies
    if let Some(deps) = doc.get_mut("dependencies") {
        if let Some(table) = deps.as_table_like_mut() {
            // Remove gpui dependency that's only used for inspector
            let dep_names: Vec<_> = table.iter().map(|(k, _)| k.to_string()).collect();
            for name in dep_names {
                if let Some(dep) = table.get(&name) {
                    if let Some(dep_table) = dep.as_table_like() {
                        // Check if this dep is only for inspector feature
                        if let Some(features) = dep_table.get("features") {
                            if features.as_array().is_some_and(|arr| {
                                arr.iter().any(|f| f.as_str() == Some("inspector"))
                                    && arr.len() == 1
                            }) {
                                table.remove(&name);
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Add proptest dependency and feature wiring required by generated test suites.
fn add_proptest_dependency(doc: &mut DocumentMut) {
    // Add to [dependencies] as optional
    if let Some(deps) = doc.get_mut("dependencies") {
        if let Some(table) = deps.as_table_like_mut() {
            if !table.contains_key("proptest") {
                let mut dep = toml_edit::InlineTable::new();
                dep.insert("version", "1".into());
                dep.insert("optional", true.into());
                let mut features = toml_edit::Array::new();
                features.push("attr-macro");
                dep.insert("features", features.into());
                table.insert("proptest", Item::Value(Value::InlineTable(dep)));
            }
        }
    }

    // Add to [dev-dependencies]
    if let Some(deps) = doc.get_mut("dev-dependencies") {
        if let Some(table) = deps.as_table_like_mut() {
            if !table.contains_key("proptest") {
                let mut dep = toml_edit::InlineTable::new();
                dep.insert("version", "1".into());
                let mut features = toml_edit::Array::new();
                features.push("attr-macro");
                dep.insert("features", features.into());
                table.insert("proptest", Item::Value(Value::InlineTable(dep)));
            }
        }
    } else {
        // Create dev-dependencies section if it doesn't exist
        let mut dev_deps = toml_edit::Table::new();
        let mut dep = toml_edit::InlineTable::new();
        dep.insert("version", "1".into());
        let mut features = toml_edit::Array::new();
        features.push("attr-macro");
        dep.insert("features", features.into());
        dev_deps.insert("proptest", Item::Value(Value::InlineTable(dep)));
        doc.insert("dev-dependencies", Item::Table(dev_deps));
    }

    // Add dep:proptest to test-support feature
    if let Some(features) = doc.get_mut("features") {
        if let Some(table) = features.as_table_like_mut() {
            if !table.contains_key("proptest") {
                let mut arr = toml_edit::Array::new();
                arr.push("dep:proptest");
                table.insert("proptest", Item::Value(Value::Array(arr)));
            }
            if let Some(test_support) = table.get_mut("test-support") {
                if let Some(arr) = test_support.as_array_mut() {
                    // Check if dep:proptest is already there
                    let has_proptest = arr.iter().any(|v| v.as_str() == Some("dep:proptest"));
                    if !has_proptest {
                        arr.push("dep:proptest");
                    }
                }
            }
        }
    }
}

/// Add lints configuration for crates that use custom cfg attributes.
fn add_custom_cfg_lints(doc: &mut DocumentMut, crate_name: &str) {
    let check_cfgs: &[&str] = match crate_name {
        "util_macros" => &["cfg(perf_enabled)"],
        "gpui" => &["cfg(rust_analyzer)"],
        // objc crate macros use cargo-clippy cfg
        "gpui_macos" => &["cfg(feature, values(\"cargo-clippy\"))"],
        // nightly_coverage feature for code coverage
        "gpui_linux" => &["cfg(feature, values(\"nightly_coverage\"))"],
        _ => return, // No custom cfgs needed
    };

    // Create [lints.rust] with check-cfg for custom attributes
    let mut check_cfg_arr = toml_edit::Array::new();
    for cfg in check_cfgs {
        check_cfg_arr.push(*cfg);
    }

    let mut unexpected_cfgs = toml_edit::InlineTable::new();
    unexpected_cfgs.insert("level", "warn".into());
    unexpected_cfgs.insert("check-cfg", toml_edit::Value::Array(check_cfg_arr));

    let mut rust_lints = toml_edit::InlineTable::new();
    rust_lints.insert(
        "unexpected_cfgs",
        toml_edit::Value::InlineTable(unexpected_cfgs),
    );

    let mut lints_table = toml_edit::Table::new();
    lints_table.insert(
        "rust",
        Item::Value(toml_edit::Value::InlineTable(rust_lints)),
    );

    doc.insert("lints", Item::Table(lints_table));
}

/// Return the pinned crates.io version for a known git-only dependency.
fn known_git_dep_version(package: &str) -> Option<String> {
    match package {
        "wgpu" => Some("29.0.1".to_string()),
        _ => None,
    }
}

/// Look up the latest version of a package on crates.io via `cargo search`.
/// Returns the version string (e.g. "29.0.1") or None if not found.
pub(crate) fn lookup_crates_io_version(package: &str) -> Option<String> {
    // Retry up to 3 times with backoff to handle crates.io rate limits
    for attempt in 0..3u64 {
        if attempt > 0 {
            std::thread::sleep(std::time::Duration::from_secs(5 * attempt));
        }
        let output = match Command::new("cargo")
            .args(["search", package, "--limit", "1"])
            .output()
        {
            Ok(o) => o,
            Err(_) => continue,
        };
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.is_empty() && !output.status.success() {
            // Likely rate limited, retry
            continue;
        }
        let prefix = format!("{package} = \"");
        for line in stdout.lines() {
            if line.starts_with(&prefix) {
                let after = &line[prefix.len()..];
                let version = after.split('"').next()?;
                return Some(version.to_string());
            }
        }
        // Got a valid response but package not found
        return None;
    }
    None
}

/// Convert a Zed tag into the generated crate version string.
fn zed_tag_to_version(tag: &str) -> String {
    // Convert "v0.185.0" to "0.185.0"
    tag.strip_prefix('v').unwrap_or(tag).to_string()
}

/// Replace the generated root workspace member list with extracted Moon crates.
pub(super) fn write_workspace_manifest(output_dir: &Path) -> Result<()> {
    let root_dir = if output_dir.is_absolute() {
        output_dir
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf()
    } else {
        std::env::current_dir()?
    };
    let cargo_toml_path = root_dir.join("Cargo.toml");
    let content = fs::read_to_string(&cargo_toml_path)
        .with_context(|| format!("Failed to read {}", cargo_toml_path.display()))?;
    let mut doc: DocumentMut = content.parse()?;

    let crate_prefix = output_dir
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("crates");

    let mut members = toml_edit::Array::new();
    members.push("xtask");
    for crate_path in CRATE_PUBLISH_ORDER {
        let crate_name = crate_name_from_path(crate_path);
        members.push(format!("{crate_prefix}/{}", moon_package_name(crate_name)));
    }

    let workspace = doc
        .entry("workspace")
        .or_insert(Item::Table(toml_edit::Table::new()));
    let Some(table) = workspace.as_table_like_mut() else {
        bail!(
            "[workspace] in {} is not a table",
            cargo_toml_path.display()
        );
    };
    table.insert("members", Item::Value(Value::Array(members)));

    fs::write(cargo_toml_path, doc.to_string())?;
    Ok(())
}
