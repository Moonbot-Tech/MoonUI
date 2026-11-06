//! Unit coverage for base button builders, interaction states, and Moon sizing.

use super::{
    Button, ButtonCustomVariant, ButtonRounded, ButtonVariant, ButtonVariantStyle, ButtonVariants,
    ButtonVisualState, MoonButtonMetrics, pointer_feedback_enabled, raised_pressed_facet,
};
use crate::{
    Disableable as _, Selectable as _, Sizable as _, Size,
    moon::{
        MoonFacet, MoonFacetKind, MoonFacetTint, MoonPalette, MoonSkin, MoonTheme, MoonThemeConfig,
        MoonThemeTokens, ThemeMode, facet, facet_tinted, rgba_from,
    },
};
use gpui::{Background, BoxShadow, Hsla, linear_color_stop, linear_gradient, point, px};

/// Catches removing an assignment from a `Button` builder, which would make the corresponding
/// public modifier silently render with its default value.
#[gpui::test]
fn test_button_builder(_cx: &mut gpui::TestAppContext) {
    let button = Button::new("complex-button")
        .label("Save Changes")
        .primary()
        .outline()
        .large()
        .tooltip("Click to save")
        .compact()
        .loading(false)
        .disabled(false)
        .selected(false)
        .tab_index(1)
        .tab_stop(true)
        .dropdown_caret(false)
        .rounded(ButtonRounded::Medium)
        .on_click(|_, _, _| {});

    assert_eq!(button.label, Some("Save Changes".into()));
    assert_eq!(button.variant, ButtonVariant::Primary);
    assert!(button.outline);
    assert_eq!(button.size, Size::Large);
    assert!(button.tooltip.is_some());
    assert!(button.compact);
    assert!(!button.loading);
    assert!(!button.disabled);
    assert!(!button.selected);
    assert_eq!(button.tab_index, 1);
    assert!(button.tab_stop);
    assert!(!button.dropdown_caret);
    assert!(matches!(button.rounded, ButtonRounded::Medium));
}

/// Catches removing the disabled or loading guard from `Button::clickable`, which would let an
/// unavailable control advertise and accept click behavior.
#[gpui::test]
fn test_button_clickable_logic(_cx: &mut gpui::TestAppContext) {
    let clickable = Button::new("test").on_click(|_, _, _| {});
    assert!(clickable.clickable());

    let disabled = Button::new("test").disabled(true).on_click(|_, _, _| {});
    assert!(!disabled.clickable());

    let loading = Button::new("test").loading(true).on_click(|_, _, _| {});
    assert!(!loading.clickable());
}

/// Catches changing the link, text, ghost, or no-padding classifications, which would give those
/// public variants the wrong spacing and decoration.
#[gpui::test]
fn test_button_variant_methods(_cx: &mut gpui::TestAppContext) {
    assert!(ButtonVariant::Link.is_link());
    assert!(ButtonVariant::Text.is_text());
    assert!(ButtonVariant::Ghost.is_ghost());

    assert!(ButtonVariant::Link.no_padding());
    assert!(ButtonVariant::Text.no_padding());
    assert!(!ButtonVariant::Ghost.no_padding());
}

/// Catches routing an outlined selected danger button through the pressed style, which would fill
/// the control instead of retaining its selected outline.
#[gpui::test]
fn test_outline_selected_uses_outline_active_style(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init);
    let window = cx.add_empty_window();
    window.update(|_, cx| {
        let variant = ButtonVariant::Danger;
        let p = MoonPalette::active(cx);
        let active_style = variant.active(true, cx);
        let selected_style = variant.selected(true, cx);

        assert_eq!(selected_style.bg.a, 0.0);
        assert_eq!(selected_style.border, rgba_from(p.red, 0.40));
        assert_eq!(selected_style.fg, rgba_from(p.red, 1.0));
        assert_ne!(selected_style.bg, active_style.bg);
    });
}

/// Catches changing `MoonButtonMetrics::base_for_size`, which would break the established compact
/// heights, radii, and text spacing used by terminal controls.
#[test]
fn test_moon_button_metrics_match_terminal_palette() {
    let micro = MoonButtonMetrics::base_for_size(Size::XSmall);
    assert_eq!(micro.height, px(18.));
    assert_eq!(micro.radius, px(4.));
    assert_eq!(micro.font_size, px(9.));
    assert_eq!(micro.line_height, px(12.));
    assert_eq!(micro.gap, px(4.));
    assert_eq!(micro.pad_x, px(7.));

    let action = MoonButtonMetrics::base_for_size(Size::Small);
    assert_eq!(action.height, px(26.));
    assert_eq!(action.radius, px(4.));
    assert_eq!(action.font_size, px(10.5));
    assert_eq!(action.line_height, px(14.));
    assert_eq!(action.gap, px(6.));
    assert_eq!(action.pad_x, px(0.));

    let toolbar = MoonButtonMetrics::base_for_size(Size::Medium);
    assert_eq!(toolbar.height, px(28.));
    assert_eq!(toolbar.radius, px(4.));

    let pill = MoonButtonMetrics::base_for_size(Size::Large);
    assert_eq!(pill.height, px(30.));
    assert_eq!(pill.radius, px(15.));
}

