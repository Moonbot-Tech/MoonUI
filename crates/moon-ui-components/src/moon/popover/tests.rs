//! Regression coverage for MoonPopover width ownership.

use super::{MoonPopover, MoonPopoverWidth, POPOVER_BORDER, POPOVER_PADDING};
use crate::moon::{MoonPalette, MoonScale, MoonTheme, MoonThemeTokens};
use gpui::{
    Context, InteractiveElement as _, IntoElement, Render, Styled as _, VisualTestContext, Window,
    div, px,
};

/// `popover.rs:MoonPopoverWidth::resolve` must apply UI and font scaling independently while
/// reserving the popup's own padding and border. Replacing either content policy with a raw outer
/// width clips fixed content as soon as the corresponding scale differs from 1.0.
#[test]
fn content_width_policies_reserve_scaled_popup_chrome() {
    for palette in [MoonPalette::TERMINAL, MoonPalette::LIGHT] {
        for (ui, font, font_delta) in [(0.5, 1.75, 0.0), (2.5, 0.75, 4.0)] {
            let mut tokens = MoonThemeTokens {
                palette,
                ..MoonThemeTokens::default()
            };
            tokens.scale = MoonScale {
                ui,
                font,
                font_delta,
            };
            let chrome = tokens.ui(POPOVER_PADDING) * 2.0 + POPOVER_BORDER * 2.0;

            assert_eq!(
                MoonPopoverWidth::UiContent(240.0).resolve(&tokens),
                Some(tokens.ui(240.0) + chrome)
            );
            assert_eq!(
                MoonPopoverWidth::FontContent(240.0).resolve(&tokens),
                Some(tokens.font_width(240.0) + chrome)
            );
            assert_eq!(MoonPopoverWidth::Intrinsic.resolve(&tokens), None);
        }
    }
}

/// Root view containing an always-open intrinsic popover with a fixed-width child.
struct IntrinsicPopoverHarness;

impl Render for IntrinsicPopoverHarness {
    /// Render the geometry probe.
    ///
    /// Args:
    ///     _window: Test window.
    ///     _cx: Test view context.
    ///
    /// Returns:
    ///     The open intrinsic popover.
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        MoonPopover::new("intrinsic-geometry")
            .default_open(true)
            .fit_content()
            .trigger(div().w(px(20.0)).h(px(20.0)))
            .content(
                div()
                    .debug_selector(|| "intrinsic-geometry:child".to_string())
                    .w(px(73.0))
                    .h(px(20.0)),
            )
    }
}

/// `popover.rs:MoonPopover::render` must leave intrinsic width unset so the rendered border box
/// shrink-wraps its child plus component-owned chrome. Restoring an unconditional default width or
/// omitting scaled padding reddens the corresponding bounds assertion.
#[gpui::test]
fn intrinsic_popover_shrink_wraps_its_rendered_child(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init);
    let scale = MoonScale {
        ui: 2.5,
        font: 0.25,
        font_delta: 0.0,
    };
    cx.update(|cx| {
        MoonTheme::global_mut(cx).scale = scale;
    });
    let window = cx.add_window(|_, _| IntrinsicPopoverHarness);
    let mut cx = VisualTestContext::from_window(window.into(), cx);

    let popup = (0..8)
        .find_map(|_| {
            cx.update(|window, _| window.refresh());
            cx.run_until_parked();
            cx.debug_bounds("intrinsic-geometry:popup")
        })
        .expect("open intrinsic popover must render");
    let child = cx
        .debug_bounds("intrinsic-geometry:child")
        .expect("intrinsic popover child must render");
    let mut tokens = MoonThemeTokens::default();
    tokens.scale = scale;
    let chrome = tokens.ui(POPOVER_PADDING) * 2.0 + POPOVER_BORDER * 2.0;

    assert_eq!(child.size.width, px(73.0));
    assert_eq!(popup.size.width, child.size.width + px(chrome));
}
