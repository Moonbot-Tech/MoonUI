use super::GalleryStyle;
use crate::{COMPONENT_COVERAGE, GalleryTheme, page_index, parse_gallery_theme, parse_theme_mode};
use moon_ui::{MoonColors, MoonPalette, MoonSkin, MoonThemeConfig, ThemeMode};
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
    assert_eq!(page_index("checkboxes"), Some(8));
    assert_eq!(page_index("Toggles"), Some(9));
    assert_eq!(page_index("missing"), None);
}

#[test]
fn gallery_theme_cli_names_match_modes() {
    assert_eq!(parse_theme_mode("dark"), Some(ThemeMode::Dark));
    assert_eq!(parse_theme_mode("Light"), Some(ThemeMode::Light));
    assert_eq!(parse_theme_mode("system"), Some(ThemeMode::System));
    assert_eq!(parse_theme_mode("missing"), None);
    // The snapshot writer must not let a second dark/light skin overwrite the original baseline.
    for name in ["dark", "light", "system", "obsidian", "light-facet"] {
        assert_eq!(parse_gallery_theme(name).unwrap().snapshot_name(), name);
    }
}

/// Catches `gallery.rs:GalleryStyle::config` leaving the previous style behind: choosing Dark after
/// Graphite would keep the graphite surfaces under a "Dark" label, and choosing a legacy style
/// after a colour mode would keep painting the mode's roles and derived palette.
#[test]
fn switching_styles_installs_each_styles_own_palette_and_mode() {
    let graphite = GalleryStyle::Graphite.config(MoonThemeConfig::moon_terminal());
    assert_eq!(graphite.mode, ThemeMode::Dark);
    assert_eq!(graphite.dark.palette, MoonPalette::GRAPHITE);

    let dark_new = GalleryStyle::DarkNew.config(graphite);
    assert_eq!(dark_new.mode, ThemeMode::Dark);
    assert_eq!(dark_new.dark.colors, Some(MoonColors::DARK));
    assert_eq!(dark_new.dark.palette, MoonColors::DARK.to_palette());

    let light_new = GalleryStyle::LightNew.config(dark_new);
    assert_eq!(light_new.mode, ThemeMode::Light);
    assert_eq!(light_new.light.colors, Some(MoonColors::LIGHT));
    assert_eq!(light_new.light.palette, MoonColors::LIGHT.to_palette());

    let light = GalleryStyle::Light.config(light_new);
    assert_eq!(light.mode, ThemeMode::Light);
    assert_eq!(light.light.palette, MoonPalette::LIGHT);
    assert_eq!(light.light.colors, None);

    let dark = GalleryStyle::Dark.config(light);
    assert_eq!(dark.mode, ThemeMode::Dark);
    assert_eq!(dark.dark.palette, MoonPalette::TERMINAL);
    assert_eq!(dark.dark.colors, None);
}

/// Catches `main.rs:parse_gallery_theme` no longer recognising `obsidian`, which would open the
/// dark terminal theme under that name, or swallowing a mode name, which would change what
/// `--theme dark` and `--theme light` open.
#[test]
fn gallery_theme_cli_names_select_obsidian_or_a_mode() {
    assert_eq!(
        parse_gallery_theme("obsidian"),
        Some(GalleryTheme::Obsidian)
    );
    assert_eq!(
        parse_gallery_theme("Obsidian"),
        Some(GalleryTheme::Obsidian)
    );
    assert_eq!(
        parse_gallery_theme("dark"),
        Some(GalleryTheme::Mode(ThemeMode::Dark))
    );
    assert_eq!(
        parse_gallery_theme("Light"),
        Some(GalleryTheme::Mode(ThemeMode::Light))
    );
    assert_eq!(
        parse_gallery_theme("system"),
        Some(GalleryTheme::Mode(ThemeMode::System))
    );
    assert_eq!(parse_gallery_theme("missing"), None);
}

