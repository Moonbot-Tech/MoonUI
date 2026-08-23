//! Zed GPUI extraction orchestration.

mod manifest;
mod source_patches;

use anyhow::{Result, bail};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

pub(crate) use manifest::{lookup_crates_io_version, remove_dep_from_features};

const APACHE_2_LICENSE: &str = include_str!("../../LICENSE");

/// GPUI crates to extract, in topological order (dependencies first).
pub const CRATE_PUBLISH_ORDER: &[&str] = &[
    // Tier 1 - Leaf crates
    "gpui_util",
    "gpui_shared_string",
    "collections",
    "refineable/derive_refineable",
    "refineable",
    "tooling/perf",
    "util_macros",
    "util",
    // Tier 2 - Core infrastructure
    "scheduler",
    "sum_tree",
    "http_client",
    "http_client_tls",
    "reqwest_client",
    "media",
    // Tier 3 - Main crates
    "gpui_macros",
    "gpui",
    // Tier 4 - Platform backends
    "gpui_wgpu",
    "gpui_macos",
    "gpui_linux",
    "gpui_windows",
    "gpui_web",
    // Tier 5 - Facade
    "gpui_platform",
];

/// Map from original Zed crate name to Moon package name.
pub fn moon_package_name(name: &str) -> String {
    if name == "gpui" {
        return "moon-gpui".to_string();
    }
    let kebab = name.replace('_', "-");
    format!("moon-{kebab}")
}

