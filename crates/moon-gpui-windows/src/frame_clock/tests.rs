use super::*;

/// A window after its first frame consumed the initial request.
fn drawn_window() -> FrameClockState {
    let state = FrameClockState::new();
    assert_eq!(state.try_begin_post(), FrameClockDecision::Post);
    state.begin_frame();
    state
}

/// Catches `frame_clock.rs:frame_clock_decision` posting with no reasons, or `begin_frame`
/// keeping the one-shot bits: a static window would get a frame message every vblank and
/// redraw forever while nothing changed.
#[test]
fn static_window_gets_no_frame_clock() {
    let state = drawn_window();
    let (posts_before, _) = state.post_counts();
    for _ in 0..100 {
        assert_eq!(state.try_begin_post(), FrameClockDecision::SkipIdle);
    }
    let (posts, skips) = state.post_counts();
    assert_eq!(posts - posts_before, 0);
    assert_eq!(skips, 100);
}

/// Catches `frame_clock.rs:begin_frame` leaving `pending` set: a request raised while the
/// frame draws would be skipped as already pending and the window would never redraw it.
#[test]
fn request_during_draw_is_not_lost() {
    let state = FrameClockState::new();
    assert_eq!(state.try_begin_post(), FrameClockDecision::Post);
    state.begin_frame();
    state.request(FRAME_REASON_REQUEST);
    assert_eq!(state.try_begin_post(), FrameClockDecision::Post);
}

/// Catches `frame_clock.rs:begin_frame` clearing sticky reasons with the one-shots (a gpu
/// canvas window stops animating after one frame) and a post while one is pending (the
/// message queue floods faster than the UI thread drains it).
#[test]
fn sticky_gpu_canvas_survives_begin_frame() {
    let state = drawn_window();
    state.set_sticky(FRAME_REASON_GPU_CANVAS, true);
    assert_eq!(state.try_begin_post(), FrameClockDecision::Post);
    assert_eq!(state.try_begin_post(), FrameClockDecision::SkipPending);
    assert_eq!(state.begin_frame(), 0);
    assert_eq!(state.try_begin_post(), FrameClockDecision::Post);
    state.begin_frame();
    state.set_sticky(FRAME_REASON_GPU_CANVAS, false);
    assert_eq!(state.try_begin_post(), FrameClockDecision::SkipIdle);
}
