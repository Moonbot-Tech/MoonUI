//! Regression coverage for Moon button sizing and icon-slot rendering.

use super::super::MoonPalette;
use super::{
    MoonButton, MoonButtonIconSlot, MoonButtonSize, MoonTheme, button_text_metrics, size_for,
};

/// Catches ignoring `button.rs:MoonButton::width` or `full_width` during rendering, which would
/// make fixed controls use content width or prevent full-row actions from filling their parent.
#[gpui::test]
fn moon_button_width_builders_preserve_layout_intent(cx: &mut gpui::TestAppContext) {
    let fixed = laid_out_bounds(cx, || MoonButton::new("fixed").label("Fixed").width(42.0));
    let full = laid_out_bounds(cx, || MoonButton::new("full").label("Full").full_width());

    assert_eq!(fixed.size.width, gpui::px(42.0));
    assert_eq!(full.size.width, gpui::px(200.0));
}

/// Catches removing the scaled `Button::px` refinement from
/// `button.rs:MoonButton::render`, which would put localized Action-button text flush against its
/// outline again. The paired default/padded geometry independently proves the seven-unit inset on
/// both sides at two UI scales and in both supported themes.
#[gpui::test]
fn explicit_horizontal_padding_adds_scaled_insets_in_both_themes(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init);
    for palette in [MoonPalette::TERMINAL, MoonPalette::LIGHT] {
        for (ui_scale, expected_delta) in [(1.0, 14.0), (2.0, 28.0)] {
            cx.update(|cx| {
                let theme = MoonTheme::global_mut(cx);
                theme.palette = palette;
                theme.scale.ui = ui_scale;
            });
            let default = laid_out_bounds(cx, || {
                MoonButton::new("action-default")
                    .label("Action")
                    .size(MoonButtonSize::Action)
            });
            let padded = laid_out_bounds(cx, || {
                MoonButton::new("action-padded")
                    .label("Action")
                    .size(MoonButtonSize::Action)
                    .padding_x(7.0)
            });

            assert_eq!(
                padded.size.width - default.size.width,
                gpui::px(expected_delta),
                "horizontal padding delta is wrong for ui_scale={ui_scale}"
            );
        }
    }
}

/// Catches changing the compact values in `button.rs:button_text_metrics` or `size_for`, which
/// would make terminal toolbar buttons visually inconsistent with the dense-toolbar specification.
#[test]
fn toolbar_compact_keeps_terminal_toolbar_dense() {
    assert_eq!(
        button_text_metrics(MoonButtonSize::ToolbarCompact),
        (10.0, 16.0, 4.0)
    );
    assert_eq!(size_for(MoonButtonSize::ToolbarCompact), crate::Size::Small);
}

/// Root view that renders one button and records the laid-out bounds of the wrapper's direct
/// child.
///
/// A `MoonButton` cannot be drawn as a bare element: `Button::render` calls `use_keyed_state`,
/// which needs a real rendering view on the stack.
struct ButtonHarness {
    build: Box<dyn Fn() -> MoonButton>,
    bounds: std::rc::Rc<std::cell::RefCell<Vec<gpui::Bounds<gpui::Pixels>>>>,
}

impl gpui::Render for ButtonHarness {
    /// Render the configured button and record its final child bounds.
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        use gpui::{ParentElement as _, Styled as _};

        let sink = self.bounds.clone();
        // Give full-width buttons an independent 200-pixel parent while keeping content-sized
        // children start-aligned so the root itself does not stretch them.
        gpui::div()
            .w(gpui::px(200.0))
            .flex()
            .flex_row()
            .items_start()
            .justify_start()
            .on_children_prepainted(move |bounds, _, _| *sink.borrow_mut() = bounds)
            .child((self.build)().render())
    }
}

