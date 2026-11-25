//! When an X11 window rebuilds its GPU surface during an interactive resize.
//!
//! A window manager sends a size per pointer motion of a resize drag, and rebuilding the surface
//! for each one waits for the GPU to go idle and replaces the swapchain. On an ordinary machine
//! that costs a few milliseconds. On a hybrid machine whose external monitor is driven by a
//! second GPU (an NVIDIA output sink under reverse PRIME) a rebuild was measured to stall for two
//! seconds on a good share of the sizes, and the X server — pointer included — with it. So only
//! the first size of a burst rebuilds the surface at once — a single resize, such as a maximize,
//! looks as it always did — and the sizes that follow it within [`SETTLE`] only note the new
//! size. The surface catches up once the sizes have stopped for that long.
//!
//! Deliberately free of every X and toolkit type, like `resize_diag.rs`: depending on nothing but
//! `std`, this file and its tests can be compiled on a host the crate does not build for.

use std::time::{Duration, Instant};

/// How long sizes must stop before a deferred surface rebuild runs; also how close behind the
/// previous size a new one must come to be deferred.
///
/// Long enough to span the gap between two pointer motions of a drag, even on a window manager
/// that waits for each frame before sending the next size; short enough that a pointer resting
/// mid-drag gets a surface of the right size before the pause is noticed.
pub(crate) const SETTLE: Duration = Duration::from_millis(150);

/// The sizes of one window as they are handled. Idle, and free, until a size arrives.
#[derive(Debug, Default)]
pub(crate) struct SurfacePacing {
    /// When the window last finished handling a size or rebuilding its surface; `None` before
    /// the first size.
    last_handled: Option<Instant>,
}

impl SurfacePacing {
    /// Whether a size arriving at `now` waits for its surface rebuild: true when it came within
    /// [`SETTLE`] of the window finishing the previous one, so it belongs to a burst under way.
    pub(crate) fn defers(&self, now: Instant) -> bool {
        self.last_handled
            .is_some_and(|last| !settled_since(last, now))
    }

    /// The window finished handling a size, or rebuilding its surface, at `at`.
    ///
    /// Stamped when the work is done rather than when the size arrived: a rebuild that stalled
    /// for longer than [`SETTLE`] leaves the next size queued behind it, and measured from its
    /// arrival that size would look like the start of a new burst and rebuild at once again.
    pub(crate) fn size_handled(&mut self, at: Instant) {
        self.last_handled = Some(at);
    }

    /// Whether sizes have stopped long enough by `now` for a deferred rebuild to run.
    pub(crate) fn settled(&self, now: Instant) -> bool {
        self.last_handled
            .is_none_or(|last| settled_since(last, now))
    }
}

/// Whether [`SETTLE`] has passed between `last` and `now`.
fn settled_since(last: Instant, now: Instant) -> bool {
    now.saturating_duration_since(last) >= SETTLE
}

#[cfg(test)]
mod tests;
