//! Guards the facet rules — what each surface resolves to on a dark and on a light palette, and
//! how a blur of the design reaches the renderer — and where the Facet backdrop is painted.

use gpui::{
    AppContext as _, Bounds, BoxShadow, Context, Hsla, IntoElement, Pixels, Render, TestAppContext,
    Window, div, linear_color_stop, linear_gradient, point, px,
};

use super::{
    MoonFacetKind, MoonFacetTint, MoonSkin, ambient_light, backdrop_fill, facet, facet_glow,
    facet_tinted, fading_halves, lift, mix, moon_backdrop, sink,
};
use crate::moon::{
    MoonBackgroundPolicy, MoonRoot, MoonTheme, MoonThemeConfig, MoonThemeTokens,
    foundation::ThemeMode,
    tokens::{MoonPalette, rgba_from},
};

const WHITE: u32 = 0xFFFFFF;

const ALL_KINDS: [MoonFacetKind; 7] = [
    MoonFacetKind::Slab,
    MoonFacetKind::Raised,
    MoonFacetKind::RaisedHover,
    MoonFacetKind::Well,
    MoonFacetKind::Socket,
    MoonFacetKind::Popup,
    MoonFacetKind::Card,
];

/// Theme tokens for `palette` at the default scale.
fn tokens_for(palette: MoonPalette) -> MoonThemeTokens {
    MoonThemeTokens {
        palette,
        ..MoonThemeTokens::default()
    }
}

/// Theme tokens for `palette` with every length doubled.
fn doubled_tokens_for(palette: MoonPalette) -> MoonThemeTokens {
    let mut tokens = tokens_for(palette);
    tokens.scale.ui = 2.0;
    tokens
}

/// An opaque gradient from `top` to `bottom`, written out rather than taken from `skin.rs`.
fn top_to_bottom(top: u32, bottom: u32) -> gpui::Background {
    linear_gradient(
        180.0,
        linear_color_stop(rgba_from(top, 1.0), 0.0),
        linear_color_stop(rgba_from(bottom, 1.0), 1.0),
    )
}

/// A shadow with no horizontal offset and no spread, written out rather than taken from
/// `skin.rs`. `design_blur` is the blur as the design gives it, a CSS `box-shadow` blur.
fn shadow(y: f32, design_blur: f32, color: Hsla, inset: bool) -> BoxShadow {
    BoxShadow {
        color,
        offset: point(px(0.0), px(y)),
        // The renderer's blur radius is the gaussian's standard deviation: half a CSS blur.
        blur_radius: px(design_blur / 2.0),
        spread_radius: px(0.0),
        inset,
    }
}

/// Catches `skin.rs:mix` weighting the wrong end, or truncating a channel instead of rounding
/// it: every facet fill is built from these three, so a slip here tints or darkens all of them.
#[test]
fn lift_sink_and_mix_move_each_channel_by_the_given_share() {
    // 0x20 + 0.25 * (0xFF - 0x20) = 87.75, 0x40 -> 111.75, 0x60 -> 135.75, each rounded up.
    assert_eq!(lift(0x204060, 0.25), 0x587088);
    // 0x20 * 0.75 = 24, 0x40 -> 48, 0x60 -> 72.
    assert_eq!(sink(0x204060, 0.25), 0x183048);
    // A quarter of the way from black to (0xFF, 0x80, 0x00): 63.75, 32, 0.
    assert_eq!(mix(0x000000, 0xFF8000, 0.25), 0x402000);

    assert_eq!(lift(0x204060, 0.0), 0x204060);
    assert_eq!(lift(0x204060, 1.0), WHITE);
    assert_eq!(sink(0x204060, 1.0), 0x000000);
    assert_eq!(
        mix(0x204060, 0xFF8000, 2.0),
        0xFF8000,
        "a share above one is clamped, not extrapolated past the end colour"
    );
}