/// Catches making `button.rs:resolve_moon` reuse a resting state tuple for hover or active,
/// which would make any public MoonButton variant look inert under the pointer.
#[test]
fn public_moon_button_variants_have_pointer_feedback_in_both_themes() {
    let variants = [
        ButtonVariant::Default,
        ButtonVariant::Panel,
        ButtonVariant::Soft,
        ButtonVariant::Blue,
        ButtonVariant::Amber,
        ButtonVariant::Green,
        ButtonVariant::Red,
        ButtonVariant::Danger,
        ButtonVariant::OutlineAmber,
        ButtonVariant::OutlineRed,
        ButtonVariant::Ghost,
        ButtonVariant::Bare,
    ];

    for p in [MoonPalette::TERMINAL, MoonPalette::LIGHT] {
        for variant in variants {
            let normal = variant.resolve_moon(p, false, ButtonVisualState::Normal);
            let hovered = variant.resolve_moon(p, false, ButtonVisualState::Hovered);
            let active = variant.resolve_moon(p, false, ButtonVisualState::Active);
            let selected = variant.resolve_moon(p, false, ButtonVisualState::Selected);
            let selected_hovered =
                variant.resolve_moon(p, false, ButtonVisualState::SelectedHovered);
            let selected_active = variant.resolve_moon(p, false, ButtonVisualState::SelectedActive);
            let visual = |style: &ButtonVariantStyle| (style.bg, style.border, style.fg);

            assert_ne!(
                visual(&normal),
                visual(&hovered),
                "{variant:?} has no hover feedback (light={})",
                p.is_light()
            );
            assert_ne!(
                visual(&hovered),
                visual(&active),
                "{variant:?} has no pressed feedback (light={})",
                p.is_light()
            );
            assert_ne!(
                visual(&normal),
                visual(&active),
                "{variant:?} has no distinct pressed state (light={})",
                p.is_light()
            );
            assert_ne!(
                visual(&selected),
                visual(&selected_hovered),
                "selected {variant:?} has no hover feedback (light={})",
                p.is_light()
            );
            assert_ne!(
                visual(&selected),
                visual(&selected_active),
                "selected {variant:?} has no distinct pressed state (light={})",
                p.is_light()
            );
            assert_ne!(
                visual(&selected_hovered),
                visual(&selected_active),
                "selected {variant:?} has no pressed feedback (light={})",
                p.is_light()
            );

            if variant.uses_neutral_interaction_surface() {
                assert_eq!(
                    hovered.border,
                    rgba_from(p.border_hover, 1.0),
                    "{variant:?} bypasses the palette hover-border role (light={})",
                    p.is_light()
                );
            }
        }
    }
}

/// Catches removing either guard in `button.rs:pointer_feedback_enabled`, which would make a
/// disabled button or a button showing a loading spinner falsely react to pointer input.
#[test]
fn disabled_and_loading_buttons_suppress_pointer_feedback() {
    assert!(pointer_feedback_enabled(false, false));
    assert!(!pointer_feedback_enabled(true, false));
    assert!(!pointer_feedback_enabled(false, true));
    assert!(!pointer_feedback_enabled(true, true));
}

