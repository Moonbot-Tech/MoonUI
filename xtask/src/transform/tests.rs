use super::{crate_name_from_path, is_internal_crate, moon_package_name};

/// Catches `moon_package_name` leaving underscores in a Zed crate name, which
/// would publish `http_client` as `moon-http_client` and miss the renamed crate.
#[test]
fn moon_package_name_turns_underscores_into_hyphens() {
    assert_eq!(moon_package_name("http_client"), "moon-http-client");
    assert_eq!(moon_package_name("util_macros"), "moon-util-macros");
}

/// Catches widening the exact `gpui` rename to every name that starts with
/// `gpui`, which would publish `gpui_macros` as `moon-gpui` and collide with
/// the core crate.
#[test]
fn moon_package_name_does_not_collapse_gpui_siblings() {
    assert_eq!(moon_package_name("gpui_macros"), "moon-gpui-macros");
}

/// Catches `crate_name_from_path` keeping the parent of a nested extraction
/// entry, which would write a workspace member for `refineable` instead of
/// `derive_refineable`.
#[test]
fn crate_name_from_path_uses_the_leaf_of_a_nested_entry() {
    assert_eq!(
        crate_name_from_path("refineable/derive_refineable"),
        "derive_refineable"
    );
    assert_eq!(crate_name_from_path("tooling/perf"), "perf");
}

/// Catches `is_internal_crate` comparing a dependency to the full extraction
/// path, which would leave `derive_refineable` and `perf` as external
/// dependencies instead of rewriting them to Moon packages.
#[test]
fn is_internal_crate_matches_the_leaf_of_a_nested_entry() {
    assert!(is_internal_crate("derive_refineable"));
    assert!(is_internal_crate("perf"));
    assert!(!is_internal_crate("serde"));
}

/// Catches `is_internal_crate` treating a parent directory as a crate, which
/// would rewrite a dependency named `tooling` even though only `perf` is
/// extracted from that directory.
#[test]
fn is_internal_crate_rejects_a_directory_prefix() {
    assert!(!is_internal_crate("tooling"));
    assert!(is_internal_crate("refineable"));
}
