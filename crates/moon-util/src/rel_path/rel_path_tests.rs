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
