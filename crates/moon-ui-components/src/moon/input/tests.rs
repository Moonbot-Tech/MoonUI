//! Layout checks for custom input metrics, including the terminal's normal font delta.

use gpui::{
    AppContext as _, Context, Entity, InteractiveElement as _, IntoElement, ParentElement as _,
    Render, Styled as _, TestAppContext, VisualTestContext, Window, div, px,
};

use super::{MoonInput, MoonInputSize, MoonInputState};
use crate::{
    Root,
    moon::{MoonTheme, MoonThemeConfig, ThemeMode},
    v_flex,
};

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
