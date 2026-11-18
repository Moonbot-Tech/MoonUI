//! A once-a-second account of what an interactive X11 resize cost.
//!
//! Whether a resize stalls is a runtime question, and the machine it stalls on is never the one
//! the code is written on. So the window keeps a handful of counters while sizes arrive and
//! writes one line per second, for as long as they keep arriving and never otherwise: how many
//! sizes it applied, how long the X server and the GPU surface took, how many frames it drew and
//! how many of those were not presented, and how long the window manager waited for its answer.
//!
//! Deliberately free of every X and toolkit type, like `display/geometry.rs`: depending on nothing
//! but `std`, this file and its tests can be compiled on a host the crate does not build for.

use std::{
    fmt,
    time::{Duration, Instant},
};

/// How long one summary covers.
const PERIOD: Duration = Duration::from_secs(1);

/// The total, the longest and the number of a series of durations.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Span {
    total: Duration,
    longest: Duration,
    count: u32,
}

impl Span {
    /// Add one duration to the series.
    fn add(&mut self, duration: Duration) {
        self.total += duration;
        self.longest = self.longest.max(duration);
        self.count += 1;
    }

    /// How many durations the series holds.
    pub(crate) fn count(&self) -> u32 {
        self.count
    }

    /// The mean of the series in milliseconds, zero for an empty one.
    pub(crate) fn mean_ms(&self) -> f64 {
        if self.count == 0 {
            return 0.0;
        }
        millis(self.total) / f64::from(self.count)
    }

    /// The longest duration of the series in milliseconds.
    pub(crate) fn longest_ms(&self) -> f64 {
        millis(self.longest)
    }
}

/// A duration in milliseconds, with its fraction.
pub(crate) fn millis(duration: Duration) -> f64 {
    // From whole nanoseconds, so that a whole number of milliseconds comes out exact.
    duration.as_nanos() as f64 / 1_000_000.0
}

/// What one period of a resize cost. Printed as the log line.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Summary {
    /// Sizes the window applied: its surface was resized and the toolkit told.
    pub(crate) applied: u32,
    /// Sizes dropped unapplied, because a newer one for the same window had already arrived.
    pub(crate) skipped: u32,
    /// Time spent waiting for the X server while applying a size.
    pub(crate) server: Span,
    /// Time spent resizing the GPU surface while applying a size.
    pub(crate) surface: Span,
    /// Frames handed to the renderer.
    pub(crate) frames: u32,
    /// Frames the renderer did not present, of `frames`.
    pub(crate) frames_dropped: u32,
    /// How long after a size was applied the window manager got its answer.
    pub(crate) answer_wait: Span,
}

impl fmt::Display for Summary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} sizes applied, {} skipped; X server {:.1} ms mean, {:.1} longest; \
             surface {:.1} ms mean, {:.1} longest; {} frames, {} not presented; \
             {} answers to the window manager, {:.1} ms mean after the size, {:.1} longest",
            self.applied,
            self.skipped,
            self.server.mean_ms(),
            self.server.longest_ms(),
            self.surface.mean_ms(),
            self.surface.longest_ms(),
            self.frames,
            self.frames_dropped,
            self.answer_wait.count(),
            self.answer_wait.mean_ms(),
            self.answer_wait.longest_ms(),
        )
    }
}

/// The counters of one window. Idle, and free, until a size arrives.
#[derive(Debug, Default)]
pub(crate) struct ResizeDiag {
    /// When the current period started; `None` while no resize is going on.
    started: Option<Instant>,
    summary: Summary,
}

impl ResizeDiag {
    /// Start a period at `now` unless one is running.
    fn open(&mut self, now: Instant) {
        self.started.get_or_insert(now);
    }

    /// A size was applied; `server` and `surface` are what the two slow steps took.
    pub(crate) fn size_applied(&mut self, now: Instant, server: Duration, surface: Duration) {
        self.open(now);
        self.summary.applied += 1;
        self.summary.server.add(server);
        self.summary.surface.add(surface);
    }

    /// `count` sizes of this window were dropped unapplied: a newer one was in the same batch.
    pub(crate) fn sizes_skipped(&mut self, now: Instant, count: u32) {
        self.open(now);
        self.summary.skipped += count;
    }

    /// A frame went to the renderer. Counted only during a resize: frames are the steady state
    /// of a window and say nothing on their own.
    pub(crate) fn frame(&mut self, presented: bool) {
        if self.started.is_none() {
            return;
        }
        self.summary.frames += 1;
        if !presented {
            self.summary.frames_dropped += 1;
        }
    }

    /// The window manager was told a size is handled, `waited` after that size was applied.
    ///
    /// Counted only during a resize, like a frame: a window manager may number a move as well,
    /// and a drag must not write a line per second.
    pub(crate) fn answered(&mut self, waited: Duration) {
        if self.started.is_none() {
            return;
        }
        self.summary.answer_wait.add(waited);
    }

    /// The summary of the period that ended by `now`, once per period; the counters start over.
    ///
    /// Call it on the window's frame clock rather than on a size: the last period of a resize
    /// ends after the last size arrived.
    pub(crate) fn take_summary(&mut self, now: Instant) -> Option<Summary> {
        let started = self.started?;
        if now.saturating_duration_since(started) < PERIOD {
            return None;
        }
        self.started = None;
        Some(std::mem::take(&mut self.summary))
    }
}

#[cfg(test)]
mod tests;
