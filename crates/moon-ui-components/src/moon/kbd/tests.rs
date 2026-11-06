//! Regression coverage for MoonKbd geometry and platform shortcut formatting.

use super::{MoonKbd, MoonKbdSize};
use gpui::Keystroke;

/// Catches conflating forward-delete and backspace labels, making distinct bindings look alike.
#[test]
fn kbd_distinguishes_delete_and_backspace() {
    #[cfg(target_os = "macos")]
    let cases = [("delete", "⌦"), ("backspace", "⌫"), ("shift-delete", "⇧⌦")];
    #[cfg(not(target_os = "macos"))]
    let cases = [
        ("delete", "Delete"),
        ("backspace", "Backspace"),
        ("shift-delete", "Shift+Delete"),
    ];
    for (source, expected) in cases {
        assert_eq!(
            MoonKbd::format_keystroke(&Keystroke::parse(source).unwrap()),
            expected
        );
    }
}

/// Catches using pointer-target height or legacy font scaling, which enlarges shortcut chips.
#[test]
fn kbd_metrics_match_designer_reference() {
    use crate::moon::{MoonSize, MoonThemeTokens};
    let mut tokens = MoonThemeTokens::default();
    tokens.scale.ui = 1.5;
    tokens.scale.font = 3.0;
    tokens.scale.font_delta = 6.0;
    for (tier, height, font, radius, pad) in [
        (MoonSize::Xs, 24.0, 16.5, 4.0, 6.0),
        (MoonSize::Sm, 30.0, 18.0, 4.0, 8.0),
        (MoonSize::Md, 36.0, 21.0, 6.0, 12.0),
        (MoonSize::Lg, 36.0, 21.0, 6.0, 12.0),
        (MoonSize::Xl, 36.0, 21.0, 6.0, 12.0),
        (MoonSize::Xxl, 36.0, 21.0, 6.0, 12.0),
    ] {
        let metrics = MoonKbd::new("Esc").size(tier.into()).metrics(&tokens);
        assert_eq!(
            (metrics.height, metrics.line_height, metrics.font_size),
            (height, height, font)
        );
        assert_eq!((metrics.radius, metrics.pad_x), (radius, pad));
        tokens.scale.tier = tier;
        assert_eq!(MoonKbd::new("density").metrics(&tokens).height, height);
    }
}

/// Catches ignoring explicit tiers or legacy Custom text scaling, resizing pinned shortcut chips.
#[test]
fn kbd_tiers_and_custom_scaling_survive() {
    use crate::moon::{MoonSize, MoonThemeTokens};
    let mut tokens = MoonThemeTokens::default();
    tokens.scale.ui = 2.0;
    tokens.scale.font = 3.0;
    tokens.scale.font_delta = 6.0;
    assert_eq!(
        MoonKbd::new("Esc")
            .size(MoonKbdSize::Tier(MoonSize::Xs))
            .metrics(&tokens)
            .height,
        32.0
    );
    assert_eq!(
        MoonKbd::new("Esc")
            .size(MoonKbdSize::Tier(MoonSize::Sm))
            .metrics(&tokens)
            .height,
        40.0
    );
    let metrics = MoonKbd::new("Esc")
        .size(MoonKbdSize::Custom {
            height: 20.0,
            font_size: 10.0,
            line_height: 12.0,
            radius: 4.0,
            pad_x: 6.0,
        })
        .metrics(&tokens);
    assert_eq!(
        (metrics.height, metrics.font_size, metrics.line_height),
        (40.0, 36.0, 42.0)
    );
}

/// Catches changing modifier ordering, separators, or special-key labels in
/// `kbd.rs:MoonKbd::format_keystroke`, which would make displayed shortcuts diverge from the
/// platform-specific Longbridge convention.
#[test]
fn kbd_formats_keystrokes_like_longbridge() {
    #[cfg(target_os = "macos")]
    {
        assert_eq!(
            MoonKbd::format_keystroke(&Keystroke::parse("cmd-enter").unwrap()),
            "⌘⏎"
        );
        assert_eq!(
            MoonKbd::format_keystroke(&Keystroke::parse("cmd-ctrl-shift-a").unwrap()),
            "⌃⇧⌘A"
        );
    }

    #[cfg(not(target_os = "macos"))]
    {
        assert_eq!(
            MoonKbd::format_keystroke(&Keystroke::parse("ctrl-a").unwrap()),
            "Ctrl+A"
        );
        assert_eq!(
            MoonKbd::format_keystroke(&Keystroke::parse("ctrl-alt-shift-a").unwrap()),
            "Ctrl+Alt+Shift+A"
        );
    }
}

