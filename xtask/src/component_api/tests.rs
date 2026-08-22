//! Regression tests for component API snapshot comparison and path normalization.

use super::{
    ApiItem, ApiSnapshot, collect_signature, compare_api, is_private_child_reexport,
    logical_api_file,
};
use std::{collections::BTreeSet, path::Path};

/// Build one API snapshot item in the canonical button facade path.
fn item(signature: &str) -> ApiItem {
    ApiItem {
        file: "crates/moon-ui-components/src/moon/button.rs".to_string(),
        signature: signature.to_string(),
    }
}

/// Build a version-one snapshot from the supplied public signatures.
fn snapshot(signatures: &[&str]) -> ApiSnapshot {
    ApiSnapshot {
        version: 1,
        items: signatures.iter().map(|signature| item(signature)).collect(),
    }
}

/// Catches letting an addition pass unrecorded, which leaves it unguarded against removal.
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

/// Catches losing the removal half while adding the addition half of the API comparison.
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

/// Catches truncating a multiline re-export before the terminating semicolon.
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

/// Catches treating a private sibling extraction as a public API file move.
#[test]
fn nested_component_items_keep_the_owning_api_file() {
    let root = Path::new("C:/MoonUI");
    let nested = root.join("crates/moon-ui-components/src/moon/dropdown/model.rs");

    assert_eq!(
        logical_api_file(root, &nested),
        root.join("crates/moon-ui-components/src/moon/dropdown.rs")
    );
}

/// Catches filtering external re-exports while retaining local implementation-only re-exports.
#[test]
fn only_private_child_reexports_are_structural() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask must live below the workspace root");
    let dropdown = workspace.join("crates/moon-ui-components/src/moon/dropdown.rs");
    assert!(is_private_child_reexport(
        &dropdown,
        "pub use model::{MoonMenuItem, MoonMenuSize};"
    ));
    assert!(!is_private_child_reexport(
        &dropdown,
        "pub use crate::scroll::ScrollableElement;"
    ));
    assert!(!is_private_child_reexport(
        &dropdown,
        "pub use super::foundation::StyledExt;"
    ));
}
