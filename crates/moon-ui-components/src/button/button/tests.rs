//! Unit coverage for base button builders, interaction states, and Moon sizing.

use super::{
    Button, ButtonRounded, ButtonVariant, ButtonVariantStyle, ButtonVariants, ButtonVisualState,
    MoonButtonMetrics, pointer_feedback_enabled,
};
use crate::{
    Disableable as _, Selectable as _, Sizable as _, Size,
    moon::{MoonPalette, rgba_from},
};
use gpui::px;

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
