//! Regression coverage for the selector pill under the Flat and Facet skins.

// Do not use `super::*`: the glob would import the `gpui::test` macro, causing `#[test]` to
// recursively expand into itself.
use std::{cell::RefCell, rc::Rc};

use gpui::{
    AppContext as _, Bounds, BoxShadow, Context, IntoElement, ParentElement as _, Pixels, Render,
    Styled as _, TestAppContext, Window, div, linear_color_stop, linear_gradient, point, px,
};

use super::super::button::scene_probe::{Painted, Pointer, painted};
use super::{MoonSelectorPill, pill_facets, pill_is_raised};
use crate::moon::{
    MoonFacetKind, MoonPalette, MoonSkin, MoonTheme, MoonThemeConfig, MoonThemeTokens, facet,
    rgba_from,
};

/// Theme tokens painting `palette` with the Facet skin at the reference scale.
fn faceted_tokens(palette: MoonPalette) -> MoonThemeTokens {
    MoonThemeTokens {
        skin: MoonSkin::Facet,
        palette,
        ..MoonThemeTokens::default()
    }
}

/// An opaque gradient from `top` at the top edge to `bottom` at the bottom edge, written out
/// here rather than taken from the code under test.
fn top_to_bottom(top: u32, bottom: u32) -> gpui::Background {
    linear_gradient(
        180.0,
        linear_color_stop(rgba_from(top, 1.0), 0.0),
        linear_color_stop(rgba_from(bottom, 1.0), 1.0),
    )
}

/// Catches `selector.rs:pill_is_raised` answering for the Flat skin, which would restyle the
/// pill of every existing theme, or for a disabled pill, which keeps its flat disabled look.
#[test]
fn only_an_enabled_pill_under_the_facet_skin_is_raised() {
    assert!(pill_is_raised(MoonSkin::Facet, false));
    assert!(!pill_is_raised(MoonSkin::Facet, true));
    assert!(!pill_is_raised(MoonSkin::Flat, false));
    assert!(!pill_is_raised(MoonSkin::Flat, true));
}

/// Catches `selector.rs:pill_facets` handing the hover surface to the resting pill, or a
/// pressed surface that still casts its drop shadow: the pill would look hovered at rest, or
/// would not sink when pressed.
#[test]
fn a_raised_pill_rests_raised_relights_under_the_pointer_and_sinks_when_pressed() {
    for palette in [MoonPalette::OBSIDIAN, MoonPalette::LIGHT] {
        let tokens = faceted_tokens(palette);
        let [rest, hover, pressed] = pill_facets(&tokens);

        assert_eq!(
            rest.fill,
            top_to_bottom(palette.panel_high, palette.panel),
            "the resting pill is lit from above"
        );
        assert!(!rest.shadows.is_empty());
        assert_eq!(rest, facet(MoonFacetKind::Raised, &tokens));
        assert_eq!(hover, facet(MoonFacetKind::RaisedHover, &tokens));
        assert_ne!(hover, rest);

        assert_eq!(
            pressed.fill,
            top_to_bottom(palette.panel, palette.panel_high),
            "the pressed pill is lit from below"
        );
        assert_eq!(pressed.edge, rest.edge);
        let inner: Vec<BoxShadow> = rest
            .shadows
            .iter()
            .filter(|shadow| shadow.inset)
            .cloned()
            .collect();
        assert_eq!(pressed.shadows, inner, "the pressed pill casts no shadow");
        assert!(pressed.shadows.len() < rest.shadows.len());
    }
}

/// The Obsidian theme with its dark side drawn in `skin`: the same palette and scale on both
/// skins, so two renders differ by the skin alone.
fn obsidian_with(skin: MoonSkin) -> MoonThemeConfig {
    let mut config = MoonThemeConfig::moon_obsidian();
    config.dark.skin = skin;
    config
}

/// The pill every probe below draws, enabled or disabled.
fn probed_pill(disabled: bool) -> MoonSelectorPill {
    MoonSelectorPill::new("pill")
        .label("BTCUSDT")
        .leading_dot(MoonPalette::OBSIDIAN.green)
        .disabled(disabled)
}

/// Catches `selector.rs:MoonSelectorPill::render` painting anything new on a flat theme: the
/// pill keeps its solid panel fill, its hairline and its hover and pressed fills, and casts no
/// shadow. The colours are the flat design's own, written out rather than read back.
#[test]
fn a_flat_theme_paints_the_pill_as_it_did_before_the_facet_skin() {
    let p = MoonPalette::OBSIDIAN;
    let flat = |fill: (u32, f32), border: (u32, f32)| {
        Painted::flat(rgba_from(fill.0, fill.1), rgba_from(border.0, border.1))
    };
    let cases = [
        (
            "resting",
            Pointer::Away,
            false,
            flat((p.panel, 1.0), (p.border, 1.0)),
        ),
        (
            "hovered",
            Pointer::Over,
            false,
            flat((p.panel_high, 1.0), (p.border_hover, 1.0)),
        ),
        (
            "pressed",
            Pointer::Pressed,
            false,
            flat((p.shell_high, 1.0), (p.border_hover, 1.0)),
        ),
        (
            "disabled",
            Pointer::Over,
            true,
            flat((p.panel, 0.48), (p.border, 0.55)),
        ),
    ];

    let mismatches = paint_mismatches(MoonSkin::Flat, cases);
    assert!(
        mismatches.is_empty(),
        "under the Flat skin:\n{}",
        mismatches.join("\n")
    );
}

