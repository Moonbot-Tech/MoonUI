//! Regression coverage for MoonStepper geometry and value changes.

use super::{MoonStepper, MoonStepperDirection, MoonStepperSize, moon_stepper_next_value};

/// Catches changing the compact/normal height or the tested button/value widths in
/// `stepper.rs:MoonStepper::metrics` away from the reviewed designer reference, which would resize
/// the control or its value slot unexpectedly.
#[test]
fn stepper_metrics_match_designer_reference() {
    let compact = MoonStepper::new("compact").size(MoonStepperSize::Compact);
    assert_eq!(compact.metrics().height, 22.0);
    assert_eq!(compact.metrics().button_width, 24.0);
    let normal = MoonStepper::new("normal");
    assert_eq!(normal.metrics().height, 26.0);
    assert_eq!(normal.metrics().value_width, 64.0);
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