/// Catches disconnecting `ButtonVariant::moon_style` from the Moon palette, which would render
/// fixed dark colors in Light theme or lose the established semantic tone emphasis.
#[test]
fn test_moon_button_variant_tokens_match_terminal_palette() {
    let p = MoonPalette::TERMINAL;
    let default = ButtonVariant::Default.moon_style(p, false, false).unwrap();
    assert_eq!(default.bg, 0x1F2126);

    let light = MoonPalette::LIGHT;
    let light_default = ButtonVariant::Default
        .moon_style(light, false, false)
        .unwrap();
    assert_eq!(light_default.bg, light.surface);
    assert_eq!(light_default.border, light.border_soft);
    assert_ne!(light_default.bg, 0x1F2126);

    let light_blue = ButtonVariant::Blue.moon_style(light, false, false).unwrap();
    assert_eq!(light_blue.bg, light.accent);
    assert_eq!(light_blue.border, light.accent);

    let light_live = ButtonVariant::Green.moon_style(light, false, true).unwrap();
    assert_eq!(light_live.bg, light.green_btn);
    assert_eq!(light_live.bg_alpha, 1.0);
    assert_eq!(light_live.fg, light.on_accent);

    let blue = ButtonVariant::Blue.moon_style(p, false, false).unwrap();
    assert_eq!(blue.bg, p.blue);
    assert_eq!(blue.bg_alpha, 0.10);
    assert_eq!(blue.border_alpha, 0.22);
    assert_eq!(blue.hover_bg_alpha, 0.18);

    let selected_blue = ButtonVariant::Blue.moon_style(p, false, true).unwrap();
    assert_eq!(selected_blue.bg_alpha, 0.18);
    assert_eq!(selected_blue.border_alpha, 0.38);

    let outline_red = ButtonVariant::OutlineRed
        .moon_style(p, false, false)
        .unwrap();
    assert_eq!(outline_red.bg_alpha, 0.0);
    assert_eq!(outline_red.border, p.red);
    assert_eq!(outline_red.fg, p.red);

    let soft = ButtonVariant::Soft.moon_style(p, false, false).unwrap();
    assert_eq!(soft.bg, 0xFFFFFF);
    assert_eq!(soft.bg_alpha, 0.02);
    assert_eq!(soft.hover_bg_alpha, 0.055);
}

// --- Facet skin -----------------------------------------------------------------------------

/// Every visual state a button resolves.
const ALL_STATES: [ButtonVisualState; 7] = [
    ButtonVisualState::Normal,
    ButtonVisualState::Hovered,
    ButtonVisualState::Active,
    ButtonVisualState::Selected,
    ButtonVisualState::SelectedHovered,
    ButtonVisualState::SelectedActive,
    ButtonVisualState::Disabled,
];

/// The neutral variants, which the Facet skin draws as raised controls.
const RAISED_VARIANTS: [ButtonVariant; 4] = [
    ButtonVariant::Default,
    ButtonVariant::Panel,
    ButtonVariant::Secondary,
    ButtonVariant::Soft,
];

/// The built-in variants the Facet skin leaves flat.
const FLAT_ONLY_VARIANTS: [ButtonVariant; 6] = [
    ButtonVariant::Ghost,
    ButtonVariant::Bare,
    ButtonVariant::Link,
    ButtonVariant::Text,
    ButtonVariant::OutlineAmber,
    ButtonVariant::OutlineRed,
];

/// The tone variants, each with the palette entry that fills it on a dark palette.
fn dark_tone_variants(p: MoonPalette) -> [(ButtonVariant, u32); 9] {
    [
        (ButtonVariant::Primary, p.blue),
        (ButtonVariant::Blue, p.blue),
        (ButtonVariant::Info, p.blue),
        (ButtonVariant::Warning, p.amber),
        (ButtonVariant::Amber, p.amber),
        (ButtonVariant::Success, p.green),
        (ButtonVariant::Green, p.green),
        (ButtonVariant::Danger, p.red),
        (ButtonVariant::Red, p.red),
    ]
}

/// Theme tokens painting `palette` with `skin` at the reference scale.
fn tokens_with(palette: MoonPalette, skin: MoonSkin) -> MoonThemeTokens {
    MoonThemeTokens {
        skin,
        palette,
        ..MoonThemeTokens::default()
    }
}

/// An opaque gradient from `top` at the top edge to `bottom` at the bottom edge, written out
/// here rather than taken from the code under test.
fn top_to_bottom(top: u32, bottom: u32) -> Background {
    linear_gradient(
        180.0,
        linear_color_stop(rgba_from(top, 1.0), 0.0),
        linear_color_stop(rgba_from(bottom, 1.0), 1.0),
    )
}

/// A shadow with no horizontal offset and no spread, written out here rather than taken from
/// the code under test. `design_blur` is the blur as the design gives it, a CSS `box-shadow`
/// blur.
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

/// Keep only the shadows drawn inside a surface.
fn inset_only(surface: &MoonFacet) -> Vec<BoxShadow> {
    surface
        .shadows
        .iter()
        .filter(|shadow| shadow.inset)
        .cloned()
        .collect()
}

/// One variant family's flat paint on the dark terminal palette, as it was before the Facet
/// skin existed. Every pair is `(0xRRGGBB, alpha)`.
struct FlatFamily {
    variants: &'static [ButtonVariant],
    /// The fill at rest, under the pointer, pressed and selected.
    fills: [(u32, f32); 4],
    /// The border in the same four states.
    borders: [(u32, f32); 4],
    /// The ink at rest, under the pointer and pressed.
    ink: (u32, f32),
    /// The ink when selected.
    selected_ink: (u32, f32),
}

