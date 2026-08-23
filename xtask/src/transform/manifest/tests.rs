use std::path::Path;

use toml_edit::{DocumentMut, Item};

use super::{WorkspaceDependencies, remove_dep_from_features, transform_dependencies};

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
