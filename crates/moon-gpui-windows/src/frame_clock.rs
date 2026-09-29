//! Per-window reasons to draw, read by the vsync thread to decide whether a
//! window gets a `WM_GPUI_FRAME_CLOCK` on this vblank.
//!
//! One-shot reasons are taken by the UI thread when it starts a frame; sticky
//! reasons stay set until their owner clears them. The UI thread clears
//! `pending` before it takes the one-shot bits, so a request raised while a
//! frame is drawing survives into the next vblank.

use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering};

use gpui::PlatformFrameRequester;

/// Something asked for a frame (UI dirty, next-frame callback, owed present).
pub(crate) const FRAME_REASON_REQUEST: u8 = 1;
/// The GPU device was recovered and the next frame must be a forced render.
pub(crate) const FRAME_REASON_FORCE_RENDER: u8 = 2;
/// The window has gpu canvases, which draw every frame.
pub(crate) const FRAME_REASON_GPU_CANVAS: u8 = 4;
/// A Direct Manipulation gesture or its inertia is in flight.
pub(crate) const FRAME_REASON_DIRECT_MANIP: u8 = 8;

const ONE_SHOT_REASONS: u8 = FRAME_REASON_REQUEST | FRAME_REASON_FORCE_RENDER;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FrameClockDecision {
    Post,
    SkipIdle,
    SkipPending,
}

/// Reasons decide first, then `pending`: the same order `try_begin_post` uses.
pub(crate) fn frame_clock_decision(reasons: u8, pending: bool) -> FrameClockDecision {
    if reasons == 0 {
        FrameClockDecision::SkipIdle
    } else if pending {
        FrameClockDecision::SkipPending
    } else {
        FrameClockDecision::Post
    }
}

#[derive(Debug)]
pub(crate) struct FrameClockState {
    reasons: AtomicU8,
    pending: AtomicBool,
    posts: AtomicU64,
    skips: AtomicU64,
}

impl FrameClockState {
    /// A new window starts dirty, so it starts with a request.
    pub(crate) fn new() -> Self {
        Self {
            reasons: AtomicU8::new(FRAME_REASON_REQUEST),
            pending: AtomicBool::new(false),
            posts: AtomicU64::new(0),
            skips: AtomicU64::new(0),
        }
    }

    pub(crate) fn request(&self, bit: u8) {
        self.reasons.fetch_or(bit, Ordering::AcqRel);
    }

    pub(crate) fn set_sticky(&self, bit: u8, on: bool) {
        if on {
            self.reasons.fetch_or(bit, Ordering::AcqRel);
        } else {
            self.reasons.fetch_and(!bit, Ordering::AcqRel);
        }
    }

    /// Vsync thread: decides this vblank and, on `Post`, marks the message pending.
    pub(crate) fn try_begin_post(&self) -> FrameClockDecision {
        // Reasons alone decide; the compare-exchange on `pending` settles a post.
        let mut decision = frame_clock_decision(self.reasons.load(Ordering::Acquire), false);
        if decision == FrameClockDecision::Post
            && self
                .pending
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_err()
        {
            decision = FrameClockDecision::SkipPending;
        }
        if decision == FrameClockDecision::Post {
            self.posts.fetch_add(1, Ordering::Relaxed);
        } else {
            self.skips.fetch_add(1, Ordering::Relaxed);
        }
        decision
    }

    /// Frame clock posts and skips since the window was created.
    pub(crate) fn post_counts(&self) -> (u64, u64) {
        (
            self.posts.load(Ordering::Relaxed),
            self.skips.load(Ordering::Relaxed),
        )
    }

    /// Vsync thread: the post failed, so the message is not pending after all.
    pub(crate) fn cancel_post(&self) {
        self.pending.store(false, Ordering::Release);
    }

    /// UI thread: starts a frame and returns the one-shot reasons it consumed.
    pub(crate) fn begin_frame(&self) -> u8 {
        self.pending.store(false, Ordering::Release);
        self.reasons.fetch_and(!ONE_SHOT_REASONS, Ordering::AcqRel) & ONE_SHOT_REASONS
    }

    /// UI thread: a frame that did not draw gives its one-shot reasons back.
    pub(crate) fn rearm(&self, bits: u8) {
        if bits != 0 {
            self.reasons.fetch_or(bits, Ordering::AcqRel);
        }
    }
}

impl PlatformFrameRequester for FrameClockState {
    fn request_frame(&self) {
        self.request(FRAME_REASON_REQUEST);
    }
}