/// Catches `main.rs:GalleryTheme::config` installing the wrong bundled theme: `dark` and
/// `light` must stay the flat terminal theme in that mode, and only `obsidian` may bring the
/// Obsidian palette and the Facet skin, with the header's style menu opening on the same name.
#[test]
fn launch_themes_install_the_terminal_theme_by_mode_and_obsidian_by_name() {
    let dark = GalleryTheme::Mode(ThemeMode::Dark).config();
    assert_eq!(dark.mode, ThemeMode::Dark);
    assert_eq!(dark.dark.palette, MoonPalette::TERMINAL);
    assert_eq!(dark.dark.skin, MoonSkin::Flat);

    let light = GalleryTheme::Mode(ThemeMode::Light).config();
    assert_eq!(light.mode, ThemeMode::Light);
    assert_eq!(light.light.palette, MoonPalette::LIGHT);
    assert_eq!(light.light.skin, MoonSkin::Flat);

    let obsidian = GalleryTheme::Obsidian.config();
    assert_eq!(obsidian.mode, ThemeMode::Dark);
    assert_eq!(obsidian.dark.palette, MoonPalette::OBSIDIAN);
    assert_eq!(obsidian.dark.skin, MoonSkin::Facet);
    assert_eq!(GalleryTheme::Obsidian.config().mode, ThemeMode::Dark);

    for (theme, style) in [
        (GalleryTheme::Obsidian, GalleryStyle::Obsidian),
        (GalleryTheme::Mode(ThemeMode::Dark), GalleryStyle::Dark),
        (GalleryTheme::Mode(ThemeMode::System), GalleryStyle::Dark),
        (GalleryTheme::Mode(ThemeMode::Light), GalleryStyle::Light),
        (GalleryTheme::LightFacet, GalleryStyle::LightFacet),
    ] {
        assert_eq!(GalleryStyle::for_theme(theme), style);
    }
}

/// Catches the light faceted style losing its skin or its side: `--theme light-facet` and the
/// "Light (facet)" menu row exist to show the Facet rules on a light palette, so each must
/// install the light palette, in light mode, with the Facet skin on that side only.
#[test]
fn the_light_facet_style_draws_the_light_palette_with_the_facet_skin() {
    let launched = GalleryTheme::LightFacet.config();
    let switched = GalleryStyle::LightFacet.config(MoonThemeConfig::moon_obsidian());
    for config in [launched, switched.clone()] {
        assert_eq!(config.mode, ThemeMode::Light);
        assert_eq!(config.light.palette, MoonPalette::LIGHT);
        assert_eq!(config.light.skin, MoonSkin::Facet);
        assert_eq!(config.dark.skin, MoonSkin::Flat);
    }
    assert_eq!(GalleryTheme::LightFacet.config().mode, ThemeMode::Light);

    let back = GalleryStyle::Light.config(switched);
    assert_eq!(back.light.skin, MoonSkin::Flat);
}

/// Catches `gallery.rs:GalleryStyle::config` leaving the Facet skin behind: choosing Graphite
/// after Obsidian would draw graphite surfaces faceted under a "Graphite" label.
#[test]
fn switching_away_from_obsidian_puts_the_flat_skin_back() {
    let obsidian = GalleryStyle::Obsidian.config(MoonThemeConfig::moon_terminal());
    assert_eq!(obsidian.mode, ThemeMode::Dark);
    assert_eq!(obsidian.dark.palette, MoonPalette::OBSIDIAN);
    assert_eq!(obsidian.dark.skin, MoonSkin::Facet);
    assert_eq!(obsidian.light.palette, MoonPalette::LIGHT);
    assert_eq!(obsidian.light.skin, MoonSkin::Flat);

    for style in GalleryStyle::ALL {
        if style == GalleryStyle::Obsidian {
            continue;
        }
        let next = style.config(obsidian.clone());
        assert_eq!(
            next.dark.skin,
            MoonSkin::Flat,
            "{}: dark side",
            style.name()
        );
        // The light faceted style is the one other style that draws with the skin, and only
        // on the side it renders.
        let light_skin = if style == GalleryStyle::LightFacet {
            MoonSkin::Facet
        } else {
            MoonSkin::Flat
        };
        assert_eq!(next.light.skin, light_skin, "{}: light side", style.name());
        assert_ne!(next.dark.palette, MoonPalette::OBSIDIAN, "{}", style.name());
    }
}

/// Catches a style's menu label drifting from the key `GalleryStyle::from_name` looks up: the
/// dropdown would show the option, but selecting it would silently do nothing.
#[test]
fn every_style_menu_key_selects_that_style() {
    for style in GalleryStyle::ALL {
        assert_eq!(GalleryStyle::from_name(style.name()), Some(style));
    }
}
