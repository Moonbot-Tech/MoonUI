//! Guards what a MoonProgress bar paints under each skin, and that the skin never changes its
//! height.

use gpui::{
    BoxShadow, Context, InteractiveElement as _, IntoElement, ParentElement as _, Render, Rgba,
    Styled as _, TestAppContext, VisualTestContext, Window, div, point, px,
};

use super::{MoonProgress, ProgressFacet};
use crate::moon::{
    MoonFacetKind, MoonTheme, MoonThemeConfig, MoonThemeTokens, MoonTone, facet, rgba_from,
};

/// Catches `progress.rs:ProgressFacet::resolve` ignoring the skin: every flat theme's bars would
/// leave the base progress bar — a tint of the tone under a solid fill — for a groove and a
/// gradient. With nothing resolved, `MoonProgress` renders the base bar exactly as it always has.
#[test]
fn flat_themes_resolve_no_progress_facet() {
    for tokens in [
        MoonThemeConfig::moon_terminal().dark,
        MoonThemeConfig::moon_terminal().light,
        MoonThemeConfig::moon_graphite().dark,
        MoonThemeConfig::moon_obsidian().light,
        MoonThemeTokens::default(),
    ] {
        let color = MoonTone::Positive.color(tokens.palette);
        assert_eq!(ProgressFacet::resolve(&tokens, color), None);
    }
}

/// Catches a faceted bar drifting from its spec: the track is the skin's well, and the fill runs
/// from its colour lifted a fifth of the way to white down to that colour, under a glow in the
/// colour at 40% — 8px of blur in the design, so 4px at the renderer. A fill lit from below, or
/// one painted in the accent whatever its tone, would make a warning bar read like a healthy one.
#[test]
fn a_faceted_bar_runs_a_lit_fill_of_its_tone_in_a_well() {
    let tokens = MoonThemeConfig::moon_obsidian().dark;
    let p = tokens.palette;

    // `green` 0x34C07A is (52, 192, 122) and `amber` 0xFFB347 is (255, 179, 71); a fifth of the
    // way to 255 adds 40.6, 12.6, 26.6 and 0, 15.2, 36.8.
    for (color, lit_top) in [
        (p.green, [92.6, 204.6, 148.6]),
        (p.amber, [255.0, 194.2, 107.8]),
    ] {
        let bar = ProgressFacet::resolve(&tokens, color).expect("faceted");
        assert_eq!(bar.track, facet(MoonFacetKind::Well, &tokens));
        assert_eq!(bar.track.edge, rgba_from(p.overlay, 0.06));

        let top = Rgba::from(bar.fill[0]);
        for (level, expected) in [top.r, top.g, top.b].into_iter().zip(lit_top) {
            assert!(
                (level * 255.0 - expected).abs() < 0.1,
                "fill top {top:?} is not {lit_top:?}"
            );
        }
        assert_eq!(bar.fill[1], rgba_from(color, 1.0));
        // The glow written out: centred on the fill, no spread, the design's 8px blur halved
        // into the renderer's blur radius.
        assert_eq!(
            bar.glow,
            BoxShadow {
                color: rgba_from(color, 0.4),
                offset: point(px(0.0), px(0.0)),
                blur_radius: px(4.0),
                spread_radius: px(0.0),
                inset: false,
            }
        );
    }
}

/// Two bars in a column, each in a probe that measures it: one at the default height, one taller.
struct ProgressHarness;

impl Render for ProgressHarness {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(200.))
            .child(
                div()
                    .debug_selector(|| "default-bar".into())
                    .child(MoonProgress::new("default").value(40.0)),
            )
            .child(
                div()
                    .debug_selector(|| "tall-bar".into())
                    .child(MoonProgress::new("tall").value(100.0).height(7.0)),
            )
    }
}

/// Catches the faceted bar changing a progress bar's size — a groove grown past the height the
/// caller set, or a bar that stops filling its parent's width: under the faceted theme each bar
/// must measure exactly what it measures under a flat one, 4px by default and the height asked
/// for otherwise, so a table row holding one never changes height with the theme.
#[gpui::test]
fn a_faceted_bar_keeps_the_height_and_width_of_the_flat_one(cx: &mut TestAppContext) {
    cx.update(crate::init);
    for (config, faceted) in [
        (MoonThemeConfig::moon_terminal(), false),
        (MoonThemeConfig::moon_obsidian(), true),
    ] {
        cx.update(|cx| MoonTheme::install_config(config, cx));
        cx.update(|cx| assert_eq!(MoonTheme::active_tokens(cx).skin.is_facet(), faceted));

        let window = cx.add_window(|_, _| ProgressHarness);
        let mut cx = VisualTestContext::from_window(window.into(), cx);
        cx.run_until_parked();

        let default_bar = cx.debug_bounds("default-bar").expect("bar must render");
        let tall_bar = cx.debug_bounds("tall-bar").expect("bar must render");
        assert_eq!(default_bar.size.height, px(4.), "faceted={faceted}");
        assert_eq!(tall_bar.size.height, px(7.), "faceted={faceted}");
        assert_eq!(default_bar.size.width, px(200.), "faceted={faceted}");
        assert_eq!(tall_bar.top(), default_bar.bottom(), "faceted={faceted}");
    }
}
