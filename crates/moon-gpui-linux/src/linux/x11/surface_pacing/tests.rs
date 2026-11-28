use super::{SETTLE, SurfacePacing};
use std::time::{Duration, Instant};

/// A single resize, the first size of a drag included, rebuilds the surface at once: deferring
/// it would leave a maximized window short of its edges until the pause runs out.
#[test]
fn the_first_size_is_not_deferred() {
    let pacing = SurfacePacing::default();
    assert!(!pacing.defers(Instant::now()));
}

/// Sizes that follow one another closely are the drag the deferral exists for.
#[test]
fn a_size_close_behind_the_previous_one_is_deferred() {
    let mut pacing = SurfacePacing::default();
    let start = Instant::now();
    pacing.size_handled(start);
    let next = start + Duration::from_millis(16);
    assert!(pacing.defers(next));
    pacing.size_handled(next);
    assert!(pacing.defers(next + Duration::from_millis(16)));
}

/// The deferred rebuild waits for the sizes to stop, measured from the newest one, not from the
/// first: a drag that keeps moving must not rebuild on every [`SETTLE`] of its length.
#[test]
fn settles_only_after_the_newest_size_has_rested() {
    let mut pacing = SurfacePacing::default();
    let start = Instant::now();
    pacing.size_handled(start);
    let newest = start + SETTLE - Duration::from_millis(1);
    pacing.size_handled(newest);
    assert!(!pacing.settled(start + SETTLE));
    assert!(pacing.settled(newest + SETTLE));
}

/// After a pause the drag starts a new burst, whose first size again rebuilds at once.
#[test]
fn a_size_after_a_pause_starts_a_new_burst() {
    let mut pacing = SurfacePacing::default();
    let start = Instant::now();
    pacing.size_handled(start);
    assert!(!pacing.defers(start + SETTLE));
}
