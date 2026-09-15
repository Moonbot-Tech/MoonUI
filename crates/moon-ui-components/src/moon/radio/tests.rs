//! Regression coverage for MoonRadio behavior and reviewed geometry.

use super::{MoonRadio, MoonRadioSize, moon_radio_click_value};

/// Catches changing tier metrics or applying legacy font scaling, which enlarges tier labels.
#[test]
fn radio_metrics_match_designer_reference() {
    use crate::moon::{MoonSize, MoonThemeTokens};
    let mut tokens = MoonThemeTokens::default();
    tokens.scale.ui = 2.0;
    tokens.scale.font = 3.0;
    tokens.scale.font_delta = 6.0;
    for (tier, outer, inner, font, line, gap, description_gap) in [
        (MoonSize::Xs, 32.0, 14.0, 28.0, 40.0, 16.0, 0.0),
        (MoonSize::Sm, 32.0, 14.0, 28.0, 40.0, 16.0, 0.0),
        (MoonSize::Md, 40.0, 18.0, 32.0, 48.0, 24.0, 4.0),
        (MoonSize::Lg, 40.0, 18.0, 32.0, 48.0, 24.0, 4.0),
        (MoonSize::Xl, 40.0, 18.0, 32.0, 48.0, 24.0, 4.0),
        (MoonSize::Xxl, 40.0, 18.0, 32.0, 48.0, 24.0, 4.0),
    ] {
        let metrics = MoonRadio::new("tier").size(tier.into()).metrics(&tokens);
        assert_eq!((metrics.outer_size, metrics.inner_size), (outer, inner));
        assert_eq!((metrics.font_size, metrics.line_height), (font, line));
        assert_eq!(
            (metrics.gap, metrics.description_gap),
            (gap, description_gap)
        );
        tokens.scale.tier = tier;
        assert_eq!(MoonRadio::new("density").metrics(&tokens).outer_size, outer);
    }
}

/// Catches ignoring explicit tiers or scaling Custom like a tier, breaking pinned radio sizes.
#[test]
fn radio_tiers_and_custom_scaling_survive() {
    use crate::moon::{MoonSize, MoonThemeTokens};
    let mut tokens = MoonThemeTokens::default();
    tokens.scale.ui = 2.0;
    tokens.scale.font = 3.0;
    tokens.scale.font_delta = 6.0;
    assert_eq!(
        MoonRadio::new("sm")
            .size(MoonRadioSize::Tier(MoonSize::Sm))
            .metrics(&tokens)
            .outer_size,
        32.0
    );
    assert_eq!(
        MoonRadio::new("md")
            .size(MoonRadioSize::Tier(MoonSize::Md))
            .metrics(&tokens)
            .outer_size,
        40.0
    );
    let metrics = MoonRadio::new("custom")
        .size(MoonRadioSize::Custom {
            dot_size: 14.0,
            font_size: 10.0,
            line_height: 13.0,
            gap: 7.0,
        })
        .metrics(&tokens);
    assert_eq!(
        (metrics.outer_size, metrics.font_size, metrics.line_height),
        (28.0, 36.0, 45.0)
    );
}

/// Catches making `radio.rs:moon_radio_click_value` select disabled radios or ignore enabled
/// clicks, which would let unavailable choices change or leave available choices unselected.
#[test]
fn radio_click_value_respects_disabled_state() {
    assert_eq!(moon_radio_click_value(false), Some(true));
    assert_eq!(moon_radio_click_value(true), None);
}

/// Renders a described choice to expose the mark and text geometry.
struct DescribedRadio(crate::moon::MoonSize);

impl gpui::Render for DescribedRadio {
    /// Returns the radio whose geometry the description regression probes.
    fn render(
        &mut self,
        _: &mut gpui::Window,
        _: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        MoonRadio::new("described")
            .size(self.0.into())
            .label("Label")
            .description("Description")
    }
}

/// Catches centring the mark on the whole stack or removing its tier gap, visibly misaligning rows.
#[gpui::test]
fn radio_description_aligns_with_label(cx: &mut gpui::TestAppContext) {
    use crate::moon::{MoonSize, MoonTheme, ThemeMode};
    cx.update(crate::init);
    for mode in [ThemeMode::Dark, ThemeMode::Light] {
        cx.update(|cx| MoonTheme::set_mode(mode, cx));
        for (tier, gap) in [(MoonSize::Sm, 0.0), (MoonSize::Md, 2.0)] {
            let window = cx.add_window(move |_, _| DescribedRadio(tier));
            let mut visual = gpui::VisualTestContext::from_window(window.into(), cx);
            visual.run_until_parked();
            let mark = visual.debug_bounds("described:mark").expect("mark");
            let label = visual.debug_bounds("described:label").expect("label");
            let description = visual
                .debug_bounds("described:description")
                .expect("description");
            assert_eq!(mark.center().y, label.center().y);
            assert_eq!(description.top() - label.bottom(), gpui::px(gap));
        }
    }
}
