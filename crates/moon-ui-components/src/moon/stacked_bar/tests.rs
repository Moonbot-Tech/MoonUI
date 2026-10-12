//! Regression coverage for stacked-bar proportions and rendered segment widths.

use gpui::{
    Context, IntoElement, ParentElement as _, Render, Styled as _, VisualTestContext, Window, div,
    px,
};

// Do not use `super::*`: the glob would import the `gpui::test` macro, causing `#[test]` to
// recursively expand into itself.
use super::{MoonBarSegment, MoonStackedBar, segment_fractions};
use crate::moon::MoonTone;

/// Catches `stacked_bar.rs:segment_fractions` dividing by the sum when a larger total is set,
/// which would draw a half-full disk as a full bar.
#[test]
fn a_larger_total_leaves_the_rest_of_the_track_empty() {
    assert_eq!(
        segment_fractions(&[1.0, 1.0, 2.0], Some(8.0)),
        vec![0.125, 0.125, 0.25]
    );
}

/// Catches `stacked_bar.rs:segment_fractions` trusting a total smaller than the segments, which
/// would let the last segments spill past the end of the bar.
#[test]
fn overflowing_segments_are_scaled_to_fit_the_bar() {
    assert_eq!(segment_fractions(&[3.0, 1.0], Some(2.0)), vec![0.75, 0.25]);
    assert_eq!(segment_fractions(&[3.0, 1.0], None), vec![0.75, 0.25]);
}

/// Catches `stacked_bar.rs:segment_fractions` letting a negative or NaN amount through, which
/// would shrink its neighbours or poison every width with NaN.
#[test]
fn unusable_amounts_draw_nothing() {
    assert_eq!(
        segment_fractions(&[-5.0, f64::NAN, 2.0], Some(4.0)),
        vec![0.0, 0.0, 0.5]
    );
    assert_eq!(segment_fractions(&[0.0, 0.0], None), vec![0.0, 0.0]);
    assert_eq!(segment_fractions(&[], Some(10.0)), Vec::<f32>::new());
}

/// Root view hosting a 200px stacked bar.
struct StackedBarHarness;

impl Render for StackedBarHarness {
    /// Render three segments of 1, 1 and 2 units against a total of 8.
    ///
    /// Args:
    ///     _window: Test window.
    ///     _cx: Test view context.
    ///
    /// Returns:
    ///     The rendered container.
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().w(px(200.0)).child(
            MoonStackedBar::new("stacked-bar-harness")
                .total(8.0)
                .segments([
                    MoonBarSegment::new(1.0).tone(MoonTone::Info),
                    MoonBarSegment::new(1.0).tone(MoonTone::Warning),
                    MoonBarSegment::new(2.0).tone(MoonTone::Positive),
                ]),
        )
    }
}

/// Catches `stacked_bar.rs:MoonStackedBar::render` letting segments flex instead of taking their
/// fraction of the track, which would draw the storage split in the wrong proportions.
#[gpui::test]
fn rendered_segments_take_their_share_of_the_track(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init);
    let window = cx.add_window(|_, _| StackedBarHarness);
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();

    let widths: Vec<_> = [
        "moon-stacked-bar:0",
        "moon-stacked-bar:1",
        "moon-stacked-bar:2",
    ]
    .into_iter()
    .map(|selector| {
        cx.debug_bounds(selector)
            .expect("every segment must register rendered bounds")
            .size
            .width
    })
    .collect();
    assert_eq!(widths, vec![px(25.0), px(25.0), px(50.0)]);
}
