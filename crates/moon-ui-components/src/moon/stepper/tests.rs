//! Regression coverage for MoonStepper geometry and value changes.

use super::{MoonStepper, MoonStepperDirection, MoonStepperSize, moon_stepper_next_value};

/// Catches restoring fixed presets or font scaling: density selects square buttons and zoom-only text.
#[test]
fn stepper_metrics_match_designer_reference() {
    use super::super::{foundation::MoonSize, theme::MoonThemeTokens};
    for (tier, h, w, font, line) in [
        (MoonSize::Xs, 20.0, 47.0, 12.0, 16.0),
        (MoonSize::Sm, 24.0, 57.0, 14.0, 20.0),
        (MoonSize::Md, 32.0, 79.0, 16.0, 24.0),
        (MoonSize::Xxl, 32.0, 79.0, 16.0, 24.0),
    ] {
        let mut tokens = MoonThemeTokens::default();
        tokens.scale.tier = tier;
        let stepper = MoonStepper::new("density");
        let m = stepper.metrics(&tokens);
        assert_eq!((m.height, m.button_width, m.value_width), (h, h, w));
        tokens.scale.ui = 1.5;
        tokens.scale.font = 2.0;
        tokens.scale.font_delta = 6.0;
        assert_eq!(stepper.text_metrics(&tokens), (font * 1.5, line * 1.5));
    }
}

/// Catches ignoring explicit aliases when the active density differs.
#[test]
#[allow(deprecated)]
fn stepper_aliases_override_density() {
    use super::super::{foundation::MoonSize, theme::MoonThemeTokens};
    let tokens = MoonThemeTokens::default();
    for (size, height) in [
        (MoonStepperSize::Compact, 24.0),
        (MoonStepperSize::Normal, 32.0),
        (MoonSize::Lg.into(), 32.0),
    ] {
        assert_eq!(
            MoonStepper::new("alias").size(size).metrics(&tokens).height,
            height
        );
    }
}

/// Catches routing Custom through UI-only scaling and losing legacy font adjustments.
#[test]
fn stepper_custom_retains_text_scaling() {
    use super::super::theme::MoonThemeTokens;
    let mut tokens = MoonThemeTokens::default();
    tokens.scale.ui = 1.5;
    tokens.scale.font = 2.0;
    tokens.scale.font_delta = 3.0;
    let stepper = MoonStepper::new("custom").size(MoonStepperSize::Custom {
        height: 22.0,
        button_width: 24.0,
        value_width: 52.0,
        font_size: 10.0,
        line_height: 13.0,
    });
    assert_eq!(stepper.text_metrics(&tokens), (23.0, 29.0));
}

/// Catches removing range clamping or positive-step normalization from
/// `stepper.rs:moon_stepper_next_value`, which would cross configured limits or make the `+`
/// button decrease the value for a negative step.
#[test]
fn stepper_next_value_clamps_to_range_and_positive_step() {
    assert_eq!(
        moon_stepper_next_value(9.5, 0.0, 10.0, 2.0, MoonStepperDirection::Increment),
        10.0
    );
    assert_eq!(
        moon_stepper_next_value(0.5, 0.0, 10.0, 2.0, MoonStepperDirection::Decrement),
        0.0
    );
    assert!(moon_stepper_next_value(1.0, 0.0, 10.0, -1.0, MoonStepperDirection::Increment) > 1.0);
}
