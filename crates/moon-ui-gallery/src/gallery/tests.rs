use crate::{COMPONENT_COVERAGE, page_index, parse_theme_mode, theme_mode_name};
use moon_ui::ThemeMode;
use serde::Deserialize;

const COMPONENT_MANIFEST_JSON: &str =
    include_str!("../../../moon-ui-components/component-manifest.json");

#[derive(Deserialize)]
struct Manifest {
    components: Vec<ManifestComponent>,
}

#[derive(Deserialize)]
struct ManifestComponent {
    concept: String,
    public_path: Option<String>,
    escape_path: Option<String>,
}

#[test]
fn gallery_has_a_visual_coverage_manifest() {
    assert!(COMPONENT_COVERAGE.len() >= 30);
    assert!(COMPONENT_COVERAGE.contains(&"MoonButton"));
    assert!(COMPONENT_COVERAGE.contains(&"MoonDataTable"));
    assert!(COMPONENT_COVERAGE.contains(&"DockArea"));
    assert!(COMPONENT_COVERAGE.contains(&"MoonWindowFrame"));
}

#[test]
fn gallery_covers_every_public_manifest_component() {
    let manifest: Manifest =
        serde_json::from_str(COMPONENT_MANIFEST_JSON).expect("valid component manifest");
    for component in manifest.components {
        for path in [component.public_path, component.escape_path]
            .into_iter()
            .flatten()
        {
            let public_name = path.rsplit("::").next().unwrap_or(&path);
            assert!(
                COMPONENT_COVERAGE.contains(&public_name),
                "gallery coverage is missing manifest component {} ({})",
                component.concept,
                path
            );
        }
    }
}

#[test]
fn gallery_page_cli_names_match_tabs() {
    assert_eq!(page_index("Controls"), Some(0));
    assert_eq!(page_index("inputs"), Some(1));
    assert_eq!(page_index("Layout"), Some(4));
    assert_eq!(page_index("NewControls"), Some(5));
    assert_eq!(page_index("Composites"), Some(6));
    assert_eq!(page_index("Stateful"), Some(7));
    assert_eq!(page_index("missing"), None);
}

#[test]
fn gallery_theme_cli_names_match_modes() {
    assert_eq!(parse_theme_mode("dark"), Some(ThemeMode::Dark));
    assert_eq!(parse_theme_mode("Light"), Some(ThemeMode::Light));
    assert_eq!(parse_theme_mode("system"), Some(ThemeMode::System));
    assert_eq!(parse_theme_mode("missing"), None);
    assert_eq!(theme_mode_name(ThemeMode::Dark), "Dark");
    assert_eq!(theme_mode_name(ThemeMode::Light), "Light");
    assert_eq!(theme_mode_name(ThemeMode::System), "System");
}