/// Catches a kind resolving without shadows, or without the inset one in the documented first
/// position: under the Facet skin that surface would lose its top light or its lift off the
/// surface below, on a dark palette or a light one.
#[test]
fn every_kind_stacks_an_inset_shadow_ahead_of_its_outer_ones() {
    for (name, palette) in [
        ("terminal", MoonPalette::TERMINAL),
        ("light", MoonPalette::LIGHT),
        ("graphite", MoonPalette::GRAPHITE),
        ("obsidian", MoonPalette::OBSIDIAN),
    ] {
        let tokens = tokens_for(palette);
        for kind in ALL_KINDS {
            let shadows = facet(kind, &tokens).shadows;
            assert!(
                !shadows.is_empty(),
                "{name} {kind:?} must resolve a shadow stack"
            );
            assert!(
                shadows[0].inset,
                "{name} {kind:?} lists its inset shadow first"
            );
            assert!(
                shadows.len() > 1 && shadows[1..].iter().all(|shadow| !shadow.inset),
                "{name} {kind:?} follows it with outer shadows only"
            );
        }
    }
}

/// Catches swapping the gradient ends in `skin.rs:facet`, which lights a raised control from
/// below so that it reads as pressed, and catches a wrong strength on its edge or shadows.
#[test]
fn a_raised_facet_runs_from_panel_high_down_to_panel_on_a_dark_palette() {
    let p = MoonPalette::OBSIDIAN;
    let raised = facet(MoonFacetKind::Raised, &tokens_for(p));

    assert_eq!(raised.fill, top_to_bottom(p.panel_high, p.panel));
    assert_eq!(raised.edge, rgba_from(p.overlay, 0.09));
    assert_eq!(
        raised.shadows,
        vec![
            shadow(1.0, 0.0, rgba_from(p.overlay, 0.16), true),
            shadow(1.0, 2.0, rgba_from(p.shadow, 0.55), false),
        ]
    );

    let hover = facet(MoonFacetKind::RaisedHover, &tokens_for(p));
    assert_eq!(hover.edge, rgba_from(p.overlay, 0.2));
    assert_eq!(
        hover.shadows, raised.shadows,
        "hover relights the fill and the edge, and leaves the shadows alone"
    );
    assert_ne!(hover.fill, raised.fill);
}

/// Catches `skin.rs:facet` reading the dark strengths on a light palette: a 55% black shadow
/// under every light-theme button, a black "highlight" along its top and a 9% black edge
/// where the palette's own border belongs.
#[test]
fn a_light_palette_takes_its_own_borders_white_highlights_and_faint_shadows() {
    let p = MoonPalette::LIGHT;
    let tokens = tokens_for(p);

    let raised = facet(MoonFacetKind::Raised, &tokens);
    assert_eq!(raised.edge, rgba_from(p.border, 1.0));
    assert_eq!(
        raised.shadows,
        vec![
            shadow(1.0, 0.0, rgba_from(WHITE, 0.9), true),
            shadow(1.0, 2.0, rgba_from(p.shadow, 0.12), false),
        ]
    );

    assert_eq!(
        facet(MoonFacetKind::RaisedHover, &tokens).edge,
        rgba_from(p.border_hover, 1.0)
    );
    assert_eq!(
        facet(MoonFacetKind::Popup, &tokens).shadows,
        vec![
            shadow(1.0, 0.0, rgba_from(WHITE, 0.9), true),
            shadow(28.0, 60.0, rgba_from(p.shadow, 0.16), false),
            shadow(6.0, 16.0, rgba_from(p.shadow, 0.10), false),
        ]
    );
    assert_eq!(
        facet(MoonFacetKind::Well, &tokens).shadows,
        vec![
            shadow(1.0, 3.0, rgba_from(p.shadow, 0.12), true),
            shadow(1.0, 0.0, rgba_from(WHITE, 0.8), false),
        ]
    );
}

