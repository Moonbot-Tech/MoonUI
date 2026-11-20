use std::path::Path;

use toml_edit::{DocumentMut, Item};

use super::{
    WorkspaceDependencies, remove_dep_from_features, resolve_workspace_dep, transform_cargo_toml,
    transform_dependencies,
};

/// Catches manifest rewrites that stop aliasing internal crates or leave removed git-only optionals in features.
#[test]
fn dependency_policy_preserves_internal_aliases_and_cleans_removed_optionals() {
    for use_local_deps in [true, false] {
        let mut doc: DocumentMut = r#"
[dependencies]
collections = { workspace = true, features = ["test-support", "inspector"], optional = true }
git_optional = { workspace = true, optional = true }

[features]
bundle = ["collections/test-support", "git_optional", "git_optional/extra"]
"#
        .parse()
        .expect("parse dependency fixture");
        let workspace_doc: DocumentMut = r#"
[workspace.dependencies]
git_optional = { git = "https://example.invalid/git-optional" }
"#
        .parse()
        .expect("parse workspace fixture");
        let workspace_table = workspace_doc
            .get("workspace")
            .and_then(|workspace| workspace.get("dependencies"))
            .and_then(Item::as_table_like)
            .expect("workspace dependency table");
        let workspace_deps = workspace_table
            .iter()
            .map(|(name, value)| (name.to_string(), value.clone()))
            .collect::<WorkspaceDependencies>();
        let mut removed_optionals = Vec::new();

        transform_dependencies(
            &mut doc,
            "dependencies",
            &workspace_deps,
            "0.185.0",
            Path::new("unused-output"),
            use_local_deps,
            &mut removed_optionals,
        )
        .expect("transform dependency fixture");
        for dep_name in &removed_optionals {
            remove_dep_from_features(&mut doc, dep_name);
        }

        let dependencies = doc
            .get("dependencies")
            .and_then(Item::as_table_like)
            .expect("dependencies table");
        let collections = dependencies
            .get("collections")
            .and_then(Item::as_table_like)
            .expect("rewritten internal dependency");
        assert_eq!(
            collections.get("package").and_then(Item::as_str),
            Some("moon-collections")
        );
        assert_eq!(
            collections.get("optional").and_then(Item::as_bool),
            Some(true)
        );
        let features = collections
            .get("features")
            .and_then(Item::as_array)
            .expect("preserved features")
            .iter()
            .filter_map(|value| value.as_str())
            .collect::<Vec<_>>();
        assert_eq!(features, vec!["test-support"]);
        if use_local_deps {
            assert_eq!(
                collections.get("path").and_then(Item::as_str),
                Some("../moon-collections")
            );
            assert!(!collections.contains_key("version"));
        } else {
            assert_eq!(
                collections.get("version").and_then(Item::as_str),
                Some("0.185.0")
            );
            assert!(!collections.contains_key("path"));
        }
        assert!(!dependencies.contains_key("git_optional"));
        assert_eq!(removed_optionals, vec!["git_optional"]);

        let bundle = doc
            .get("features")
            .and_then(Item::as_table_like)
            .and_then(|features| features.get("bundle"))
            .and_then(Item::as_array)
            .expect("bundle feature")
            .iter()
            .filter_map(|value| value.as_str())
            .collect::<Vec<_>>();
        assert_eq!(bundle, vec!["collections/test-support"]);
    }
}

/// Parse one `name = <value>` line and return its value item.
fn dependency_item(line: &str) -> Item {
    let doc: DocumentMut = line.parse().expect("parse dependency line");
    doc.get("dep").expect("dependency value").clone()
}

/// Return the string entries of a resolved dependency's `features` array.
fn resolved_features(resolved: &Item) -> Vec<String> {
    resolved
        .as_table_like()
        .and_then(|table| table.get("features"))
        .and_then(Item::as_array)
        .expect("resolved features")
        .iter()
        .filter_map(|value| value.as_str().map(str::to_string))
        .collect()
}