/// The flat paint of every variant the Facet skin restyles, on the dark terminal palette `p`.
///
/// The alphas, and the two colours that are not palette entries, are copied from the flat
/// design rather than read back from `ButtonVariant::moon_style`.
fn flat_dark_families(p: MoonPalette) -> Vec<FlatFamily> {
    const DEFAULT_FILL: u32 = 0x1F2126;
    const WHITE: u32 = 0xFFFFFF;
    let neutral_hover = (p.overlay, 0.10);
    let neutral_pressed = (p.overlay, 0.06);
    let hover_border = (p.border_hover, 1.0);
    let tone = |color: u32, fills: [f32; 4], borders: [f32; 4]| {
        (
            fills.map(|alpha| (color, alpha)),
            borders.map(|alpha| (color, alpha)),
        )
    };
    let (blue_fills, blue_borders) =
        tone(p.blue, [0.10, 0.18, 0.12, 0.18], [0.22, 0.42, 0.30, 0.38]);
    let (amber_fills, amber_borders) =
        tone(p.amber, [0.10, 0.18, 0.12, 0.18], [0.22, 0.42, 0.30, 0.38]);
    let (green_fills, green_borders) =
        tone(p.green, [0.14, 0.22, 0.14, 0.22], [0.30, 0.44, 0.34, 0.30]);
    let (red_fills, red_borders) = tone(p.red, [0.10, 0.22, 0.14, 0.10], [0.38, 0.48, 0.36, 0.38]);
    let (danger_fills, danger_borders) =
        tone(p.red, [0.14, 0.22, 0.14, 0.14], [0.38, 0.48, 0.36, 0.38]);

    vec![
        FlatFamily {
            variants: &[ButtonVariant::Default],
            fills: [
                (DEFAULT_FILL, 1.0),
                neutral_hover,
                neutral_pressed,
                (p.accent, p.accent_tint_a),
            ],
            borders: [(p.border, 1.0), hover_border, hover_border, (p.accent, 1.0)],
            ink: (p.text, 0.86),
            selected_ink: (p.accent_fg, 1.0),
        },
        FlatFamily {
            variants: &[ButtonVariant::Panel],
            fills: [
                (p.shell_high, 1.0),
                neutral_hover,
                neutral_pressed,
                (p.shell_high, 1.0),
            ],
            borders: [(p.border, 1.0), hover_border, hover_border, (p.border, 1.0)],
            ink: (p.text, 1.0),
            selected_ink: (p.text, 1.0),
        },
        FlatFamily {
            variants: &[ButtonVariant::Secondary, ButtonVariant::Soft],
            fills: [(WHITE, 0.02), neutral_hover, neutral_pressed, (WHITE, 0.02)],
            borders: [(WHITE, 0.05), hover_border, hover_border, (WHITE, 0.05)],
            ink: (p.text_soft, 1.0),
            selected_ink: (p.text_soft, 1.0),
        },
        FlatFamily {
            variants: &[
                ButtonVariant::Primary,
                ButtonVariant::Blue,
                ButtonVariant::Info,
            ],
            fills: blue_fills,
            borders: blue_borders,
            ink: (p.blue, 1.0),
            selected_ink: (p.blue, 1.0),
        },
        FlatFamily {
            variants: &[ButtonVariant::Warning, ButtonVariant::Amber],
            fills: amber_fills,
            borders: amber_borders,
            ink: (p.amber, 1.0),
            selected_ink: (p.amber, 1.0),
        },
        FlatFamily {
            variants: &[ButtonVariant::Success, ButtonVariant::Green],
            fills: green_fills,
            borders: green_borders,
            ink: (p.green, 1.0),
            selected_ink: (p.green, 1.0),
        },
        FlatFamily {
            variants: &[ButtonVariant::Red],
            fills: red_fills,
            borders: red_borders,
            ink: (p.red, 1.0),
            selected_ink: (p.red, 1.0),
        },
        FlatFamily {
            variants: &[ButtonVariant::Danger],
            fills: danger_fills,
            borders: danger_borders,
            ink: (p.red, 1.0),
            selected_ink: (p.red, 1.0),
        },
    ]
}

/// One variant family's flat resting paint on the light palette, as it was before the Facet
/// skin existed. Every pair is `(0xRRGGBB, alpha)`.
struct FlatLightFamily {
    variants: &'static [ButtonVariant],
    fill: (u32, f32),
    border: (u32, f32),
    ink: (u32, f32),
}

