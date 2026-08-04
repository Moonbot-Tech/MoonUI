//! Regression coverage for drum arithmetic.

use gpui::{ScrollDelta, point, px};

use super::{MoonWheelMove, wheel_move, wheel_steps, wrap_value};

/// Catches replacing `rem_euclid` with `%` in `time_wheel.rs:wrap_value`, which would make a drum
/// stick — or panic on the `as u32` cast — as soon as the user spins it past `00`.
#[test]
fn a_drum_wraps_in_both_directions() {
    assert_eq!(wrap_value(0, 24), 0);
    assert_eq!(wrap_value(23, 24), 23);
    assert_eq!(wrap_value(24, 24), 0);
    assert_eq!(wrap_value(-1, 24), 23);
    assert_eq!(wrap_value(-25, 24), 23);
    assert_eq!(wrap_value(-1, 60), 59);
    assert_eq!(wrap_value(125, 60), 5);
}

/// Catches a zero `len` reaching the modulo in `time_wheel.rs:wrap_value`, which divides by zero.
#[test]
fn an_empty_drum_does_not_divide_by_zero() {
    assert_eq!(wrap_value(7, 0), 0);
}

/// Catches inverting the sign in `time_wheel.rs:wheel_move`. GPUI scroll offsets grow when the
/// content moves down, so a positive delta is a scroll up and must walk the drum backwards; the
/// inverted version makes every picker in the app scroll the wrong way.
#[test]
fn scrolling_up_walks_the_drum_backwards() {
    assert_eq!(
        wheel_move(&ScrollDelta::Lines(point(0.0, 1.0)), 20.0),
        MoonWheelMove::Notches(-1)
    );
    assert_eq!(
        wheel_move(&ScrollDelta::Lines(point(0.0, -1.0)), 20.0),
        MoonWheelMove::Notches(1)
    );
    assert_eq!(
        wheel_move(&ScrollDelta::Pixels(point(px(0.0), px(20.0))), 20.0),
        MoonWheelMove::Rows(-1.0)
    );
}

/// Catches honouring the raw line count in `time_wheel.rs:wheel_move`. Windows multiplies one
/// wheel notch by the system lines-per-scroll setting — three by default — so a drum that trusts
/// it jumps three values per notch instead of the one the user asked for.
#[test]
fn one_wheel_notch_moves_exactly_one_row() {
    for lines in [1.0, 3.0, 9.0, 0.5] {
        assert_eq!(
            wheel_move(&ScrollDelta::Lines(point(0.0, lines)), 20.0),
            MoonWheelMove::Notches(-1),
            "a notch reported as {lines} lines must still be one row"
        );
    }
    assert_eq!(
        wheel_move(&ScrollDelta::Lines(point(0.0, 0.0)), 20.0),
        MoonWheelMove::Notches(0)
    );
}

/// Catches rounding the remainder away in `time_wheel.rs:wheel_steps`, which would make a trackpad
/// — which sends many sub-row pixel deltas — unable to move the drum at all.
#[test]
fn sub_row_movement_accumulates_instead_of_vanishing() {
    let (steps, carry) = wheel_steps(0.4, 0.0);
    assert_eq!(steps, 0);
    let (steps, carry) = wheel_steps(0.4, carry);
    assert_eq!(steps, 0);
    let (steps, carry) = wheel_steps(0.4, carry);
    assert_eq!(steps, 1, "three fifths of a row must add up to one step");
    assert!(
        carry.abs() < 1.0,
        "the remainder must stay below one whole step, got {carry}"
    );
}

/// Catches letting a non-finite delta through `time_wheel.rs:wheel_steps`, which would poison the
/// carry and freeze the drum for the rest of the session.
#[test]
fn a_broken_delta_leaves_the_carry_intact() {
    let (steps, carry) = wheel_steps(f32::NAN, 0.25);
    assert_eq!(steps, 0);
    assert_eq!(carry, 0.25);
}
