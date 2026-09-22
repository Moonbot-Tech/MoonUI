use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use super::{
    component_matches_ignore_ascii_case, insert_subtree, normalize_lexically, path_within_subtree,
};

/// Catches `normalize_lexically` dropping a `..` pop or keeping a `.` component,
/// which would leave `foo/../bar` at `foo/bar` and open the wrong directory.
#[test]
fn normalize_lexically_collapses_dot_and_parent_inside_the_path() {
    assert_eq!(
        normalize_lexically(Path::new("foo/../bar")).unwrap(),
        PathBuf::from("bar")
    );
    assert_eq!(
        normalize_lexically(Path::new("a/./b/../c")).unwrap(),
        PathBuf::from("a/c")
    );
    assert_eq!(
        normalize_lexically(Path::new("/a/../b")).unwrap(),
        PathBuf::from("/b")
    );
}

/// Catches `normalize_lexically` accepting a `..` that climbs past the path root,
/// which would turn `foo/../../bar` into `bar` instead of refusing the path.
#[test]
fn normalize_lexically_rejects_parent_that_escapes_the_root() {
    let escaped = normalize_lexically(Path::new("foo/../../bar"));
    assert!(
        escaped.is_err(),
        "foo/../../bar climbed out of the path and must be rejected, got {escaped:?}"
    );
    assert_eq!(
        escaped.unwrap_err().to_string(),
        "parent reference `..` points outside of base directory"
    );
    let leading = normalize_lexically(Path::new(".."));
    assert!(
        leading.is_err(),
        "a leading .. has no parent to climb to, got {leading:?}"
    );
}

/// Catches `insert_subtree` leaving a descendant in place after a broader grant,
/// and dropping a sibling whose name only shares a string prefix (`proj-extra`
/// under a new `proj` grant).
#[test]
fn insert_subtree_prunes_descendants_and_keeps_prefix_siblings() {
    let mut subtrees = vec![
        PathBuf::from("proj/src"),
        PathBuf::from("proj/tests"),
        PathBuf::from("proj-extra"),
    ];
    insert_subtree(&mut subtrees, PathBuf::from("proj"));
    assert_eq!(
        subtrees,
        vec![PathBuf::from("proj-extra"), PathBuf::from("proj")]
    );
}

/// Catches `insert_subtree` appending a path an existing grant already covers,
/// which would leave a second copy that survives removal of the parent grant.
#[test]
fn insert_subtree_ignores_a_path_an_existing_grant_covers() {
    let mut subtrees = vec![PathBuf::from("proj"), PathBuf::from("other")];
    insert_subtree(&mut subtrees, PathBuf::from("proj/src"));
    assert_eq!(
        subtrees,
        vec![PathBuf::from("proj"), PathBuf::from("other")]
    );
}

/// Catches `path_within_subtree` treating a prefix-sibling (`proj2`) as inside a
/// `proj` grant, or rejecting the grant itself and its descendants.
#[test]
fn path_within_subtree_matches_the_grant_and_not_a_prefix_sibling() {
    let grants = [PathBuf::from("proj"), PathBuf::from("other/leaf")];

    assert!(
        path_within_subtree(Path::new("proj"), grants.iter().map(PathBuf::as_path)),
        "the grant path itself must count as inside the grant"
    );
    assert!(
        path_within_subtree(Path::new("proj/src"), grants.iter().map(PathBuf::as_path)),
        "a descendant of the grant must count as inside it"
    );
    assert!(
        path_within_subtree(Path::new("other/leaf"), grants.iter().map(PathBuf::as_path)),
        "an exact second grant must count as inside"
    );
    assert!(
        !path_within_subtree(Path::new("proj2"), grants.iter().map(PathBuf::as_path)),
        "proj2 only shares a string prefix with proj and must stay outside"
    );
    assert!(
        !path_within_subtree(Path::new("other"), grants.iter().map(PathBuf::as_path)),
        "a parent of a grant is not inside that grant"
    );
    assert!(
        !path_within_subtree(
            Path::new("other/leafier"),
            grants.iter().map(PathBuf::as_path)
        ),
        "other/leafier is a prefix-sibling of other/leaf and must stay outside"
    );
}

/// Catches `component_matches_ignore_ascii_case` comparing with `==`, so a
/// `.ZED` directory would miss the `.zed` settings classifier on Windows and macOS.
#[test]
fn component_matches_ignore_ascii_case_folds_ascii_case() {
    assert!(
        component_matches_ignore_ascii_case(OsStr::new(".ZED"), ".zed"),
        ".ZED must match .zed; ASCII case is not significant"
    );
    assert!(
        component_matches_ignore_ascii_case(OsStr::new(".Zed"), ".zed"),
        ".Zed must match .zed; ASCII case is not significant"
    );
    assert!(
        !component_matches_ignore_ascii_case(OsStr::new(".zed"), "zed"),
        "a missing leading dot is a different component"
    );
    assert!(
        !component_matches_ignore_ascii_case(OsStr::new("settings"), ".zed"),
        "an unrelated name must not match .zed"
    );
}