/// The flat resting paint of every variant the Facet skin restyles, on the light palette `p`,
/// with the alphas copied from the flat design.
fn flat_light_families(p: MoonPalette) -> Vec<FlatLightFamily> {
    vec![
        FlatLightFamily {
            variants: &[ButtonVariant::Default],
            fill: (p.surface, 1.0),
            border: (p.border_soft, 1.0),
            ink: (p.text, 0.86),
        },
        FlatLightFamily {
            variants: &[ButtonVariant::Panel],
            fill: (p.surface, 1.0),
            border: (p.border_soft, 1.0),
            ink: (p.text, 1.0),
        },
        FlatLightFamily {
            variants: &[ButtonVariant::Secondary, ButtonVariant::Soft],
            fill: (p.surface, 1.0),
            border: (p.border_soft, 1.0),
            ink: (p.text_soft, 1.0),
        },
        FlatLightFamily {
            variants: &[
                ButtonVariant::Primary,
                ButtonVariant::Blue,
                ButtonVariant::Info,
            ],
            fill: (p.accent, 0.10),
            border: (p.accent, 0.22),
            ink: (p.accent, 1.0),
        },
        FlatLightFamily {
            variants: &[ButtonVariant::Warning, ButtonVariant::Amber],
            fill: (p.amber, 0.10),
            border: (p.amber, 0.22),
            ink: (p.amber, 1.0),
        },
        FlatLightFamily {
            variants: &[ButtonVariant::Success, ButtonVariant::Green],
            fill: (p.green_btn, 0.14),
            border: (p.green_btn, 0.30),
            ink: (p.green_text, 1.0),
        },
        FlatLightFamily {
            variants: &[ButtonVariant::Red],
            fill: (p.red, 0.10),
            border: (p.red, 0.38),
            ink: (p.red_text, 1.0),
        },
        FlatLightFamily {
            variants: &[ButtonVariant::Danger],
            fill: (p.red, 0.14),
            border: (p.red, 0.38),
            ink: (p.red_text, 1.0),
        },
    ]
}

/// Catches the Facet skin leaking into a flat theme: `button.rs:resolve_facet` answering for
/// the Flat skin, or a restyled variant's flat colours drifting while the facet was added.
/// Either one repaints every button of every existing theme.
#[gpui::test]
fn flat_theme_buttons_keep_their_colours_and_resolve_no_facet(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init);
    cx.update(|cx| MoonTheme::install_config(MoonThemeConfig::default(), cx));
    let paint = |(color, alpha): (u32, f32)| rgba_from(color, alpha);

    cx.update(|cx| {
        assert_eq!(MoonSkin::active(cx), MoonSkin::Flat);
        let tokens = MoonTheme::active_tokens(cx);
        let p = tokens.palette;
        assert!(!p.is_light(), "the default theme starts on its dark side");

        for family in flat_dark_families(p) {
            for variant in family.variants {
                let resolved = [
                    ("resting", variant.normal(false, cx)),
                    ("hovered", variant.hovered(false, cx)),
                    ("pressed", variant.active(false, cx)),
                    ("selected", variant.selected(false, cx)),
                ];
                for (ix, (state, style)) in resolved.iter().enumerate() {
                    let ink = if ix == 3 {
                        family.selected_ink
                    } else {
                        family.ink
                    };
                    assert_eq!(
                        (style.bg, style.border, style.fg),
                        (
                            paint(family.fills[ix]),
                            paint(family.borders[ix]),
                            paint(ink)
                        ),
                        "flat {state} {variant:?} drifted on the dark palette"
                    );
                }
                for state in ALL_STATES {
                    assert_eq!(
                        variant.resolve_facet(&tokens, false, state),
                        None,
                        "{variant:?} {state:?} paints a facet under the Flat skin"
                    );
                }
            }
        }
    });

    cx.update(|cx| MoonTheme::set_mode(ThemeMode::Light, cx));
    cx.update(|cx| {
        assert_eq!(MoonSkin::active(cx), MoonSkin::Flat);
        let tokens = MoonTheme::active_tokens(cx);
        let p = tokens.palette;
        assert!(p.is_light());

        for family in flat_light_families(p) {
            for variant in family.variants {
                let style = variant.normal(false, cx);
                assert_eq!(
                    (style.bg, style.border, style.fg),
                    (paint(family.fill), paint(family.border), paint(family.ink)),
                    "flat resting {variant:?} drifted on the light palette"
                );
                for state in ALL_STATES {
                    assert_eq!(
                        variant.resolve_facet(&tokens, false, state),
                        None,
                        "{variant:?} {state:?} paints a facet under the Flat skin"
                    );
                }
            }
        }
    });
}

