//! Guards the frame a text input paints under each skin, and that the skin never changes the
//! input's size.

use gpui::{
    AppContext as _, BoxShadow, Context, Entity, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, Styled as _, TestAppContext, VisualTestContext, Window, div, point,
    px,
};

use super::{MoonInput, MoonInputSize, MoonInputState};
use crate::input::{InputFacet, input_style_for, trigger_facets};
use crate::moon::{
    MoonFacetKind, MoonTheme, MoonThemeConfig, MoonThemeTokens, MoonTone, ThemeMode, facet,
    rgba_from,
};
use crate::{Root, v_flex};

/// Catches `input.rs:InputFacet::resolve` ignoring the skin or the disabled flag: every flat
/// theme's inputs would be repainted as wells, or a disabled input on the faceted theme would
/// lose its flat dimmed frame. With no frame resolved, an input paints the flat fill, border and
/// hover it always had.
#[test]
fn flat_themes_and_disabled_inputs_resolve_no_frame() {
    for tokens in [
        MoonThemeConfig::moon_terminal().dark,
        MoonThemeConfig::moon_terminal().light,
        MoonThemeConfig::moon_graphite().dark,
        MoonThemeConfig::moon_obsidian().light,
        MoonThemeTokens::default(),
    ] {
        let accent = MoonTone::Info.color(tokens.palette);
        for selected in [false, true] {
            for focused in [false, true] {
                for disabled in [false, true] {
                    assert_eq!(
                        InputFacet::resolve(&tokens, accent, selected, focused, disabled),
                        None,
                        "a flat theme never resolves a frame"
                    );
                }
            }
        }
    }

    let faceted = MoonThemeConfig::moon_obsidian().dark;
    let accent = MoonTone::Info.color(faceted.palette);
    for focused in [false, true] {
        assert_eq!(
            InputFacet::resolve(&faceted, accent, false, focused, true),
            None,
            "a disabled input keeps the flat frame"
        );
    }
}

/// Catches a faceted input drifting from its spec. At rest it is the skin's well, edged in the
/// highlight ink at 6%, and under the pointer the edge rises to that ink at 20% over the same
/// fill. A selected input keeps the flat input's tone edge, 78% at rest and 82% under the
/// pointer. In focus the edge is the tone at 92% — under the pointer too, and whether or not the
/// input is selected — and a glow in the tone at 28% joins the well's shadows: 8px of blur in the
/// design, so 4px at the renderer. A glow left on an unfocused input, or missing from a focused
/// one, misreports where typing will land.
#[test]
fn a_faceted_input_rests_in_a_well_and_glows_in_its_tone_in_focus() {
    let tokens = MoonThemeConfig::moon_obsidian().dark;
    let p = tokens.palette;
    let well = facet(MoonFacetKind::Well, &tokens);
    let tone = MoonTone::Danger.color(p);
    assert_eq!(tone, p.red, "the Danger tone of a dark palette is its red");
    let edge = |alpha: f32| rgba_from(p.red, alpha);

    let resting = InputFacet::resolve(&tokens, tone, false, false, false).expect("faceted");
    assert_eq!(resting.fill, well.fill);
    assert_eq!(resting.edge, rgba_from(p.overlay, 0.06));
    assert_eq!(resting.hover_edge, rgba_from(p.overlay, 0.2));
    assert_eq!(resting.shadows, well.shadows);

    let selected = InputFacet::resolve(&tokens, tone, true, false, false).expect("faceted");
    assert_eq!(selected.fill, well.fill);
    assert_eq!(
        (selected.edge, selected.hover_edge),
        (edge(0.78), edge(0.82))
    );
    assert_eq!(selected.shadows, well.shadows);

    // The glow written out: centred on the input, no spread, the design's 8px blur halved into
    // the renderer's blur radius.
    let glow = BoxShadow {
        color: edge(0.28),
        offset: point(px(0.0), px(0.0)),
        blur_radius: px(4.0),
        spread_radius: px(0.0),
        inset: false,
    };
    let mut well_and_glow = well.shadows.clone();
    well_and_glow.push(glow);
    for selected in [false, true] {
        let focused = InputFacet::resolve(&tokens, tone, selected, true, false).expect("faceted");
        assert_eq!(focused.fill, well.fill);
        assert_eq!((focused.edge, focused.hover_edge), (edge(0.92), edge(0.92)));
        assert_eq!(focused.shadows, well_and_glow);
    }
}

