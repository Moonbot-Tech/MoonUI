//! Exact source rewrites required by the extracted GPUI crates.

use anyhow::Result;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

#[cfg(test)]
mod tests;

/// Patch source files to remove inspector feature references.
/// Replaces various inspector cfg patterns with simpler versions.
pub(super) fn patch_inspector_cfgs(crate_dir: &Path) -> Result<()> {
    let src_dir = crate_dir.join("src");
    if !src_dir.exists() {
        return Ok(());
    }

    for entry in WalkDir::new(&src_dir) {
        let entry = entry?;
        if entry.file_type().is_file() && entry.path().extension().is_some_and(|e| e == "rs") {
            let content = fs::read_to_string(entry.path())?;

            // Replace various inspector cfg patterns
            let patched = content
                // Simple case: #[cfg(any(feature = "inspector", debug_assertions))]
                .replace(
                    "#[cfg(any(feature = \"inspector\", debug_assertions))]",
                    "#[cfg(debug_assertions)]",
                )
                // Negated case: #[cfg(not(any(feature = "inspector", debug_assertions)))]
                .replace(
                    "#[cfg(not(any(feature = \"inspector\", debug_assertions)))]",
                    "#[cfg(not(debug_assertions))]",
                )
                // Complex case with rust_analyzer: all(any(feature = "inspector", debug_assertions), not(rust_analyzer))
                .replace(
                    "all(any(feature = \"inspector\", debug_assertions), not(rust_analyzer))",
                    "all(debug_assertions, not(rust_analyzer))",
                );

            if patched != content {
                fs::write(entry.path(), patched)?;
            }
        }
    }

    Ok(())
}

/// Patch gpui_macos source to fix unnecessary unsafe block.
/// NSBeep() is now safe in newer objc bindings.
pub(super) fn patch_gpui_macos_source(crate_dir: &Path) -> Result<()> {
    let window_rs = crate_dir.join("src/window.rs");
    if !window_rs.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(&window_rs)?;

    // Remove unnecessary unsafe around NSBeep()
    let patched = content.replace("unsafe { NSBeep() }", "NSBeep()");

    if patched != content {
        fs::write(&window_rs, patched)?;
    }

    Ok(())
}

/// Remove GPL zlog/ztracing instrumentation from sum_tree during extraction.
pub(super) fn patch_sum_tree_tracing(crate_dir: &Path) -> Result<()> {
    for rel_path in ["src/sum_tree.rs", "src/cursor.rs"] {
        let path = crate_dir.join(rel_path);
        if !path.exists() {
            continue;
        }

        let content = fs::read_to_string(&path)?;
        let patched = content
            .replace("use ztracing::instrument;\n", "")
            .replace("use ztracing::instrument;\r\n", "")
            .replace("    #[instrument(skip_all)]\n", "")
            .replace("    #[instrument(skip_all)]\r\n", "")
            .replace(
                "\n    #[ctor::ctor(unsafe)]\n    fn init_logger() {\n        zlog::init_test();\n    }\n",
                "\n",
            )
            .replace(
                "\r\n    #[ctor::ctor(unsafe)]\r\n    fn init_logger() {\r\n        zlog::init_test();\r\n    }\r\n",
                "\r\n",
            )
            .replace("\n\n\n    #[test]", "\n\n    #[test]")
            .replace("\r\n\r\n\r\n    #[test]", "\r\n\r\n    #[test]");

        if patched != content {
            fs::write(&path, patched)?;
        }
    }

    Ok(())
}

/// Patch text.rs example to remove external font dependency.
/// The example uses include_bytes! for a font file outside the crate.
pub(super) fn patch_text_example(crate_dir: &Path) -> Result<()> {
    let text_rs = crate_dir.join("examples/text.rs");
    if !text_rs.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(&text_rs)?;
    // Normalize line endings for cross-platform compatibility
    let content = content.replace("\r\n", "\n");

    // Remove the Cow import (no longer needed without include_bytes)
    let patched = content.replace("    borrow::Cow,\n", "");

    // Remove the font loading block
    let patched = patched.replace(
        r#"let fonts = [include_bytes!(
            "../../../assets/fonts/geist-mono/GeistMono-Regular.ttf"
        )]
        .iter()
        .map(|b| Cow::Borrowed(&b[..]))
        .collect();

        _ = cx.text_system().add_fonts(fonts);

        "#,
        "",
    );

    if patched != content {
        fs::write(&text_rs, patched)?;
        println!("  Patched examples/text.rs (removed external font dependency)");
    }

    Ok(())
}
