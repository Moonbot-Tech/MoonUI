//! Guards the lengths and the thumb a slider draws under each skin, and that the skin never
//! changes the row a slider takes.

use gpui::{
    AppContext as _, BoxShadow, Context, Entity, Hsla, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, Rgba, Styled as _, TestAppContext, VisualTestContext, Window, div,
    point, px,
};

use super::{MoonSlider, MoonSliderState};
use crate::moon::{MoonPalette, MoonSkin, MoonTheme, MoonThemeConfig, rgba_from};
use crate::slider::{SliderMetrics, SliderThumbFacet};

/// A shadow with no horizontal offset and no spread, written out rather than taken from
/// `slider.rs`.
fn shadow(y: f32, blur: f32, color: Hsla) -> BoxShadow {
    BoxShadow {
        color,
        offset: point(px(0.0), px(y)),
        blur_radius: px(blur),
        spread_radius: px(0.0),
        inset: false,
    }
}

/// Catches the Flat skin picking up the groove's lengths or the faceted thumb: every flat
/// theme's slider must stay the hairline it was — a 2px track its fill covers whole, under a 7px
/// thumb — and resolve no faceted thumb, so the thumb paints its flat disc and glow alone.
#[test]
fn the_flat_skin_keeps_the_hairline_slider() {
    assert_eq!(
        SliderMetrics::for_skin(MoonSkin::Flat),
        SliderMetrics {
            track: px(2.),
            fill: px(2.),
            thumb: px(7.),
        }
    );
    assert_eq!(SliderMetrics::for_skin(MoonSkin::default()).thumb, px(7.));

    for p in [
        MoonPalette::TERMINAL,
        MoonPalette::LIGHT,
        MoonPalette::OBSIDIAN,
    ] {
        for disabled in [false, true] {
            assert_eq!(
                SliderThumbFacet::resolve(MoonSkin::Flat, p, rgba_from(p.accent, 1.0), disabled),
                None
            );
        }
    }
}

/// Catches the Facet skin's slider drifting from its spec: the track becomes a 4px groove, the
/// fill keeps its 2px along the groove's middle, and the thumb grows to 10px. A fill as thick as
/// the groove would hide the groove; a thumb left at 7px would sit lost in it.
#[test]
fn the_facet_skin_sinks_a_groove_under_a_larger_thumb() {
    assert_eq!(
        SliderMetrics::for_skin(MoonSkin::Facet),
        SliderMetrics {
            track: px(4.),
            fill: px(2.),
            thumb: px(10.),
        }
    );
}

/// Catches the faceted thumb drifting from its spec: it runs from its colour mixed halfway to
/// white down to that colour, and casts a 1px-down, 2px-blur shadow at 60% under the 14px glow
/// at 72% the flat thumb has. A disabled thumb fades the fill to 42%, the shadow to 25% and the
/// glow to 14%; left at full strength it would look as usable as an enabled one.
#[test]
fn a_faceted_thumb_is_lit_from_above_and_keeps_the_flat_glow() {
    let p = MoonPalette::OBSIDIAN;
    let accent = rgba_from(p.accent, 1.0);
    // The accent 0xFFB347 is (255, 179, 71); halfway to 255 is (255, 217, 163).
    let lit_top = [255.0, 217.0, 163.0];

    for (disabled, fill_alpha, drop_alpha, glow_alpha) in
        [(false, 1.0, 0.6, 0.72), (true, 0.42, 0.25, 0.14)]
    {
        let thumb =
            SliderThumbFacet::resolve(MoonSkin::Facet, p, accent, disabled).expect("faceted");

        let top = Rgba::from(thumb.fill[0]);
        for (level, expected) in [top.r, top.g, top.b].into_iter().zip(lit_top) {
            assert!(
                (level * 255.0 - expected).abs() < 0.1,
                "thumb top {top:?} is not {lit_top:?}"
            );
        }
        assert!((top.a - fill_alpha).abs() < 1e-6, "disabled={disabled}");
        assert_eq!(
            thumb.fill[1],
            Hsla {
                a: fill_alpha,
                ..accent
            },
            "disabled={disabled}"
        );
        assert_eq!(
            thumb.shadows,
            vec![
                shadow(1.0, 2.0, rgba_from(p.shadow, drop_alpha)),
                shadow(
                    0.0,
                    14.0,
                    Hsla {
                        a: glow_alpha,
                        ..accent
                    }
                ),
            ],
            "disabled={disabled}"
        );
    }
}

/// A slider in a fixed-width column, inside a probe that measures the row it takes.
struct SliderHarness {
    state: Entity<MoonSliderState>,
}

impl Render for SliderHarness {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().w(px(200.)).child(
            div()
                .debug_selector(|| "slider-row".into())
                .flex()
                .child(MoonSlider::new(&self.state)),
        )
    }
}

/// Catches the Facet skin growing the row a slider takes — a container sized from the larger
/// thumb or the thicker groove instead of the fixed 18px hit area: under the faceted theme the
/// slider must take exactly the 200x18 row it takes under a flat one, so a form never reflows
/// when the theme changes.
#[gpui::test]
fn a_faceted_slider_takes_the_same_row_as_a_flat_one(cx: &mut TestAppContext) {
    cx.update(crate::init);
    for (config, faceted) in [
        (MoonThemeConfig::moon_terminal(), false),
        (MoonThemeConfig::moon_obsidian(), true),
    ] {
        cx.update(|cx| MoonTheme::install_config(config, cx));
        cx.update(|cx| assert_eq!(MoonSkin::active(cx).is_facet(), faceted));

        let window = cx.add_window(|_, cx| SliderHarness {
            state: cx.new(|_| MoonSliderState::new().default_value(40.)),
        });
        let mut cx = VisualTestContext::from_window(window.into(), cx);
        cx.run_until_parked();

        let row = cx.debug_bounds("slider-row").expect("slider must render");
        assert_eq!(row.size.width, px(200.), "faceted={faceted}");
        assert_eq!(row.size.height, px(18.), "faceted={faceted}");
    }
}
