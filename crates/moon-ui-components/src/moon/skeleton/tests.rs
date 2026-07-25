//! Regression coverage for MoonSkeleton rendering controls.

use super::{MoonSkeleton, SkeletonRenderPlan};

/// Catches removing the secondary-alpha reduction or ignoring `.animated(false)` in
/// `skeleton.rs:MoonSkeleton::render_plan`, which would render secondary placeholders at primary
/// emphasis or keep static placeholders pulsing.
#[test]
fn skeleton_keeps_longbridge_secondary_and_animation_controls() {
    let plan = MoonSkeleton::new("test")
        .secondary()
        .animated(false)
        .render_plan();

    assert_eq!(
        plan,
        SkeletonRenderPlan {
            alpha: 0.26,
            animated: false,
        }
    );
}