/// Catches turning a well into one more raised surface in `skin.rs:facet`: an input would be
/// lit along its top like the button beside it instead of reading as sunk into the panel.
#[test]
fn a_well_is_darkest_at_the_top_and_carries_its_shadow_inside() {
    let p = MoonPalette::OBSIDIAN;
    let well = facet(MoonFacetKind::Well, &tokens_for(p));

    // `gutter` 0x08090B sunk by 15%: 6.8, 7.65 and 9.35, rounded.
    const WELL_TOP: u32 = 0x070809;
    assert_eq!(well.fill, top_to_bottom(WELL_TOP, p.gutter));
    assert_eq!(well.edge, rgba_from(p.overlay, 0.06));
    assert_eq!(
        well.shadows,
        vec![
            shadow(1.0, 3.0, rgba_from(p.shadow, 0.7), true),
            shadow(1.0, 0.0, rgba_from(p.overlay, 0.07), false),
        ]
    );
}

/// Catches a socket losing its stronger edge in `skin.rs:facet`, or drifting from the well it
/// is cut from: an unchecked checkbox would vanish into its card, or stop matching the input
/// beside it.
#[test]
fn a_socket_is_a_well_under_an_edge_that_reads_at_small_sizes() {
    for p in [MoonPalette::OBSIDIAN, MoonPalette::LIGHT] {
        let tokens = tokens_for(p);
        let well = facet(MoonFacetKind::Well, &tokens);
        let socket = facet(MoonFacetKind::Socket, &tokens);

        assert_eq!(socket.fill, well.fill);
        assert_eq!(socket.shadows, well.shadows);
        let expected_edge = if p.is_light() {
            rgba_from(p.border_hover, 1.0)
        } else {
            rgba_from(p.overlay, 0.17)
        };
        assert_eq!(socket.edge, expected_edge);
        assert_ne!(socket.edge, well.edge);
    }
}

/// Catches `skin.rs:MoonFacet::paint` dropping one of the three parts of a surface, or setting
/// a border width or radius of its own: a component painting through it would lose its edge or
/// shadow, or have its frame resized by the skin.
#[test]
fn painting_a_facet_sets_its_fill_edge_and_shadows_and_nothing_else() {
    use gpui::{Fill, StyleRefinement};

    let surface = facet(MoonFacetKind::Raised, &tokens_for(MoonPalette::OBSIDIAN));
    let painted = surface.clone().paint(StyleRefinement::default());

    assert_eq!(painted.background, Some(Fill::from(surface.fill)));
    assert_eq!(painted.border_color, Some(surface.edge));
    assert_eq!(painted.box_shadow, Some(surface.shadows));
    assert_eq!(
        StyleRefinement {
            background: None,
            border_color: None,
            box_shadow: None,
            ..painted
        },
        StyleRefinement::default()
    );
}

/// Catches a light palette's well taking the dark palette's depth in `skin.rs:well_sink`: 15% of
/// black on a near-white floor turns every input and groove into a slab of grey.
#[test]
fn a_light_well_sinks_a_fraction_of_what_a_dark_one_does() {
    let p = MoonPalette::LIGHT;
    let well = facet(MoonFacetKind::Well, &tokens_for(p));

    // `gutter` 0xEEF2F6 sunk by 4%: 228.48, 232.32 and 236.16, rounded.
    const WELL_TOP: u32 = 0xE4E8EC;
    assert_eq!(well.fill, top_to_bottom(WELL_TOP, p.gutter));
}