/// Catches `button.rs:resolve_facet` losing the raised surface of a neutral variant on the
/// faceted theme, swapping its hover and resting surfaces, or pressing it without turning the
/// light: the Obsidian theme would keep flat slabs for buttons, or they would not move under
/// the pointer.
#[gpui::test]
fn faceted_theme_raises_the_neutral_variants(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init);
    cx.update(|cx| MoonTheme::install_config(MoonThemeConfig::moon_obsidian(), cx));

    cx.update(|cx| {
        let tokens = MoonTheme::active_tokens(cx);
        let p = tokens.palette;
        assert_eq!(tokens.skin, MoonSkin::Facet);

        for variant in RAISED_VARIANTS {
            let rest = variant
                .resolve_facet(&tokens, false, ButtonVisualState::Normal)
                .unwrap_or_else(|| panic!("{variant:?} has no resting facet"));
            assert_eq!(
                rest.fill.as_solid(),
                None,
                "{variant:?} must rest on a gradient, not a solid fill"
            );
            assert_eq!(rest.fill, top_to_bottom(p.panel_high, p.panel));
            assert!(
                !rest.shadows.is_empty(),
                "{variant:?} must rest with a shadow stack"
            );
            assert_eq!(rest, facet(MoonFacetKind::Raised, &tokens));

            assert_eq!(
                variant.resolve_facet(&tokens, false, ButtonVisualState::Hovered),
                Some(facet(MoonFacetKind::RaisedHover, &tokens)),
                "{variant:?} under the pointer"
            );

            let pressed = variant
                .resolve_facet(&tokens, false, ButtonVisualState::Active)
                .unwrap_or_else(|| panic!("{variant:?} has no pressed facet"));
            assert_eq!(
                pressed.fill,
                top_to_bottom(p.panel, p.panel_high),
                "pressed {variant:?} is lit from below: the resting ends swapped"
            );
            assert_eq!(pressed.edge, rest.edge);
            assert_eq!(
                pressed.shadows,
                vec![shadow(1.0, 0.0, rgba_from(p.overlay, 0.16), true)],
                "pressed {variant:?} keeps its highlight and casts no drop shadow"
            );
            assert_eq!(pressed, raised_pressed_facet(&tokens));
        }

        // Selection does not restyle a panel or soft button under the Flat skin either.
        for variant in [
            ButtonVariant::Panel,
            ButtonVariant::Secondary,
            ButtonVariant::Soft,
        ] {
            assert_eq!(
                variant.resolve_facet(&tokens, false, ButtonVisualState::Selected),
                Some(facet(MoonFacetKind::Raised, &tokens)),
                "selected {variant:?}"
            );
        }
    });

    cx.update(|cx| MoonTheme::set_mode(ThemeMode::Light, cx));
    cx.update(|cx| {
        let tokens = MoonTheme::active_tokens(cx);
        for variant in RAISED_VARIANTS {
            for state in ALL_STATES {
                assert_eq!(
                    variant.resolve_facet(&tokens, false, state),
                    None,
                    "the light side of Obsidian is flat: {variant:?} {state:?}"
                );
            }
        }
    });
}

/// Catches `button.rs:resolve_facet` washing a tone variant with the wrong tone or the wrong
/// strength: a danger button tinted like a neutral one, or a hovered or selected button that
/// does not light up.
#[test]
fn faceted_tone_variants_wash_soft_at_rest_and_light_up_when_hovered_or_selected() {
    let p = MoonPalette::OBSIDIAN;
    let tokens = tokens_with(p, MoonSkin::Facet);

    for (variant, tone) in dark_tone_variants(p) {
        let at = |state: ButtonVisualState| {
            variant
                .resolve_facet(&tokens, false, state)
                .unwrap_or_else(|| panic!("{variant:?} {state:?} has no facet"))
        };
        let soft = facet_tinted(tone, MoonFacetTint::Soft, &tokens);
        let lit = facet_tinted(tone, MoonFacetTint::Selected, &tokens);

        assert_eq!(at(ButtonVisualState::Normal), soft, "resting {variant:?}");
        assert_eq!(at(ButtonVisualState::Hovered), lit, "hovered {variant:?}");
        assert_eq!(at(ButtonVisualState::Selected), lit, "selected {variant:?}");

        let selected_hover = at(ButtonVisualState::SelectedHovered);
        assert_eq!(selected_hover.edge, rgba_from(tone, 1.0));
        assert_eq!(
            (selected_hover.fill, &selected_hover.shadows),
            (lit.fill, &lit.shadows),
            "a selected {variant:?} under the pointer changes its edge only"
        );

        for state in [ButtonVisualState::Active, ButtonVisualState::SelectedActive] {
            let pressed = at(state);
            assert_eq!((pressed.fill, pressed.edge), (lit.fill, lit.edge));
            assert_eq!(
                pressed.shadows,
                inset_only(&lit),
                "{state:?} {variant:?} drops its drop shadow and glow"
            );
            assert!(pressed.shadows.len() < lit.shadows.len());
        }

        assert_eq!(
            variant.resolve_facet(&tokens, false, ButtonVisualState::Disabled),
            None,
            "a disabled {variant:?} keeps the flat disabled look"
        );
        for state in ALL_STATES {
            assert_eq!(
                variant.resolve_facet(&tokens, true, state),
                None,
                "an outlined {variant:?} keeps its flat outline in {state:?}"
            );
        }
    }
}