/// Catches `resolve_workspace_dep` replacing the workspace definition's features with the
/// member's own list. Cargo's reference ("Inheriting a dependency from a workspace") states that
/// `features` on a `workspace = true` dependency "are additive with the features from the
/// workspace dependency", so `serde = { version = "1", features = ["derive"] }` in the workspace
/// and `serde = { workspace = true, features = ["rc"] }` in the member build serde with both
/// `derive` and `rc`. Dropping `derive` makes the standalone crate stop compiling its
/// `#[derive(Serialize)]` uses.
#[test]
fn member_features_add_to_the_workspace_features() {
    let workspace = dependency_item(r#"dep = { version = "1", features = ["derive"] }"#);
    let usage = dependency_item(r#"dep = { workspace = true, features = ["rc"] }"#);

    let resolved = resolve_workspace_dep(&workspace, &usage)
        .expect("resolve dependency")
        .expect("versioned dependency stays");

    assert_eq!(resolved_features(&resolved), vec!["derive", "rc"]);
}

const CHARACTERIZATION_MANIFEST: &str = r#"[package]
name = "gpui"
version = "0.1.0"
edition.workspace = true
publish.workspace = true

[features]
default = []
inspector = ["gpui_macros/inspector"]
test-support = ["collections/test-support"]

[dependencies]
collections = { workspace = true, features = ["inspector"] }
gpui_macros = { workspace = true, features = ["inspector"] }
serde = { workspace = true }
pinned = { workspace = true, optional = true }
ztracing.workspace = true

[dev-dependencies]
git_only = { workspace = true }
zlog.workspace = true
ctor.workspace = true

[build-dependencies]
cc = { workspace = true }

[target.'cfg(windows)'.dependencies]
util = { workspace = true }
windows = { workspace = true }

[target.'cfg(unix)'.dev-dependencies]
sum_tree = { workspace = true }

[lints]
workspace = true
"#;

const CHARACTERIZATION_WORKSPACE: &str = r#"[workspace.dependencies]
serde = { version = "1.0", default-features = false, features = ["derive"] }
pinned = { git = "https://example.invalid/pinned", version = "2.1", package = "pinned-fork" }
git_only = { git = "https://example.invalid/git-only" }
cc = "1.2"
windows = { version = "0.61", features = ["Win32_Foundation"] }
"#;

/// Rewrite the characterization fixture as `original_name` and return the written manifest.
fn transform_characterization_fixture(original_name: &str, use_local_deps: bool) -> String {
    let crate_dir = tempfile::tempdir().expect("crate tempdir");
    let cargo_toml = crate_dir.path().join("Cargo.toml");
    std::fs::write(&cargo_toml, CHARACTERIZATION_MANIFEST).expect("write fixture manifest");
    let workspace_doc: DocumentMut = CHARACTERIZATION_WORKSPACE
        .parse()
        .expect("parse workspace fixture");
    let workspace_deps = workspace_doc
        .get("workspace")
        .and_then(|workspace| workspace.get("dependencies"))
        .and_then(Item::as_table_like)
        .expect("workspace dependency table")
        .iter()
        .map(|(name, value)| (name.to_string(), value.clone()))
        .collect::<WorkspaceDependencies>();

    transform_cargo_toml(
        crate_dir.path(),
        Path::new("unused-output"),
        original_name,
        &workspace_deps,
        "v0.185.0",
        use_local_deps,
    )
    .expect("transform fixture manifest");
    std::fs::read_to_string(&cargo_toml).expect("read rewritten manifest")
}

const EXPECTED_GPUI_LOCAL: &str = r#"[package]
name = "moon-gpui"
version = "0.185.0"
edition = "2024"
publish = false
repository = "https://github.com/Moonbot-Tech/MoonUI"
license = "Apache-2.0"
description = "Moon standalone build of Zed's gpui crate"

[features]
default = []
test-support = ["collections/test-support", "dep:proptest"]
proptest = ["dep:proptest"]

[dependencies]
collections = { package = "moon-collections", path = "../moon-collections" }
gpui_macros = { package = "moon-gpui-macros", path = "../moon-gpui-macros" }
serde = { version = "1.0", default-features = false, features = ["derive"] }
pinned = { version = "2.1", package = "pinned-fork", optional = true }
ztracing.workspace = true
proptest = { version = "1", optional = true, features = ["attr-macro"] }

[dev-dependencies]
zlog.workspace = true
ctor.workspace = true
gpui_platform = { package = "moon-gpui-platform", path = "../moon-gpui-platform" }
proptest = { version = "1", features = ["attr-macro"] }

[build-dependencies]
cc = { version = "1.2" }

[target.'cfg(windows)'.dependencies]
util = { package = "moon-util", path = "../moon-util" }
windows = { version = "0.61", features = ["Win32_Foundation"] }

[target.'cfg(unix)'.dev-dependencies]
sum_tree = { package = "moon-sum-tree", path = "../moon-sum-tree" }

[lib]
name = "gpui"
path = "src/lib.rs"

[lints]
rust = { unexpected_cfgs = { level = "warn", check-cfg = ["cfg(rust_analyzer)"] } }
"#;

const EXPECTED_GPUI_VERSIONED: &str = r#"[package]
name = "moon-gpui"
version = "0.185.0"
edition = "2024"
publish = false
repository = "https://github.com/Moonbot-Tech/MoonUI"
license = "Apache-2.0"
description = "Moon standalone build of Zed's gpui crate"

[features]
default = []
test-support = ["collections/test-support", "dep:proptest"]
proptest = ["dep:proptest"]

[dependencies]
collections = { package = "moon-collections", version = "0.185.0" }
gpui_macros = { package = "moon-gpui-macros", version = "0.185.0" }
serde = { version = "1.0", default-features = false, features = ["derive"] }
pinned = { version = "2.1", package = "pinned-fork", optional = true }
ztracing.workspace = true
proptest = { version = "1", optional = true, features = ["attr-macro"] }

[dev-dependencies]
zlog.workspace = true
ctor.workspace = true
gpui_platform = { package = "moon-gpui-platform", version = "0.185.0" }
proptest = { version = "1", features = ["attr-macro"] }

[build-dependencies]
cc = { version = "1.2" }

[target.'cfg(windows)'.dependencies]
util = { package = "moon-util", version = "0.185.0" }
windows = { version = "0.61", features = ["Win32_Foundation"] }

[target.'cfg(unix)'.dev-dependencies]
sum_tree = { package = "moon-sum-tree", version = "0.185.0" }

[lib]
name = "gpui"
path = "src/lib.rs"

[lints]
rust = { unexpected_cfgs = { level = "warn", check-cfg = ["cfg(rust_analyzer)"] } }
"#;

const EXPECTED_SUM_TREE_LOCAL: &str = r#"[package]
name = "moon-sum-tree"
version = "0.185.0"
edition = "2024"
publish = false
repository = "https://github.com/Moonbot-Tech/MoonUI"
license = "Apache-2.0"
description = "Moon standalone build of Zed's sum_tree crate"

[features]
default = []
inspector = ["gpui_macros/inspector"]
test-support = ["collections/test-support", "dep:proptest"]
proptest = ["dep:proptest"]

[dependencies]
collections = { package = "moon-collections", path = "../moon-collections" }
gpui_macros = { package = "moon-gpui-macros", path = "../moon-gpui-macros" }
serde = { version = "1.0", default-features = false, features = ["derive"] }
pinned = { version = "2.1", package = "pinned-fork", optional = true }
proptest = { version = "1", optional = true, features = ["attr-macro"] }

[dev-dependencies]
proptest = { version = "1", features = ["attr-macro"] }

[build-dependencies]
cc = { version = "1.2" }

[target.'cfg(windows)'.dependencies]
util = { package = "moon-util", path = "../moon-util" }
windows = { version = "0.61", features = ["Win32_Foundation"] }

[target.'cfg(unix)'.dev-dependencies]
sum_tree = { package = "moon-sum-tree", path = "../moon-sum-tree" }
"#;

const EXPECTED_COLLECTIONS_VERSIONED: &str = r#"[package]
name = "moon-collections"
version = "0.185.0"
edition = "2024"
publish = false
repository = "https://github.com/Moonbot-Tech/MoonUI"
license = "Apache-2.0"
description = "Moon standalone build of Zed's collections crate"

[features]
default = []
inspector = ["gpui_macros/inspector"]
test-support = ["collections/test-support"]

[dependencies]
collections = { package = "moon-collections", version = "0.185.0" }
gpui_macros = { package = "moon-gpui-macros", version = "0.185.0" }
serde = { version = "1.0", default-features = false, features = ["derive"] }
pinned = { version = "2.1", package = "pinned-fork", optional = true }
ztracing.workspace = true

[dev-dependencies]
zlog.workspace = true
ctor.workspace = true

[build-dependencies]
cc = { version = "1.2" }

[target.'cfg(windows)'.dependencies]
util = { package = "moon-util", version = "0.185.0" }
windows = { version = "0.61", features = ["Win32_Foundation"] }

[target.'cfg(unix)'.dev-dependencies]
sum_tree = { package = "moon-sum-tree", version = "0.185.0" }
"#;

/// Catches a restructuring of `transform_cargo_toml` that drops, reorders or
/// double-applies one of its rewrites: the package metadata, the gpui `[lib]`
/// alias and `gpui_platform` dev alias, the sum_tree GPL-dependency removals,
/// the top-level and per-target dependency rewrites, the inspector and
/// proptest feature wiring, or the custom cfg lints. Any of those would emit a
/// different generated manifest for an extracted crate.
#[test]
fn transform_cargo_toml_rewrites_the_whole_manifest() {
    for (original_name, use_local_deps, expected) in [
        ("gpui", true, EXPECTED_GPUI_LOCAL),
        ("gpui", false, EXPECTED_GPUI_VERSIONED),
        ("sum_tree", true, EXPECTED_SUM_TREE_LOCAL),
        ("collections", false, EXPECTED_COLLECTIONS_VERSIONED),
    ] {
        assert_eq!(
            transform_characterization_fixture(original_name, use_local_deps),
            expected,
            "{original_name} (use_local_deps = {use_local_deps})"
        );
    }
}