/// Catches a facet shadow built from a raw `px(..)` instead of `tokens.ui(..)`: at a doubled UI
/// scale the control doubles and its shadow, glow or backdrop light would stay the old size.
#[test]
fn facet_lengths_follow_the_ui_scale() {
    let p = MoonPalette::OBSIDIAN;
    let tokens = doubled_tokens_for(p);

    // Each length is the design's, doubled: the slab's `0 18px 40px` and `0 2px 6px` shadows,
    // and a glow blurred by 18.
    assert_eq!(
        facet(MoonFacetKind::Slab, &tokens).shadows,
        vec![
            shadow(2.0, 0.0, rgba_from(p.overlay, 0.12), true),
            shadow(36.0, 80.0, rgba_from(p.shadow, 0.65), false),
            shadow(4.0, 12.0, rgba_from(p.shadow, 0.5), false),
        ]
    );
    assert_eq!(
        facet_glow(p.blue, 0.26, 18.0, &tokens),
        shadow(0.0, 36.0, rgba_from(p.blue, 0.26), false),
        "a glow has no offset and no spread, and its design blur is scaled"
    );
    // The light written out: 260 design pixels of blur, doubled and then halved for the renderer,
    // around 140 of spread, doubled only.
    assert_eq!(
        ambient_light(p.accent, 0.10, &tokens),
        BoxShadow {
            color: rgba_from(p.accent, 0.10),
            offset: point(px(0.0), px(0.0)),
            blur_radius: px(260.0),
            spread_radius: px(280.0),
            inset: false,
        }
    );
}

/// Catches `skin.rs:renderer_blur` passing a design blur through as it is, or the conversion
/// reaching an offset or a spread. A blur of the design is a CSS `box-shadow` blur, twice the
/// gaussian's standard deviation that the renderer takes as its blur radius: unhalved, every
/// glow and shadow spreads twice as wide and arrives much fainter than designed.
#[test]
fn a_design_blur_reaches_the_renderer_halved_and_offsets_and_spread_do_not() {
    let p = MoonPalette::OBSIDIAN;
    let tokens = tokens_for(p);

    // A glow the design blurs by 18.
    assert_eq!(
        facet_glow(p.accent, 0.26, 18.0, &tokens),
        BoxShadow {
            color: rgba_from(p.accent, 0.26),
            offset: point(px(0.0), px(0.0)),
            blur_radius: px(9.0),
            spread_radius: px(0.0),
            inset: false,
        }
    );

    // The slab's long shadow, `0 18px 40px` in the design: 18 down still, blurred by 20.
    assert_eq!(
        facet(MoonFacetKind::Slab, &tokens).shadows[1],
        BoxShadow {
            color: rgba_from(p.shadow, 0.65),
            offset: point(px(0.0), px(18.0)),
            blur_radius: px(20.0),
            spread_radius: px(0.0),
            inset: false,
        }
    );

    // A backdrop light, `0 0 260px 140px` in the design: spread by 140 still, blurred by 130.
    assert_eq!(
        ambient_light(p.blue, 0.08, &tokens),
        BoxShadow {
            color: rgba_from(p.blue, 0.08),
            offset: point(px(0.0), px(0.0)),
            blur_radius: px(130.0),
            spread_radius: px(140.0),
            inset: false,
        }
    );
}