/// Catches `kbd.rs:MoonKbd::raised_surface` answering for the Flat skin, which would restyle
/// the shortcut chips of every existing theme, losing the raised surface of a filled chip on
/// the faceted theme, or lighting an outline chip, which has no fill.
#[test]
fn only_a_filled_chip_under_the_facet_skin_is_raised() {
    use crate::moon::{MoonFacetKind, MoonPalette, MoonSkin, MoonThemeTokens, facet, rgba_from};
    use gpui::{linear_color_stop, linear_gradient};

    for palette in [MoonPalette::OBSIDIAN, MoonPalette::LIGHT] {
        let tokens = |skin: MoonSkin| MoonThemeTokens {
            skin,
            palette,
            ..MoonThemeTokens::default()
        };
        let faceted = tokens(MoonSkin::Facet);

        let raised = MoonKbd::new("Esc")
            .raised_surface(&faceted)
            .expect("a filled chip is raised under the Facet skin");
        assert_eq!(
            raised.fill,
            linear_gradient(
                180.0,
                linear_color_stop(rgba_from(palette.panel_high, 1.0), 0.0),
                linear_color_stop(rgba_from(palette.panel, 1.0), 1.0),
            ),
            "the chip is lit from above"
        );
        assert!(!raised.shadows.is_empty());
        assert_eq!(raised, facet(MoonFacetKind::Raised, &faceted));

        assert_eq!(
            MoonKbd::new("Esc").outline(true).raised_surface(&faceted),
            None,
            "an outline chip keeps its hairline"
        );
        for outline in [false, true] {
            assert_eq!(
                MoonKbd::new("Esc")
                    .outline(outline)
                    .raised_surface(&tokens(MoonSkin::Flat)),
                None,
                "a chip is raised under the Flat skin (outline={outline})"
            );
        }
    }
}

/// Catches `kbd.rs:MoonKbd::render` painting anything new on a flat theme, not painting the
/// raised surface it resolves on the faceted one, or lighting an outline chip. The flat colours
/// are the flat design's own: the panel at 92% under a 72% border, and an unfilled chip with a
/// solid border for the outline.
#[test]
fn a_chip_paints_flat_on_a_flat_theme_and_raised_on_the_faceted_one() {
    use super::super::button::scene_probe::{Painted, Pointer, painted};
    use crate::moon::{
        MoonFacetKind, MoonPalette, MoonSkin, MoonThemeConfig, MoonThemeTokens, facet, rgba_from,
    };

    let p = MoonPalette::OBSIDIAN;
    let obsidian_with = |skin: MoonSkin| {
        let mut config = MoonThemeConfig::moon_obsidian();
        config.dark.skin = skin;
        config
    };
    let chip = |skin: MoonSkin, outline: bool| {
        painted(obsidian_with(skin), Pointer::Away, move || {
            MoonKbd::new("Esc").outline(outline)
        })
    };
    let flat_filled = Painted::flat(rgba_from(p.panel, 0.92), rgba_from(p.border, 0.72));
    let flat_outline = Painted::flat(rgba_from(p.shell, 0.0), rgba_from(p.border, 1.0));
    assert_eq!(flat_outline.fill, None, "an outline chip has no fill");

    assert_eq!(chip(MoonSkin::Flat, false), flat_filled);
    assert_eq!(chip(MoonSkin::Flat, true), flat_outline);

    let tokens = MoonThemeTokens {
        skin: MoonSkin::Facet,
        palette: p,
        ..MoonThemeTokens::default()
    };
    assert_eq!(
        chip(MoonSkin::Facet, false),
        Painted::facet(facet(MoonFacetKind::Raised, &tokens))
    );
    assert_eq!(chip(MoonSkin::Facet, true), flat_outline);
}

/// Catches dropping the modifier-only and Caps Lock labels from `kbd.rs:MoonKbd::format_keystroke`.
///
/// A shortcut that is one modifier parses back under GPUI's own name for it, so without these the
/// hotkey field shows "Control", "Platform" and "Capslock" — three labels no keyboard is printed
/// with, and the last one wrong for the key it names.
#[test]
fn kbd_labels_modifier_only_and_capslock_shortcuts() {
    #[cfg(target_os = "macos")]
    {
        assert_eq!(
            MoonKbd::format_keystroke(&Keystroke::parse("ctrl").unwrap()),
            "⌃"
        );
        assert_eq!(
            MoonKbd::format_keystroke(&Keystroke::parse("capslock").unwrap()),
            "⇪"
        );
    }
    #[cfg(not(target_os = "macos"))]
    {
        // `Keystroke::parse` reports a lone Control as the key "control", not as the modifier.
        assert_eq!(
            MoonKbd::format_keystroke(&Keystroke::parse("ctrl").unwrap()),
            "Ctrl"
        );
        assert_eq!(
            MoonKbd::format_keystroke(&Keystroke::parse("alt").unwrap()),
            "Alt"
        );
        assert_eq!(
            MoonKbd::format_keystroke(&Keystroke::parse("cmd").unwrap()),
            "Win"
        );
        assert_eq!(
            MoonKbd::format_keystroke(&Keystroke::parse("capslock").unwrap()),
            "Caps Lock"
        );
        assert_eq!(
            MoonKbd::format_keystroke(&Keystroke::parse("ctrl-capslock").unwrap()),
            "Ctrl+Caps Lock"
        );
    }
}
