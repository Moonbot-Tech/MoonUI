//! Regression coverage for step states, colours and wrapping layout.

use gpui::{
    Context, IntoElement, ParentElement as _, Render, Styled as _, VisualTestContext, Window, div,
    px,
};

// Do not use `super::*`: the glob would import the `gpui::test` macro, causing `#[test]` to
// recursively expand into itself.
use super::{MoonPalette, MoonStepState, MoonSteps, step_colors, step_state};

/// Catches an off-by-one in `steps.rs:step_state`, which would show the current install step as
/// already done or leave a finished one pending.
#[test]
fn steps_before_the_current_one_are_done_and_after_it_pending() {
    let states = MoonSteps::new("states")
        .steps(["Download", "Install", "Login", "Ready"])
        .current(2)
        .states();
    assert_eq!(
        states,
        vec![
            MoonStepState::Done,
            MoonStepState::Done,
            MoonStepState::Current,
            MoonStepState::Pending,
        ]
    );
    let finished = MoonSteps::new("finished")
        .steps(["Download", "Install"])
        .current(2)
        .states();
    assert_eq!(finished, vec![MoonStepState::Done, MoonStepState::Done]);
    assert_eq!(step_state(0, 0), MoonStepState::Current);
}

/// Catches `steps.rs:step_colors` painting pending steps with the accent, which would make the
/// three states indistinguishable at a glance.
#[test]
fn only_done_markers_are_filled_and_pending_steps_lose_the_accent() {
    let accent = 0x123456;
    for p in [MoonPalette::TERMINAL, MoonPalette::LIGHT] {
        let done = step_colors(MoonStepState::Done, accent, p);
        let current = step_colors(MoonStepState::Current, accent, p);
        let pending = step_colors(MoonStepState::Pending, accent, p);
        assert_eq!(done.marker_fill, Some(accent));
        assert_eq!(current.marker_fill, None);
        assert_eq!(pending.marker_fill, None);
        assert_eq!(current.marker_border, accent);
        assert_ne!(pending.marker_border, accent);
        assert_ne!(pending.number, accent);
    }
}

/// Root view hosting four steps inside a container of a chosen width.
struct StepsHarness {
    width: f32,
    vertical: bool,
}

impl Render for StepsHarness {
    /// Render four steps inside a fixed-width container.
    ///
    /// Args:
    ///     _window: Test window.
    ///     _cx: Test view context.
    ///
    /// Returns:
    ///     The rendered container.
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().w(px(self.width)).child(
            MoonSteps::new("steps-harness")
                .steps(["Download", "Install", "Login", "Ready"])
                .current(1)
                .vertical(self.vertical),
        )
    }
}

/// Render the harness and return the top edge of every step.
///
/// Args:
///     cx: Test application context.
///     width: Container width.
///     vertical: Whether the steps stack.
///
/// Returns:
///     The y origin of each of the four steps.
fn step_tops(cx: &mut gpui::TestAppContext, width: f32, vertical: bool) -> Vec<gpui::Pixels> {
    cx.update(crate::init);
    let window = cx.add_window(move |_, _| StepsHarness { width, vertical });
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
    ["moon-step:0", "moon-step:1", "moon-step:2", "moon-step:3"]
        .into_iter()
        .map(|selector| {
            cx.debug_bounds(selector)
                .expect("every step must register rendered bounds")
                .origin
                .y
        })
        .collect()
}

/// Catches dropping `flex_wrap` or the per-step minimum width in `steps.rs:MoonSteps::render`:
/// a narrow settings column would then squeeze four labels into one unreadable row.
#[gpui::test]
fn a_narrow_row_wraps_and_a_wide_one_stays_on_one_line(cx: &mut gpui::TestAppContext) {
    let wide = step_tops(cx, 1000.0, false);
    assert!(
        wide.iter().all(|top| *top == wide[0]),
        "a wide row must keep every step on one line: {wide:?}"
    );
    let narrow = step_tops(cx, 260.0, false);
    assert!(
        narrow[3] > narrow[0],
        "a narrow row must wrap later steps below the first: {narrow:?}"
    );
}

/// Catches `MoonSteps::vertical` being ignored, which would leave a stacked layout as a row.
#[gpui::test]
fn vertical_steps_stack_one_below_another(cx: &mut gpui::TestAppContext) {
    let tops = step_tops(cx, 1000.0, true);
    assert!(
        tops.windows(2).all(|pair| pair[1] > pair[0]),
        "stacked steps must each sit below the previous one: {tops:?}"
    );
}