/// Catches `skin.rs:facet_tinted` mixing the tone over the wrong base or with the two shares
/// swapped, and catches a selected surface losing the glow that tells it from a soft one.
#[test]
fn tinted_facets_wash_the_panel_with_the_tone_and_only_selected_and_solid_glow() {
    let p = MoonPalette::OBSIDIAN;
    let tokens = tokens_for(p);
    let tone = p.accent;
    // `panel` 0x171A1F with `accent` 0xFFB347 mixed in at 14%, 6%, 20% and 8%.
    const SOFT_TOP: u32 = 0x372F25;
    const SOFT_BOTTOM: u32 = 0x252321;
    const SELECTED_TOP: u32 = 0x453927;
    const SELECTED_BOTTOM: u32 = 0x2A2622;
    // `accent` lifted by 25%, sunk by 6% and sunk by 20%.
    const SOLID_TOP: u32 = 0xFFC675;
    const SOLID_BOTTOM: u32 = 0xF0A843;
    const SOLID_EDGE: u32 = 0xCC8F39;

    let soft = facet_tinted(tone, MoonFacetTint::Soft, &tokens);
    assert_eq!(soft.fill, top_to_bottom(SOFT_TOP, SOFT_BOTTOM));
    assert_eq!(soft.edge, rgba_from(tone, 0.42));
    assert_eq!(
        soft.shadows,
        vec![
            shadow(1.0, 0.0, rgba_from(p.overlay, 0.10), true),
            shadow(1.0, 2.0, rgba_from(p.shadow, 0.55), false),
        ]
    );

    let selected = facet_tinted(tone, MoonFacetTint::Selected, &tokens);
    assert_eq!(selected.fill, top_to_bottom(SELECTED_TOP, SELECTED_BOTTOM));
    assert_eq!(selected.edge, rgba_from(tone, 0.7));
    let mut soft_and_glow = soft.shadows.clone();
    soft_and_glow.push(shadow(0.0, 18.0, rgba_from(tone, 0.26), false));
    assert_eq!(selected.shadows, soft_and_glow);

    let solid = facet_tinted(tone, MoonFacetTint::Solid, &tokens);
    assert_eq!(solid.fill, top_to_bottom(SOLID_TOP, SOLID_BOTTOM));
    assert_eq!(solid.edge, rgba_from(SOLID_EDGE, 1.0));
    assert_eq!(
        solid.shadows,
        vec![
            shadow(1.0, 0.0, rgba_from(WHITE, 0.45), true),
            shadow(1.0, 2.0, rgba_from(p.shadow, 0.55), false),
            shadow(0.0, 18.0, rgba_from(tone, 0.32), false),
        ]
    );

    let light = MoonPalette::LIGHT;
    let solid_on_light = facet_tinted(light.accent, MoonFacetTint::Solid, &tokens_for(light));
    assert_eq!(
        solid_on_light.shadows,
        vec![
            shadow(1.0, 0.0, rgba_from(WHITE, 0.45), true),
            shadow(1.0, 2.0, rgba_from(light.shadow, 0.12), false),
            shadow(0.0, 18.0, rgba_from(light.accent, 0.2), false),
        ],
        "a light palette keeps the white highlight and takes a fainter shadow and glow"
    );
}

/// Catches swapping the two halves in `skin.rs:fading_halves`: the rule would be solid at both
/// ends and vanish in the middle, a gap where the line belongs.
#[test]
fn a_fading_rule_rises_to_its_colour_in_the_middle_and_falls_away_again() {
    let color = rgba_from(MoonPalette::OBSIDIAN.accent, 0.6);
    let clear = Hsla { a: 0.0, ..color };

    for angle in [90.0, 180.0] {
        assert_eq!(
            fading_halves(color, angle),
            [
                linear_gradient(
                    angle,
                    linear_color_stop(clear, 0.0),
                    linear_color_stop(color, 1.0)
                ),
                linear_gradient(
                    angle,
                    linear_color_stop(color, 0.0),
                    linear_color_stop(clear, 1.0)
                ),
            ]
        );
    }
}

/// Catches the backdrop fill losing its top light, or carrying it all the way down the window
/// so that the whole window is lifted off `shell` instead of only its upper part.
#[test]
fn the_backdrop_fill_is_lit_at_the_top_and_plain_shell_from_halfway_down() {
    let p = MoonPalette::OBSIDIAN;
    // `shell` 0x050608 lifted by 3.5%: 13.75, 14.715 and 16.645, rounded.
    const LIT_SHELL: u32 = 0x0E0F11;

    assert_eq!(
        backdrop_fill(p),
        linear_gradient(
            180.0,
            linear_color_stop(rgba_from(LIT_SHELL, 1.0), 0.0),
            linear_color_stop(rgba_from(p.shell, 1.0), 0.5),
        )
    );
}