/// Draw each case's pill under the Obsidian palette with `skin` and describe every one that
/// did not paint what was expected, so one run names all the states that drifted. A case is
/// `(name, pointer, disabled, expected)`.
fn paint_mismatches(skin: MoonSkin, cases: [(&str, Pointer, bool, Painted); 4]) -> Vec<String> {
    cases
        .into_iter()
        .filter_map(|(name, pointer, disabled, expected)| {
            let got = painted(obsidian_with(skin), pointer, move || probed_pill(disabled));
            (got != expected).then(|| format!("the {name} pill painted {got:?}, not {expected:?}"))
        })
        .collect()
}

/// Catches `selector.rs:MoonSelectorPill::render` not painting the raised surfaces it resolves:
/// a pill that stays a flat slab on the faceted theme, does not relight under the pointer or
/// keeps its drop shadow while pressed. Also catches a disabled pill picking a surface up.
#[test]
fn the_faceted_theme_paints_the_pill_raised_relit_and_pressed() {
    let p = MoonPalette::OBSIDIAN;
    let tokens = faceted_tokens(p);
    let raised = facet(MoonFacetKind::Raised, &tokens);
    let cases = [
        (
            "resting",
            Pointer::Away,
            false,
            Painted::facet(raised.clone()),
        ),
        (
            "hovered",
            Pointer::Over,
            false,
            Painted::facet(facet(MoonFacetKind::RaisedHover, &tokens)),
        ),
        (
            "pressed",
            Pointer::Pressed,
            false,
            Painted {
                fill: Some(top_to_bottom(p.panel, p.panel_high)),
                border: Some(raised.edge),
                shadows: raised
                    .shadows
                    .iter()
                    .filter(|shadow| shadow.inset)
                    .cloned()
                    .collect(),
            },
        ),
        (
            "disabled",
            Pointer::Over,
            true,
            Painted::flat(rgba_from(p.panel, 0.48), rgba_from(p.border, 0.55)),
        ),
    ];

    let mismatches = paint_mismatches(MoonSkin::Facet, cases);
    assert!(
        mismatches.is_empty(),
        "under the Facet skin:\n{}",
        mismatches.join("\n")
    );
}

/// A window rendering one pill at the origin and recording its laid-out bounds.
struct PillHarness {
    disabled: bool,
    bounds: Rc<RefCell<Vec<Bounds<Pixels>>>>,
}

impl Render for PillHarness {
    /// Render the pill start-aligned in a wide row, so the row does not stretch it.
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let sink = self.bounds.clone();
        div()
            .w(px(400.0))
            .flex()
            .flex_row()
            .items_start()
            .justify_start()
            .on_children_prepainted(move |bounds, _, _| *sink.borrow_mut() = bounds)
            .child(probed_pill(self.disabled))
    }
}

/// Draw a pill in a real window and return its box.
fn laid_out_pill(cx: &mut TestAppContext, disabled: bool) -> Bounds<Pixels> {
    let bounds = Rc::new(RefCell::new(Vec::new()));
    let sink = bounds.clone();
    let window = cx.add_window(move |_, _| PillHarness {
        disabled,
        bounds: sink,
    });
    cx.update_window(window.into(), |_, window, cx| {
        window.draw(cx).clear();
    })
    .unwrap();

    let out = bounds.borrow();
    assert_eq!(out.len(), 1, "expected exactly the pill as a child");
    out[0]
}

/// Catches the Facet skin changing the pill's box, by adding a border width or padding, and
/// catches the faceted render path panicking: the pill keeps its shape and size on both skins,
/// enabled or disabled. The 24-pixel height is the pill's own design-reference default.
#[gpui::test]
fn the_facet_skin_leaves_the_pill_box_unchanged(cx: &mut TestAppContext) {
    cx.update(crate::init);

    for disabled in [false, true] {
        cx.update(|cx| MoonTheme::install_config(obsidian_with(MoonSkin::Flat), cx));
        let flat = laid_out_pill(cx, disabled);
        cx.update(|cx| MoonTheme::install_config(obsidian_with(MoonSkin::Facet), cx));
        let faceted = laid_out_pill(cx, disabled);

        assert_eq!(flat.origin, point(px(0.0), px(0.0)));
        assert_eq!(flat.size.height, px(24.0));
        assert_eq!(
            faceted, flat,
            "the pill moved under the Facet skin (disabled={disabled})"
        );
    }
}