/// Catches a selected neutral button falling back to the raised surface in
/// `button.rs:resolve_facet`, which would hide which toolbar toggle is on, and catches its
/// ink changing with the skin.
#[test]
fn a_selected_neutral_button_is_an_accent_facet_with_its_flat_ink() {
    let p = MoonPalette::OBSIDIAN;
    let tokens = tokens_with(p, MoonSkin::Facet);
    let variant = ButtonVariant::Default;

    assert_eq!(
        variant.resolve_facet(&tokens, false, ButtonVisualState::Selected),
        Some(facet_tinted(p.accent, MoonFacetTint::Selected, &tokens))
    );
    assert_eq!(
        variant
            .resolve_moon(p, false, ButtonVisualState::Selected)
            .fg,
        rgba_from(p.accent_fg, 1.0),
        "the selected ink is the flat one"
    );
}

/// Catches a variant with no surface of its own picking one up in `button.rs:resolve_facet`:
/// a ghost toolbar button or a text link drawn as a raised slab on the faceted theme.
#[test]
fn ghost_bare_link_text_and_outline_variants_stay_flat_under_the_facet_skin() {
    for palette in [MoonPalette::OBSIDIAN, MoonPalette::LIGHT] {
        let tokens = tokens_with(palette, MoonSkin::Facet);
        for variant in FLAT_ONLY_VARIANTS {
            for outline in [false, true] {
                for state in ALL_STATES {
                    assert_eq!(
                        variant.resolve_facet(&tokens, outline, state),
                        None,
                        "{variant:?} {state:?} (outline={outline}, light={})",
                        palette.is_light()
                    );
                }
            }
        }
    }
}

/// Catches a faceted state resolving to the surface of the state before it in
/// `button.rs:resolve_facet`, which would make a faceted button look inert under the pointer
/// or under a press, on a dark palette or a light one.
#[test]
fn faceted_buttons_keep_pointer_feedback_on_dark_and_light_palettes() {
    for palette in [MoonPalette::OBSIDIAN, MoonPalette::LIGHT] {
        let tokens = tokens_with(palette, MoonSkin::Facet);
        let tones = dark_tone_variants(palette).map(|(variant, _)| variant);
        for variant in RAISED_VARIANTS.into_iter().chain(tones) {
            let at = |state: ButtonVisualState| {
                variant
                    .resolve_facet(&tokens, false, state)
                    .unwrap_or_else(|| panic!("{variant:?} {state:?} has no facet"))
            };
            let light = palette.is_light();

            assert_ne!(
                at(ButtonVisualState::Normal),
                at(ButtonVisualState::Hovered),
                "{variant:?} has no hover feedback (light={light})"
            );
            assert_ne!(
                at(ButtonVisualState::Hovered),
                at(ButtonVisualState::Active),
                "{variant:?} has no pressed feedback (light={light})"
            );
            assert_ne!(
                at(ButtonVisualState::Normal),
                at(ButtonVisualState::Active),
                "{variant:?} has no distinct pressed state (light={light})"
            );
            assert_ne!(
                at(ButtonVisualState::Selected),
                at(ButtonVisualState::SelectedHovered),
                "selected {variant:?} has no hover feedback (light={light})"
            );
            assert_ne!(
                at(ButtonVisualState::Selected),
                at(ButtonVisualState::SelectedActive),
                "selected {variant:?} has no distinct pressed state (light={light})"
            );
            assert_ne!(
                at(ButtonVisualState::SelectedHovered),
                at(ButtonVisualState::SelectedActive),
                "selected {variant:?} has no pressed feedback (light={light})"
            );
        }
    }
}