/// Extract the crate name from an extraction entry (handles nested paths).
pub fn crate_name_from_path(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// Check if a dependency name matches any extracted crate.
pub fn is_internal_crate(dep_name: &str) -> bool {
    CRATE_PUBLISH_ORDER
        .iter()
        .any(|path| crate_name_from_path(path) == dep_name)
}

/// Transform the configured Zed GPUI crates into the requested output tree.
///
/// The output directory is removed before extraction, so callers must provide a disposable target.
pub fn run(
    zed_tag: &str,
    zed_path: Option<&str>,
    output_dir: &str,
    use_local_deps: bool,
) -> Result<()> {
    println!("Transforming gpui from zed tag: {zed_tag}");
    if use_local_deps {
        println!("Using path dependencies for local testing");
    }

    // Get or clone zed repo
    let zed_dir = match zed_path {
        Some(path) => {
            // Use local path as-is (assume already at correct version)
            let path = PathBuf::from(path);
            println!("Using local zed at: {}", path.display());
            path
        }
        None => clone_zed(zed_tag)?,
    };

    // Parse zed's root Cargo.toml to get workspace dependency versions
    let workspace_deps = manifest::parse_workspace_deps(&zed_dir)?;
    println!("Parsed {} workspace dependencies", workspace_deps.len());

    // Create output directory
    let output_path = PathBuf::from(output_dir);
    if output_path.exists() {
        fs::remove_dir_all(&output_path)?;
    }
    fs::create_dir_all(&output_path)?;
    write_root_license_files(&output_path)?;

    // Transform each crate
    for crate_name in CRATE_PUBLISH_ORDER {
        println!("Transforming: {crate_name}");
        transform_crate(
            &zed_dir,
            &output_path,
            crate_name,
            &workspace_deps,
            zed_tag,
            use_local_deps,
        )?;
    }

    // Write metadata file
    write_metadata(&output_path, zed_tag, &zed_dir)?;
    manifest::write_workspace_manifest(&output_path)?;

    println!("\nTransform complete! Crates written to: {output_dir}");
    println!("Run 'cargo build --workspace' to verify.");

    Ok(())
}

/// Clone the requested Zed tag into a retained temporary directory.
fn clone_zed(tag: &str) -> Result<PathBuf> {
    let temp_dir = tempfile::tempdir()?;
    let path = temp_dir.keep();

    println!("Cloning zed at tag {tag}...");
    let status = Command::new("git")
        .args([
            "clone",
            "--depth=1",
            "--branch",
            tag,
            "https://github.com/zed-industries/zed.git",
            path.to_str().unwrap(),
        ])
        .status()?;

    if !status.success() {
        bail!("Failed to clone zed repository");
    }

    Ok(path)
}

/// Copy and rewrite one crate while preserving the extraction step order.
fn transform_crate(
    zed_dir: &Path,
    output_dir: &Path,
    crate_path: &str,
    workspace_deps: &manifest::WorkspaceDependencies,
    zed_tag: &str,
    use_local_deps: bool,
) -> Result<()> {
    // Handle paths that start with "tooling/" specially
    let src_dir = if crate_path.starts_with("tooling/") {
        zed_dir.join(crate_path)
    } else {
        zed_dir.join("crates").join(crate_path)
    };
    if !src_dir.exists() {
        bail!("Crate not found: {}", src_dir.display());
    }

    // Extract just the crate name from path (e.g., "refineable/derive_refineable" -> "derive_refineable")
    let crate_name = crate_path.rsplit('/').next().unwrap_or(crate_path);
    let package_name = moon_package_name(crate_name);
    let dest_dir = output_dir.join(&package_name);

    // Copy crate directory
    copy_dir_recursive(&src_dir, &dest_dir)?;
    materialize_apache_license_files(&dest_dir)?;

    // Patch examples that reference external assets
    if crate_name == "gpui" {
        source_patches::patch_text_example(&dest_dir)?;
    }

    // Transform Cargo.toml
    manifest::transform_cargo_toml(
        &dest_dir,
        output_dir,
        crate_name,
        workspace_deps,
        zed_tag,
        use_local_deps,
    )?;

    // MoonUI intentionally does not carry Zed's GPL zlog/ztracing helper crates.
    if crate_name == "sum_tree" {
        source_patches::patch_sum_tree_tracing(&dest_dir)?;
    }

    // Patch source files for specific crates to remove inspector feature references
    if crate_name == "gpui_macros" || crate_name == "gpui" {
        source_patches::patch_inspector_cfgs(&dest_dir)?;
    }

    // Patch gpui_macos to fix unnecessary unsafe block
    if crate_name == "gpui_macos" {
        source_patches::patch_gpui_macos_source(&dest_dir)?;
    }

    Ok(())
}

/// Recursively copy a crate directory into its generated destination.
fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<()> {
    fs::create_dir_all(dest)?;

    for entry in WalkDir::new(src) {
        let entry = entry?;
        let relative = entry.path().strip_prefix(src)?;
        let target = dest.join(relative);

        if entry.file_type().is_dir() {
            fs::create_dir_all(&target)?;
        } else {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(entry.path(), &target)?;
        }
    }

    Ok(())
}

/// Write the workspace root Apache license used by generated crates.
fn write_root_license_files(output_dir: &Path) -> Result<()> {
    let root_dir = if output_dir.is_absolute() {
        output_dir
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf()
    } else {
        std::env::current_dir()?
    };

    write_utf8_no_bom(&root_dir.join("LICENSE"), APACHE_2_LICENSE)?;
    Ok(())
}

/// Replace copied license pointers with complete Apache license text.
fn materialize_apache_license_files(crate_dir: &Path) -> Result<()> {
    let mut found = false;

    for entry in WalkDir::new(crate_dir) {
        let entry = entry?;
        if entry.path().file_name().and_then(|name| name.to_str()) != Some("LICENSE-APACHE") {
            continue;
        }
        found = true;

        if apache_license_needs_materialization(entry.path())? {
            write_utf8_no_bom(entry.path(), APACHE_2_LICENSE)?;
        }
    }

    if !found {
        write_utf8_no_bom(&crate_dir.join("LICENSE-APACHE"), APACHE_2_LICENSE)?;
    }

    Ok(())
}

/// Return whether a copied Apache license is a symlink or path-pointer placeholder.
fn apache_license_needs_materialization(path: &Path) -> Result<bool> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() {
        return Ok(true);
    }

    let content = fs::read_to_string(path)?;
    let pointer = content.trim_start_matches('\u{feff}').trim();
    Ok(pointer == "LICENSE"
        || pointer == "LICENSE-APACHE"
        || (pointer.lines().count() == 1
            && pointer.contains("LICENSE")
            && (pointer.starts_with("../") || pointer.starts_with("..\\"))))
}

/// Write normalized UTF-8 text with LF endings, no BOM, and one trailing newline.
fn write_utf8_no_bom(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut normalized = content.replace("\r\n", "\n");
    if !normalized.ends_with('\n') {
        normalized.push('\n');
    }

    fs::write(path, normalized.as_bytes())?;
    Ok(())
}

/// Write extraction provenance into the generated output directory.
fn write_metadata(output_dir: &Path, zed_tag: &str, zed_dir: &Path) -> Result<()> {
    // Get commit SHA
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(zed_dir)
        .output()?;
    let sha = String::from_utf8_lossy(&output.stdout).trim().to_string();

    let metadata = serde_json::json!({
        "zed_tag": zed_tag,
        "zed_commit": sha,
        "transformed_at": chrono::Utc::now().to_rfc3339(),
        "crates": CRATE_PUBLISH_ORDER,
    });

    let path = output_dir.join("transform-metadata.json");
    fs::write(path, serde_json::to_string_pretty(&metadata)?)?;

    Ok(())
}
