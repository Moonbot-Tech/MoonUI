use super::rel_path;
use crate::paths::PathStyle;

/// Catches `RelPath::ends_with` accepting any string tail, so `foo/bar` would
/// count as ending in `ar` and a glob would hide a different file.
#[test]
fn ends_with_rejects_a_partial_component() {
    assert!(
        !rel_path("foo/bar").ends_with(rel_path("ar")),
        "ar is only the tail of the bar component and must not match"
    );
    assert!(
        !rel_path("a/bc").ends_with(rel_path("c")),
        "c is only the tail of bc; a component boundary is required"
    );
    assert!(
        !rel_path("proj-extra").ends_with(rel_path("extra")),
        "extra shares a string suffix with proj-extra but is not its own component"
    );
}

/// Catches `RelPath::ends_with` dropping the exact-match arm, so a path would
/// no longer match itself and a glob of that full relative path would miss it.
#[test]
fn ends_with_matches_the_path_itself() {
    assert!(
        rel_path("foo/bar").ends_with(rel_path("foo/bar")),
        "a path is a suffix of itself"
    );
    assert!(
        rel_path("readme").ends_with(rel_path("readme")),
        "a single component is a suffix of itself"
    );
}

/// Catches `RelPath::ends_with` ignoring the slash that separates components,
/// so `foo/bar/baz` would miss a glob that asks for `bar/baz`.
#[test]
fn ends_with_matches_a_whole_component_suffix() {
    assert!(
        rel_path("foo/bar/baz").ends_with(rel_path("bar/baz")),
        "bar/baz is a component suffix of foo/bar/baz"
    );
    assert!(
        rel_path("foo/bar/baz").ends_with(rel_path("baz")),
        "the final component is a suffix"
    );
    assert!(
        !rel_path("foo/bar").ends_with(rel_path("foo")),
        "a parent directory is not a suffix of its child"
    );
    assert!(
        !rel_path("foo/bar").ends_with(rel_path("foo/bar/baz")),
        "a longer path is not a suffix of a shorter one"
    );
}

/// Catches `RelPath::display` rewriting separators for POSIX, so a path shown
/// on a POSIX host would appear with backslashes.
#[test]
fn display_posix_keeps_internal_slashes() {
    assert_eq!(
        rel_path("foo/bar/baz").display(PathStyle::Posix),
        "foo/bar/baz"
    );
    assert_eq!(rel_path("readme").display(PathStyle::Posix), "readme");
}

/// Catches `RelPath::display` leaving `/` in place for Windows, so a path
/// shown to a Windows user would stay `foo/bar` instead of `foo\bar`.
#[test]
fn display_windows_rewrites_slashes_to_backslashes() {
    assert_eq!(
        rel_path("foo/bar/baz").display(PathStyle::Windows),
        "foo\\bar\\baz"
    );
}

/// Catches `RelPath::last_n_components` skipping one extra component, so a
/// request for the last two components of `foo/bar/baz` would return only `baz`
/// and a path suffix would name the wrong file.
#[test]
fn last_n_components_returns_the_trailing_suffix() {
    assert_eq!(
        rel_path("foo/bar/baz").last_n_components(2),
        Some(rel_path("bar/baz")),
        "the last two components are bar/baz"
    );
    assert_eq!(
        rel_path("foo/bar/baz").last_n_components(1),
        Some(rel_path("baz")),
        "the last component is baz"
    );
}

/// Catches `RelPath::last_n_components` treating an exact component count as
/// past the end, so asking for every component of a path would return nothing
/// and a full-path suffix check would miss the file.
#[test]
fn last_n_components_returns_the_whole_path_when_the_count_matches() {
    assert_eq!(
        rel_path("foo/bar/baz").last_n_components(3),
        Some(rel_path("foo/bar/baz")),
        "a count equal to the path length is the path itself"
    );
    assert_eq!(
        rel_path("foo").last_n_components(1),
        Some(rel_path("foo")),
        "a single component asked for in full is that component"
    );
}