/// Catches `skin.rs:moon_backdrop` ignoring the skin: every flat theme would get a gradient
/// window fill and two coloured lights it never asked for.
#[test]
fn flat_tokens_have_no_backdrop() {
    for palette in [
        MoonPalette::TERMINAL,
        MoonPalette::LIGHT,
        MoonPalette::OBSIDIAN,
    ] {
        assert!(moon_backdrop(&tokens_for(palette)).is_none());
    }
}

/// Catches `skin.rs:MoonSkin::active` reading the configured dark side instead of the active
/// one, or defaulting to Facet with no theme: components would draw facets on a flat theme.
#[gpui::test]
fn the_active_skin_is_the_installed_themes_active_side(cx: &mut TestAppContext) {
    cx.update(|cx| assert_eq!(MoonSkin::active(cx), MoonSkin::Flat, "no theme installed"));

    cx.update(crate::init);
    cx.update(|cx| MoonTheme::install_config(MoonThemeConfig::moon_obsidian(), cx));
    cx.update(|cx| assert_eq!(MoonSkin::active(cx), MoonSkin::Facet));

    cx.update(|cx| MoonTheme::set_mode(ThemeMode::Light, cx));
    cx.update(|cx| {
        assert_eq!(
            MoonSkin::active(cx),
            MoonSkin::Flat,
            "Obsidian's light side is flat"
        )
    });

    cx.update(|cx| MoonTheme::install_config(MoonThemeConfig::moon_terminal(), cx));
    cx.update(|cx| assert_eq!(MoonSkin::active(cx), MoonSkin::Flat));
}

/// The content view of the probe windows below; it paints nothing.
struct Blank;

impl Render for Blank {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

/// Open a window on a `MoonRoot` with `policy` and return where its backdrop was painted and
/// the size of the window it was painted in.
fn backdrop_in_a_root(
    policy: MoonBackgroundPolicy,
    cx: &mut TestAppContext,
) -> (Option<Bounds<Pixels>>, gpui::Size<Pixels>) {
    let (_root, cx) = cx.add_window_view(|window, cx| {
        let view = cx.new(|_| Blank);
        MoonRoot::new(view, window, cx)
            .bordered(false)
            .background_policy(policy)
    });
    cx.run_until_parked();
    let viewport = cx.update(|window, _| window.viewport_size());
    (cx.debug_bounds("moon-backdrop"), viewport)
}

/// Catches `root.rs:MoonRoot::render` painting the backdrop under the Flat skin, which would
/// change every existing theme, or under a root that paints no fill, where the opaque backdrop
/// would cover the GPU canvas such a root exists to show through.
#[gpui::test]
fn only_an_opaque_root_under_the_facet_skin_paints_the_backdrop(cx: &mut TestAppContext) {
    cx.update(crate::init);

    cx.update(|cx| MoonTheme::install_config(MoonThemeConfig::moon_terminal(), cx));
    let (flat, _) = backdrop_in_a_root(MoonBackgroundPolicy::Opaque, cx);
    assert_eq!(flat, None, "a flat theme's root has no backdrop element");

    cx.update(|cx| MoonTheme::install_config(MoonThemeConfig::moon_obsidian(), cx));
    let (faceted, viewport) = backdrop_in_a_root(MoonBackgroundPolicy::Opaque, cx);
    assert_eq!(
        faceted,
        Some(Bounds::new(point(px(0.0), px(0.0)), viewport)),
        "the backdrop fills the window behind the content"
    );

    for policy in [
        MoonBackgroundPolicy::NoFill,
        MoonBackgroundPolicy::Transparent,
    ] {
        let (see_through, _) = backdrop_in_a_root(policy, cx);
        assert_eq!(see_through, None, "{policy:?} root must stay see-through");
    }

    cx.update(|cx| MoonTheme::set_mode(ThemeMode::Light, cx));
    let (light_side, _) = backdrop_in_a_root(MoonBackgroundPolicy::Opaque, cx);
    assert_eq!(light_side, None, "Obsidian's light side is flat");
}