/// Catches `input.rs:input_style_for` giving a faceted field the flat fill, or a flat theme the
/// sunk one. The date field is drawn by mirrored donor code that knows only this pair: with the
/// flat `shell_high` under the Facet skin it would stand out as a light box beside the wells
/// of the inputs around it, and with `gutter` under a flat theme every field would darken.
#[test]
fn an_input_like_field_takes_the_well_floor_only_under_the_facet_skin() {
    let faceted = MoonThemeConfig::moon_obsidian().dark;
    let p = faceted.palette;
    assert_eq!(
        input_style_for(false, &faceted),
        (rgba_from(p.gutter, 1.0), rgba_from(p.text_soft, 1.0))
    );
    assert_eq!(
        input_style_for(true, &faceted),
        (rgba_from(p.panel, 0.55), rgba_from(p.text_muted, 0.45))
    );

    for flat in [
        MoonThemeConfig::moon_terminal().dark,
        MoonThemeConfig::moon_terminal().light,
        MoonThemeConfig::moon_graphite().dark,
        MoonThemeTokens::default(),
    ] {
        let p = flat.palette;
        assert_eq!(
            input_style_for(false, &flat),
            (rgba_from(p.shell_high, 1.0), rgba_from(p.text_soft, 1.0))
        );
        assert_eq!(
            input_style_for(true, &flat),
            (rgba_from(p.panel, 0.55), rgba_from(p.text_muted, 0.45))
        );
    }
}

/// Catches `input.rs:trigger_facets` ignoring the skin or the disabled flag, or handing a
/// button-styled trigger the input's well: the select beside a button would sink while the
/// button stands, and the one beside an input would stand while the input sinks.
#[test]
fn a_faceted_trigger_matches_the_control_it_is_styled_as() {
    use crate::button::{ButtonVariant, ButtonVisualState, raised_pressed_facet};

    for tokens in [
        MoonThemeConfig::moon_terminal().dark,
        MoonThemeConfig::moon_terminal().light,
        MoonThemeTokens::default(),
    ] {
        for variant in [None, Some(ButtonVariant::Panel), Some(ButtonVariant::Blue)] {
            assert_eq!(trigger_facets(&tokens, variant, false, false), None);
        }
    }

    let tokens = MoonThemeConfig::moon_obsidian().dark;
    let p = tokens.palette;
    for variant in [None, Some(ButtonVariant::Panel), Some(ButtonVariant::Blue)] {
        assert_eq!(trigger_facets(&tokens, variant, false, true), None);
    }

    assert_eq!(
        trigger_facets(&tokens, Some(ButtonVariant::Panel), false, false),
        Some([
            facet(MoonFacetKind::Raised, &tokens),
            facet(MoonFacetKind::RaisedHover, &tokens),
            raised_pressed_facet(&tokens),
        ])
    );

    let well = facet(MoonFacetKind::Well, &tokens);
    for variant in [
        ButtonVariant::Blue,
        ButtonVariant::Amber,
        ButtonVariant::Danger,
    ] {
        let surfaces = trigger_facets(&tokens, Some(variant), false, false).expect("tone surface");
        for (surface, state) in surfaces.into_iter().zip([
            ButtonVisualState::Normal,
            ButtonVisualState::Hovered,
            ButtonVisualState::Active,
        ]) {
            assert_eq!(Some(surface), variant.resolve_facet(&tokens, false, state));
        }
    }
    for variant in [ButtonVariant::Ghost, ButtonVariant::Bare] {
        assert_eq!(trigger_facets(&tokens, Some(variant), false, false), None);
    }
    let [rest, hovered, pressed] = trigger_facets(&tokens, None, false, false).expect("faceted");
    assert_eq!(rest, well);
    assert_eq!(hovered.fill, well.fill);
    assert_eq!(hovered.edge, rgba_from(p.overlay, 0.2));
    assert_eq!(hovered.shadows, well.shadows);
    assert_eq!(pressed, hovered);

    // In focus, or with its menu open, the field takes the input's focus frame in the Info tone.
    let frame =
        InputFacet::resolve(&tokens, MoonTone::Info.color(p), false, true, false).expect("faceted");
    let [rest, hovered, _] = trigger_facets(&tokens, None, true, false).expect("faceted");
    assert_eq!((rest.edge, hovered.edge), (frame.edge, frame.hover_edge));
    assert_eq!(rest.shadows, frame.shadows);
    assert_ne!(rest.shadows, well.shadows);
}