/// Catches `RelPath::last_n_components` returning the original path when the
/// caller asks for more components than the path has, so a too-long suffix
/// would still match.
#[test]
fn last_n_components_is_none_when_the_count_exceeds_the_path() {
    assert!(
        rel_path("foo/bar/baz").last_n_components(4).is_none(),
        "four components do not fit in a three-component path"
    );
    assert!(
        rel_path("foo").last_n_components(2).is_none(),
        "two components do not fit in a single-component path"
    );
}

/// Catches `RelPath::join` gluing two paths into one component, so `foo` joined
/// with `bar` would become `foobar` and a later lookup would miss `foo/bar`.
#[test]
fn join_inserts_a_slash_between_two_non_empty_paths() {
    assert_eq!(
        rel_path("foo").join(rel_path("bar")).as_unix_str(),
        "foo/bar"
    );
    assert_eq!(
        rel_path("foo/bar").join(rel_path("baz/qux")).as_unix_str(),
        "foo/bar/baz/qux"
    );
}

/// Catches `RelPath::join` inserting a slash around an empty side, so joining
/// an empty directory with `foo/bar` would produce `/foo/bar` and leave the
/// relative-path guarantee.
#[test]
fn join_keeps_the_non_empty_side_when_the_other_is_empty() {
    assert_eq!(
        super::RelPath::empty()
            .join(rel_path("foo/bar"))
            .as_unix_str(),
        "foo/bar",
        "joining onto an empty path is the other path"
    );
    assert_eq!(
        rel_path("foo/bar")
            .join(super::RelPath::empty())
            .as_unix_str(),
        "foo/bar",
        "joining an empty path leaves the receiver unchanged"
    );
    assert_eq!(
        super::RelPath::empty()
            .join(super::RelPath::empty())
            .as_unix_str(),
        "",
        "joining two empty paths stays empty"
    );
}

/// Catches `RelPathBuf::set_extension` dropping the slash that `pop` removes,
/// so `foo/bar.rs` would become `foobar.txt` and the renamed file would sit
/// beside its parent instead of inside it.
#[test]
fn set_extension_keeps_the_parent_separator() {
    let mut nested = super::rel_path_buf("foo/bar.rs");
    assert!(
        nested.set_extension("txt"),
        "foo/bar.rs has a file name whose extension can change"
    );
    assert_eq!(
        nested.as_unix_str(),
        "foo/bar.txt",
        "the parent separator must stay between foo and bar.txt"
    );

    let mut flat = super::rel_path_buf("bar.rs");
    assert!(
        flat.set_extension("txt"),
        "bar.rs has a file name whose extension can change"
    );
    assert_eq!(
        flat.as_unix_str(),
        "bar.txt",
        "a path with no slash still replaces only the extension"
    );
}

/// Catches `RelPath::from_proto` accepting a path that still contains `..`, `.`,
/// or an absolute prefix, and rejecting a path that is already a normalized
/// relative path. A wire value could then escape the relative-path guarantee,
/// or a stored `foo/bar` could no longer be loaded.
#[test]
fn from_proto_accepts_only_an_already_normalized_relative_path() {
    let path =
        super::RelPath::from_proto("foo/bar").expect("foo/bar is already a relative wire path");
    assert_eq!(path.as_unix_str(), "foo/bar");

    let single = super::RelPath::from_proto("readme").expect("a single component is a wire path");
    assert_eq!(single.as_unix_str(), "readme");

    assert!(
        super::RelPath::from_proto("foo/../bar").is_err(),
        "a parent component is not a wire path"
    );
    assert!(
        super::RelPath::from_proto("foo/./bar").is_err(),
        "a dot component is not a wire path"
    );
    assert!(
        super::RelPath::from_proto("/foo").is_err(),
        "an absolute path is not a relative wire path"
    );
}