/// Catches `button.rs:resolve_facet` washing a tone variant whose flat fill is opaque: the
/// light palette's selected green button carries white ink for a solid green fill, and on a
/// pale green wash that label would be unreadable.
#[test]
fn an_opaque_tone_fill_becomes_a_solid_facet() {
    let p = MoonPalette::LIGHT;
    let tokens = tokens_with(p, MoonSkin::Facet);
    let variant = ButtonVariant::Green;

    assert_eq!(
        variant.resolve_facet(&tokens, false, ButtonVisualState::Normal),
        Some(facet_tinted(p.green_btn, MoonFacetTint::Soft, &tokens)),
        "the translucent resting fill stays a wash"
    );
    assert_eq!(
        variant.resolve_facet(&tokens, false, ButtonVisualState::Selected),
        Some(facet_tinted(p.green_btn, MoonFacetTint::Solid, &tokens))
    );
    assert_eq!(
        variant
            .resolve_moon(p, false, ButtonVisualState::Selected)
            .fg,
        rgba_from(p.on_accent, 1.0),
        "the ink this fill has to carry"
    );
}

/// Catches a custom variant's facet tone being ignored, or honoured under the Flat skin, in
/// `button.rs:resolve_facet`: the solid primary button would stay a flat accent slab on the
/// faceted theme, or would change on every flat one. Also catches a custom variant with no
/// tone picking a facet up.
#[gpui::test]
fn a_custom_variant_is_a_solid_facet_only_with_a_tone_and_only_under_the_facet_skin(
    cx: &mut gpui::TestAppContext,
) {
    cx.update(crate::init);
    cx.update(|cx| MoonTheme::install_config(MoonThemeConfig::default(), cx));

    cx.update(|cx| {
        let p = MoonPalette::OBSIDIAN;
        let tone = p.accent;
        let fill = rgba_from(tone, 1.0);
        let hover = rgba_from(tone, 0.9);
        let active = rgba_from(tone, 0.8);
        let label = rgba_from(p.shell, 1.0);
        let plain = ButtonCustomVariant::new(cx)
            .color(fill)
            .hover(hover)
            .active(active)
            .foreground(label);
        let toned = ButtonVariant::Custom(plain.facet_solid(tone));
        let plain = ButtonVariant::Custom(plain);

        // Under the Flat skin the tone changes nothing: the flat colours are the ones set.
        let flat = MoonTheme::active_tokens(cx);
        assert_eq!(flat.skin, MoonSkin::Flat);
        for (state, expected) in [
            (ButtonVisualState::Normal, fill),
            (ButtonVisualState::Hovered, hover),
            (ButtonVisualState::Active, active),
            (ButtonVisualState::Selected, active),
        ] {
            let style = toned.resolve(false, state, cx);
            assert_eq!((style.bg, style.fg), (expected, label), "flat {state:?}");
            assert_eq!(toned.resolve_facet(&flat, false, state), None);
        }

        let tokens = tokens_with(p, MoonSkin::Facet);
        let solid = facet_tinted(tone, MoonFacetTint::Solid, &tokens);
        assert_eq!(
            toned.resolve_facet(&tokens, false, ButtonVisualState::Normal),
            Some(solid.clone())
        );

        let hovered = toned
            .resolve_facet(&tokens, false, ButtonVisualState::Hovered)
            .expect("a hovered solid button has a facet");
        let mut with_second_glow = solid.shadows.clone();
        with_second_glow.push(shadow(0.0, 18.0, rgba_from(tone, 0.26), false));
        assert_eq!((hovered.fill, hovered.edge), (solid.fill, solid.edge));
        assert_eq!(hovered.shadows, with_second_glow);

        // Like its flat colours, a custom variant draws every selected state as the pressed one.
        for state in [
            ButtonVisualState::Active,
            ButtonVisualState::Selected,
            ButtonVisualState::SelectedHovered,
            ButtonVisualState::SelectedActive,
        ] {
            let pressed = toned
                .resolve_facet(&tokens, false, state)
                .unwrap_or_else(|| panic!("solid {state:?} has no facet"));
            assert_eq!((pressed.fill, pressed.edge), (solid.fill, solid.edge));
            assert_eq!(pressed.shadows, inset_only(&solid), "solid {state:?}");
            assert!(pressed.shadows.len() < solid.shadows.len());
        }
        assert_eq!(
            toned.resolve_facet(&tokens, false, ButtonVisualState::Disabled),
            None
        );

        for state in ALL_STATES {
            assert_eq!(
                plain.resolve_facet(&tokens, false, state),
                None,
                "a custom variant with no tone keeps its own colours in {state:?}"
            );
        }
    });
}