/// One normal-size input in a fixed-width column, inside a probe that measures it.
struct InputHarness;

impl Render for InputHarness {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().w(px(180.)).child(
            div()
                .debug_selector(|| "input-probe".into())
                .child(MoonInput::new("field").normal().placeholder("Coin")),
        )
    }
}

/// Catches the faceted frame changing an input's size — a well that brings the design mock-up's
/// 22px height with it: under the faceted theme a normal input must measure the 180x28 it
/// measures under a flat one, so a form row never changes height with the theme.
#[gpui::test]
fn a_faceted_input_keeps_the_size_of_the_flat_one(cx: &mut TestAppContext) {
    cx.update(crate::init);
    for (config, faceted) in [
        (MoonThemeConfig::moon_terminal(), false),
        (MoonThemeConfig::moon_obsidian(), true),
    ] {
        cx.update(|cx| MoonTheme::install_config(config, cx));
        cx.update(|cx| assert_eq!(MoonTheme::active_tokens(cx).skin.is_facet(), faceted));

        let window = cx.add_window(|window, cx| {
            let view = cx.new(|_| InputHarness);
            crate::Root::new(view, window, cx).bordered(false)
        });
        let mut cx = VisualTestContext::from_window(window.into(), cx);
        cx.run_until_parked();

        let probe = cx.debug_bounds("input-probe").expect("input must render");
        assert_eq!(probe.size.width, px(180.), "faceted={faceted}");
        assert_eq!(probe.size.height, px(28.), "faceted={faceted}");
    }
}

/// The following sibling measures the input's real layout height rather than its builder fields.
struct HeightHarness {
    input: Entity<MoonInputState>,
    size: MoonInputSize,
}

impl Render for HeightHarness {
    /// Leave vertical size unconstrained so a wrong input height cannot be hidden by the host.
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w(px(240.))
            .debug_selector(|| "input-height-host".into())
            .child(
                MoonInput::new("height-input")
                    .state(&self.input)
                    .size(self.size),
            )
            .child(
                div()
                    .w_full()
                    .h(px(1.))
                    .debug_selector(|| "after-input".into()),
            )
    }
}

/// Restoring the multiline-only height override makes the Custom case 22px instead of 19px.
#[gpui::test]
fn custom_height_uses_its_own_text_metrics(cx: &mut TestAppContext) {
    cx.update(crate::init);
    for mode in [ThemeMode::Dark, ThemeMode::Light] {
        cx.update(|cx| {
            let mut config = MoonThemeConfig::default().with_font_delta(3.);
            config.mode = mode;
            MoonTheme::install_config(config, cx);
        });
        for (size, expected) in [
            (MoonInputSize::Compact, 23.),
            (MoonInputSize::Small, 25.),
            (MoonInputSize::Normal, 31.),
            (
                MoonInputSize::Custom {
                    height: 19.,
                    radius: 2.,
                    font_size: 10.,
                    line_height: 16.,
                    pad_x: 4.,
                    pad_y: 0.,
                    gap: 2.,
                },
                19.,
            ),
            (
                MoonInputSize::Custom {
                    height: 0.,
                    radius: 2.,
                    font_size: 10.,
                    line_height: 16.,
                    pad_x: 4.,
                    pad_y: 0.,
                    gap: 2.,
                },
                19.,
            ),
        ] {
            let window = cx.add_window(move |window, cx| {
                let input = cx.new(|cx| MoonInputState::new(window, cx).default_value("123"));
                let view = cx.new(|_| HeightHarness { input, size });
                Root::new(view, window, cx).bordered(false)
            });
            let mut visual = VisualTestContext::from_window(window.into(), cx);
            visual.run_until_parked();
            let host = visual
                .debug_bounds("input-height-host")
                .expect("host rendered");
            let next = visual
                .debug_bounds("after-input")
                .expect("next sibling rendered");
            assert_eq!(
                next.top() - host.top(),
                px(expected),
                "size={size:?}, mode={mode:?}"
            );
        }
    }
}