/// Lay the built button out in a real window and return its box.
fn laid_out_bounds(
    cx: &mut gpui::TestAppContext,
    build: impl Fn() -> MoonButton + 'static,
) -> gpui::Bounds<gpui::Pixels> {
    use gpui::AppContext as _;

    cx.update(crate::init);
    let bounds = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let sink = bounds.clone();
    let window = cx.add_window(move |_, _| ButtonHarness {
        build: Box::new(build),
        bounds: sink,
    });
    cx.update_window(window.into(), |_, window, cx| {
        window.draw(cx).clear();
    })
    .unwrap();

    let out = bounds.borrow();
    assert_eq!(out.len(), 1, "expected exactly the button as a child");
    out[0]
}

/// Catches changing the toolbar-compact height in `button.rs:MoonButton::render`, which would make
/// dense toolbar rows taller than the established 26-pixel contract.
#[gpui::test]
fn toolbar_compact_renders_at_dense_height(cx: &mut gpui::TestAppContext) {
    let bounds = laid_out_bounds(cx, || {
        MoonButton::new("dense").size(MoonButtonSize::ToolbarCompact)
    });

    assert_eq!(bounds.size.height, gpui::px(26.0));
}

/// Catches emitting an empty segment container in `button.rs:MoonButton::render`, which would add
/// a phantom gap and shift an icon-only glyph away from the center of its square button.
#[gpui::test]
fn icon_only_button_lays_out_square(cx: &mut gpui::TestAppContext) {
    let bounds = laid_out_bounds(cx, || {
        MoonButton::new("icon-only")
            .size(MoonButtonSize::ToolbarCompact)
            .leading_icon(MoonButtonIconSlot::new("icons/settings.svg"))
    });

    assert_eq!(
        bounds.size.width, bounds.size.height,
        "icon-only button is {:?} - not square, so the empty segment container and its \
         phantom gap are still there",
        bounds.size
    );
}

/// Catches attaching a lone trailing icon as a child in `button.rs:MoonButton::render`, which
/// would bypass the icon-only path and render a non-square control.
#[gpui::test]
fn trailing_only_icon_button_lays_out_square(cx: &mut gpui::TestAppContext) {
    let bounds = laid_out_bounds(cx, || {
        MoonButton::new("trailing-only")
            .size(MoonButtonSize::ToolbarCompact)
            .trailing_icon(MoonButtonIconSlot::new("icons/settings.svg"))
    });

    assert_eq!(
        bounds.size.width, bounds.size.height,
        "trailing-only icon button is {:?} - not square, so the icon is still being \
         attached as a child instead of filling the icon slot",
        bounds.size
    );
}

/// Catches promoting a two-icon button into the icon-only branch in
/// `button.rs:MoonButton::render`, which would collapse its width and hide one icon slot.
#[gpui::test]
fn leading_and_trailing_icons_keep_both_slots(cx: &mut gpui::TestAppContext) {
    let bounds = laid_out_bounds(cx, || {
        MoonButton::new("two-icons")
            .size(MoonButtonSize::ToolbarCompact)
            .leading_icon(MoonButtonIconSlot::new("icons/settings.svg"))
            .trailing_icon(MoonButtonIconSlot::new("icons/settings.svg"))
    });

    assert!(
        bounds.size.width > bounds.size.height,
        "two-icon button collapsed to {:?}",
        bounds.size
    );
}

/// Catches promoting a labeled button into the icon-only branch in
/// `button.rs:MoonButton::render`, which would collapse the Settings action and hide its label.
#[gpui::test]
fn icon_with_label_button_stays_wide(cx: &mut gpui::TestAppContext) {
    let bounds = laid_out_bounds(cx, || {
        MoonButton::new("icon-and-label")
            .size(MoonButtonSize::ToolbarCompact)
            .leading_icon(MoonButtonIconSlot::new("icons/settings.svg"))
            .text_segment("Settings", 0xFFFFFF, 500.0)
    });

    assert!(
        bounds.size.width > bounds.size.height,
        "labelled button collapsed to {:?}",
        bounds.size
    );
}
