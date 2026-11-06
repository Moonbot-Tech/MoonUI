//! Content-zoom contracts for `window.rs`.
//!
//! Browser-style page zoom folds into the window's scale factor. These tests pin the places it
//! must be honoured — geometry, pointer input and the deferred apply from a render — and the two
//! boundaries that cross back to platform space: IME geometry and GPU canvas frame info.

use std::{
    cell::{Cell, RefCell},
    ops::Range,
    rc::Rc,
};

use super::{gpu_canvas_frame_plan, gpu_canvas_prepare_text};
use crate::{
    AnyWindowHandle, App, AppContext as _, AsyncWindowContext, Bounds, Context, FileDropEvent,
    GpuCanvasDrawContext, GpuCanvasDriver, GpuCanvasHandle, GpuCanvasPrepareContext,
    GpuCanvasRetainedTextLayer, GpuCanvasTextContext, GpuCanvasTextTransform, GpuFrameDecision,
    GpuFrameInfo, InputEvent as _, InputHandler, InteractiveElement as _, IntoElement,
    MouseMoveEvent, ParentElement as _, Pixels, PlatformInput, PlatformInputHandler, Point, Render,
    ScrollDelta, ScrollWheelEvent, Size, StatefulInteractiveElement as _, Styled as _,
    TestAppContext, TouchPhase, UTF16Selection, Window, div, gpu_canvas, point, px, size,
};

/// A view that invalidates once from prepaint or paint after its first stable frame.
struct MidDrawNotification {
    armed: Rc<Cell<bool>>,
    renders: Rc<Cell<usize>>,
    during_paint: bool,
}

impl Render for MidDrawNotification {
    /// Register the one-shot mutation in the selected draw phase.
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.renders.set(self.renders.get() + 1);
        let entity = cx.entity_id();
        let prepaint = self.armed.clone();
        let paint = self.armed.clone();
        let during_paint = self.during_paint;
        crate::canvas(
            move |_, _, cx| {
                if !during_paint && prepaint.replace(false) {
                    cx.notify(entity);
                }
            },
            move |_, _, _, cx| {
                if during_paint && paint.replace(false) {
                    cx.notify(entity);
                }
            },
        )
        .size(px(40.))
    }
}

/// Catches ignoring the queued views in the frame-entry predicate: a measured layout change
/// would remain invisible until unrelated input, even though the frame clock was woken for it.
#[test]
fn mid_draw_notification_is_drawn_without_another_event() {
    for during_paint in [false, true] {
        let armed = Rc::new(Cell::new(false));
        let renders = Rc::new(Cell::new(0));
        let (mut app, any) = open_window({
            let armed = armed.clone();
            let renders = renders.clone();
            move |_, _| MidDrawNotification {
                armed,
                renders,
                during_paint,
            }
        });
        app.update_window(any, |_, window, cx| {
            let before = renders.get();
            armed.set(true);
            window.draw(cx).clear();
            assert!(
                window.invalidator.is_dirty(),
                "a mid-draw change still owes a frame"
            );
            // The platform's next tick must take the UI branch without another input event.
            if window.invalidator.is_dirty() {
                window.draw(cx).clear();
            }
            assert_eq!(
                renders.get() - before,
                2,
                "one applying frame follows the mutation"
            );
            assert!(
                !window.invalidator.is_dirty(),
                "the consumed debt must settle"
            );
        })
        .unwrap();
    }
}

/// A cached consumer observes a separate model, as table hosts observe their layout state.
struct MidDrawModelConsumer {
    model: crate::Entity<usize>,
    armed: Rc<Cell<bool>>,
    seen: Rc<Cell<usize>>,
    during_paint: bool,
}

impl Render for MidDrawModelConsumer {
    /// Publish one layout measurement while recording the model value actually rendered.
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.seen.set(*self.model.read(cx));
        let prepaint_model = self.model.clone();
        let paint_model = self.model.clone();
        let prepaint = self.armed.clone();
        let paint = self.armed.clone();
        let during_paint = self.during_paint;
        crate::canvas(
            move |_, _, cx| {
                if !during_paint && prepaint.replace(false) {
                    prepaint_model.update(cx, |value, cx| {
                        *value += 1;
                        cx.notify();
                    });
                }
            },
            move |_, _, _, cx| {
                if during_paint && paint.replace(false) {
                    paint_model.update(cx, |value, cx| {
                        *value += 1;
                        cx.notify();
                    });
                }
            },
        )
        .size(px(40.))
    }
}

/// The model itself has no view path, so only its observer can invalidate this cached child.
struct CachedModelHost(crate::Entity<MidDrawModelConsumer>);

impl Render for CachedModelHost {
    /// Use the production view cache rather than repainting every consumer unconditionally.
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().child(crate::AnyView::from(self.0.clone()).cached(crate::StyleRefinement::default()))
    }
}

/// Queuing a frame alone cannot deliver a model notification: the observer must also run.
#[test]
fn mid_draw_model_notification_reaches_its_cached_observer() {
    for during_paint in [false, true] {
        let armed = Rc::new(Cell::new(false));
        let seen = Rc::new(Cell::new(0));
        let (mut app, window) = open_window({
            let armed = armed.clone();
            let seen = seen.clone();
            move |_, cx| {
                let model = cx.new(|_| 0_usize);
                CachedModelHost(cx.new(|cx| {
                    cx.observe(&model, |_, _, cx| cx.notify()).detach();
                    MidDrawModelConsumer {
                        model,
                        armed,
                        seen,
                        during_paint,
                    }
                }))
            }
        });
        app.update_window(window, |_, window, cx| {
            armed.set(true);
            window.refresh();
            window.draw(cx).clear();
        })
        .unwrap();
        assert_eq!(
            seen.get(),
            1,
            "the cached consumer must draw the measured value without input"
        );
        app.update_window(window, |_, window, _| {
            assert!(
                !window.invalidator.is_dirty(),
                "the observer and frame debt must settle"
            );
        })
        .unwrap();
    }
}

/// A root view that draws nothing; the window's own state is what these tests read.
struct EmptyView;

impl Render for EmptyView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

/// Open a test window on `build` and draw its first frame.
fn open_window<V: Render>(
    build: impl FnOnce(&mut Window, &mut Context<V>) -> V,
) -> (TestAppContext, AnyWindowHandle) {
    let mut app = TestAppContext::single();
    let window = app.add_window(build);
    let any: AnyWindowHandle = window.into();
    draw(&mut app, any);
    (app, any)
}

/// Native and client-drawn close use one replaceable policy; a veto keeps the real window alive.
#[test]
fn user_close_respects_and_replaces_the_native_policy() {
    let (mut app, any) = open_window(|_, _| EmptyView);
    let calls = Rc::new(Cell::new(0));
    app.update_window(any, |_, window, cx| {
        let calls = calls.clone();
        window.on_window_should_close(cx, move |_, _| {
            calls.set(calls.get() + 1);
            false
        });
    })
    .unwrap();
    let platform = app.test_window(any);
    let mut native = platform.0.lock().should_close_handler.take().unwrap();
    assert!(!native());
    app.update_window(any, |_, window, cx| window.request_close(cx))
        .unwrap();
    assert_eq!(calls.get(), 2);
    app.update_window(any, |_, window, cx| {
        assert!(!window.removed);
        window.on_window_should_close(cx, |_, _| true);
        window.request_close(cx);
    })
    .unwrap();
    assert!(
        app.update_window(any, |_, _, _| ()).is_err(),
        "accepted close must release the window"
    );
    assert_eq!(calls.get(), 2, "the superseded veto must not run");
}

/// Teardown is unconditional; a window without a policy still accepts an ordinary user close.
#[test]
fn forced_close_bypasses_policy_and_default_user_close_succeeds() {
    let (mut app, any) = open_window(|_, _| EmptyView);
    app.update_window(any, |_, window, cx| {
        window.on_window_should_close(cx, |_, _| {
            panic!("forced close must not ask the user policy")
        });
        window.remove_window();
    })
    .unwrap();
    assert!(app.update_window(any, |_, _, _| ()).is_err());
    let (mut app, any) = open_window(|_, _| EmptyView);
    app.update_window(any, |_, window, cx| window.request_close(cx))
        .unwrap();
    assert!(app.update_window(any, |_, _, _| ()).is_err());
}

/// A static scene still owes its last frame after a transient backend refusal, without new input.
#[test]
fn deferred_idle_present_retries_then_settles() {
    let (mut app, any) = open_window(|_, _| EmptyView);
    let platform = app.test_window(any);
    platform.0.lock().deferred_presents = 2;
    let requests = platform.0.lock().frame_requests.clone();
    for _ in 0..2 {
        let before = requests.0.load(std::sync::atomic::Ordering::SeqCst);
        platform.simulate_frame();
        assert_eq!(
            requests.0.load(std::sync::atomic::Ordering::SeqCst) - before,
            1,
            "one next native tick is requested"
        );
        app.update_window(any, |_, window, _| {
            assert!(
                window.needs_present.get(),
                "the rejected idle frame remains owed"
            );
            assert!(
                !window.invalidator.is_dirty(),
                "retry must not rebuild the UI"
            );
        })
        .unwrap();
    }
    platform.simulate_frame();
    app.update_window(any, |_, window, _| assert!(!window.needs_present.get()))
        .unwrap();
    assert_eq!(platform.0.lock().draw_attempts, 3);
    platform.simulate_frame();
    assert_eq!(
        platform.0.lock().draw_attempts,
        3,
        "a successful present settles"
    );
}

/// Request one GPU-only frame, consuming the local request just as a retained chart does.
struct OneShotPresent {
    request: Rc<Cell<bool>>,
    text_prepares: Rc<Cell<usize>>,
}

impl GpuCanvasDriver for OneShotPresent {
    fn frame(&mut self, info: GpuFrameInfo) -> GpuFrameDecision {
        if info.presentable && self.request.replace(false) {
            GpuFrameDecision::RequestPresent
        } else {
            GpuFrameDecision::Skip
        }
    }

    fn prepare_text(&mut self, _: &mut GpuCanvasTextContext<'_>) -> anyhow::Result<()> {
        self.text_prepares.set(self.text_prepares.get() + 1);
        Ok(())
    }

    fn draw(&mut self, _: &mut GpuCanvasDrawContext<'_>) -> anyhow::Result<()> {
        Ok(())
    }
}

/// Open the production canvas path with a retained request and count actual text preparations.
fn one_shot_present_window() -> (
    TestAppContext,
    AnyWindowHandle,
    Rc<Cell<bool>>,
    Rc<Cell<usize>>,
) {
    let request = Rc::new(Cell::new(false));
    let prepares = Rc::new(Cell::new(0));
    let handles = vec![GpuCanvasHandle::new(OneShotPresent {
        request: request.clone(),
        text_prepares: prepares.clone(),
    })];
    let (app, any) = open_window(move |_, _| CanvasesView { handles });
    (app, any, request, prepares)
}

/// A failed GPU-only request must CREATE window debt even when its initial window debt was false.
#[test]
fn deferred_gpu_only_present_creates_and_consumes_window_debt() {
    let (mut app, any, request, _) = one_shot_present_window();
    let platform = app.test_window(any);
    platform.simulate_frame();
    app.update_window(any, |_, window, _| assert!(!window.needs_present.get()))
        .unwrap();
    let initial_attempts = platform.0.lock().draw_attempts;
    platform.0.lock().deferred_presents = 2;
    request.set(true);
    let requests = platform.0.lock().frame_requests.clone();
    for _ in 0..2 {
        let before = requests.0.load(std::sync::atomic::Ordering::SeqCst);
        platform.simulate_frame();
        assert_eq!(
            requests.0.load(std::sync::atomic::Ordering::SeqCst) - before,
            1
        );
        app.update_window(any, |_, window, _| assert!(window.needs_present.get()))
            .unwrap();
    }
    assert!(
        !request.get(),
        "the producer already consumed its sole change"
    );
    platform.simulate_frame();
    app.update_window(any, |_, window, _| assert!(!window.needs_present.get()))
        .unwrap();
    assert_eq!(platform.0.lock().draw_attempts - initial_attempts, 3);
    platform.simulate_frame();
    assert_eq!(platform.0.lock().draw_attempts - initial_attempts, 3);
}

/// An occluded window retains debt but must not repeatedly shape the same pending text.
#[test]
fn deferred_present_skips_text_while_unpresentable_and_finishes_on_restore() {
    let (mut app, any, _, prepares) = one_shot_present_window();
    let platform = app.test_window(any);
    let before = prepares.get();
    platform.0.lock().presentable = false;
    for _ in 0..3 {
        platform.simulate_frame();
    }
    assert_eq!(
        prepares.get(),
        before,
        "no text preparation while the backend cannot present"
    );
    app.update_window(any, |_, window, _| assert!(window.needs_present.get()))
        .unwrap();
    platform.0.lock().presentable = true;
    platform.simulate_frame();
    assert_eq!(prepares.get(), before + 1);
    app.update_window(any, |_, window, _| assert!(!window.needs_present.get()))
        .unwrap();
}

/// A late native tick after close must not keep requesting work for the removed window.
#[test]
fn deferred_present_does_not_rearm_a_closed_window() {
    let (mut app, any) = open_window(|_, _| EmptyView);
    let platform = app.test_window(any);
    platform.0.lock().deferred_presents = 1;
    platform.simulate_frame();
    app.update_window(any, |_, window, _| window.remove_window())
        .unwrap();
    let requests = platform.0.lock().frame_requests.clone();
    let before = requests.0.load(std::sync::atomic::Ordering::SeqCst);
    platform.simulate_frame();
    assert_eq!(requests.0.load(std::sync::atomic::Ordering::SeqCst), before);
}

/// Draw one frame of `any`.
fn draw(app: &mut TestAppContext, any: AnyWindowHandle) {
    app.update_window(any, |_, window, cx| {
        window.draw(cx).clear();
    })
    .unwrap();
}

/// Set the content zoom of `any` from outside a draw.
fn zoom(app: &mut TestAppContext, any: AnyWindowHandle, zoom: f32) {
    app.update_window(any, |_, window, cx| window.set_content_zoom(zoom, cx))
        .unwrap();
}

/// The platform factor of `any`, read before any zoom is set.
fn platform_factor(app: &mut TestAppContext, any: AnyWindowHandle) -> f32 {
    app.update_window(any, |_, window, _| window.scale_factor())
        .unwrap()
}

/// Catches dropping either term of `window.rs:sync_platform_geometry` (the interface would keep
/// rendering at the platform density, or lay out against a viewport the zoom no longer fits) or
/// leaving `mouse_position` in the old space in `apply_content_zoom` (the frame that applies the
/// zoom hovers the element at the old position in the new space).
#[test]
fn content_zoom_multiplies_the_platform_factor_and_divides_the_viewport() {
    let (mut app, any) = open_window(|_, _| EmptyView);
    app.update_window(any, |_, window, cx| {
        window.dispatch_event(
            MouseMoveEvent {
                position: point(px(100.), px(100.)),
                modifiers: Default::default(),
                pressed_button: None,
            }
            .to_platform_input(),
            cx,
        );
        let factor = window.scale_factor();
        let viewport = window.viewport_size();
        window.set_content_zoom(2.0, cx);
        assert_eq!(window.content_zoom(), 2.0);
        assert_eq!(window.scale_factor(), factor * 2.0);
        assert_eq!(window.viewport_size(), viewport.map(|d| d / 2.0));
        assert_eq!(
            window.mouse_position(),
            point(px(50.), px(50.)),
            "the tracked pointer follows the space it is compared in"
        );
    })
    .unwrap();
}

/// Catches a re-sync restoring the bare platform reads in `window.rs:bounds_changed`: the first
/// resize or monitor change would silently drop the zoom.
#[test]
fn content_zoom_survives_a_platform_resize() {
    let (mut app, any) = open_window(|_, _| EmptyView);
    let platform_factor = platform_factor(&mut app, any);
    zoom(&mut app, any, 2.0);
    app.simulate_window_resize(any, size(px(800.), px(600.)));
    app.update_window(any, |_, window, _| {
        assert_eq!(window.viewport_size(), size(px(400.), px(300.)));
        assert_eq!(window.scale_factor(), platform_factor * 2.0);
    })
    .unwrap();
}

/// Catches an arm of `window.rs:unzoom_input` moved into its pass-through group: a click at zoom
/// would land on the element twice as far from the origin as the one under the pointer.
#[test]
fn pointer_positions_arrive_in_content_space() {
    let (mut app, any) = open_window(|_, _| EmptyView);
    zoom(&mut app, any, 2.0);
    app.update_window(any, |_, window, cx| {
        window.dispatch_event(
            MouseMoveEvent {
                position: point(px(100.), px(100.)),
                modifiers: Default::default(),
                pressed_button: None,
            }
            .to_platform_input(),
            cx,
        );
        assert_eq!(window.mouse_position(), point(px(50.), px(50.)));

        window.dispatch_event(
            PlatformInput::FileDrop(FileDropEvent::Pending {
                position: point(px(80.), px(40.)),
            }),
            cx,
        );
        assert_eq!(window.mouse_position(), point(px(40.), px(20.)));
    })
    .unwrap();
}

/// A 50 px square that records the scroll delta it receives.
struct ScrollProbe {
    seen: Rc<Cell<Option<ScrollDelta>>>,
}

impl Render for ScrollProbe {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let seen = self.seen.clone();
        div()
            .id("scroll-probe")
            .size(px(50.))
            .on_scroll_wheel(move |event, _, _| seen.set(Some(event.delta)))
    }
}

/// Catches scaling `Lines` or forgetting `Pixels` in `window.rs:unzoom_input`: a trackpad would
/// scroll twice as far as the finger moved, or a wheel notch would shrink with the zoom.
#[test]
fn pixel_scroll_deltas_are_divided_but_line_deltas_are_not() {
    let seen: Rc<Cell<Option<ScrollDelta>>> = Rc::new(Cell::new(None));
    let (mut app, any) = open_window({
        let seen = seen.clone();
        move |_, _| ScrollProbe { seen }
    });
    zoom(&mut app, any, 2.0);
    draw(&mut app, any);
    let scroll = |delta: ScrollDelta| {
        ScrollWheelEvent {
            position: point(px(20.), px(20.)),
            delta,
            modifiers: Default::default(),
            touch_phase: TouchPhase::Moved,
        }
        .to_platform_input()
    };
    app.update_window(any, |_, window, cx| {
        window.dispatch_event(scroll(ScrollDelta::Pixels(point(px(10.), px(20.)))), cx);
        assert!(
            matches!(seen.get(), Some(ScrollDelta::Pixels(d)) if d == point(px(5.), px(10.))),
            "pixel deltas are content space: {:?}",
            seen.get()
        );
        window.dispatch_event(scroll(ScrollDelta::Lines(point(0., 3.))), cx);
        assert!(
            matches!(seen.get(), Some(ScrollDelta::Lines(d)) if d == point(0., 3.)),
            "line deltas are unitless: {:?}",
            seen.get()
        );
    })
    .unwrap();
}

/// Catches removing the guard in `window.rs:set_content_zoom`: a stored `0` would divide the
/// viewport by zero and lay the whole window out at infinity.
#[test]
fn an_impossible_content_zoom_is_ignored() {
    let (mut app, any) = open_window(|_, _| EmptyView);
    let platform_factor = platform_factor(&mut app, any);
    zoom(&mut app, any, 2.0);
    for impossible in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        zoom(&mut app, any, impossible);
        app.update_window(any, |_, window, _| {
            assert_eq!(window.content_zoom(), 2.0, "{impossible} must be ignored");
            assert_eq!(window.scale_factor(), platform_factor * 2.0);
        })
        .unwrap();
    }
}

/// A root view whose only job is to own a bounds observer.
struct BoundsCounter;

impl Render for BoundsCounter {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

/// Catches dropping the observer call in `window.rs:apply_content_zoom` (a consumer that measures
/// against the viewport never re-measures) or the no-op check in `set_content_zoom` (a root that
/// re-applies the theme zoom every frame would fire observers at frame rate).
#[test]
fn changing_content_zoom_notifies_bounds_observers_once() {
    let count: Rc<Cell<usize>> = Rc::new(Cell::new(0));
    let (mut app, any) = open_window({
        let count = count.clone();
        move |window, cx: &mut Context<BoundsCounter>| {
            cx.observe_window_bounds(window, move |_, _, _| count.set(count.get() + 1))
                .detach();
            BoundsCounter
        }
    });
    assert_eq!(count.get(), 0, "drawing alone changes no bounds");
    zoom(&mut app, any, 2.0);
    assert_eq!(count.get(), 1, "a new zoom is a bounds change");
    zoom(&mut app, any, 2.0);
    assert_eq!(count.get(), 1, "the same zoom again is not");
}

/// A root view that, once armed, asks for a zoom from inside its own render, as a themed root
/// does, and records the zoom and viewport each render saw.
struct ZoomingView {
    armed: Rc<Cell<bool>>,
    renders: Rc<RefCell<Vec<(f32, Size<Pixels>)>>>,
}

impl Render for ZoomingView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.renders
            .borrow_mut()
            .push((window.content_zoom(), window.viewport_size()));
        if self.armed.get() {
            window.set_content_zoom(2.0, cx);
        }
        div()
    }
}

/// Catches applying the zoom mid-frame in `window.rs:set_content_zoom` (the root is laid out at
/// the old size and painted at the new density for one frame) or omitting `set_dirty` there (a
/// zoom set from a render would never get the frame that applies it).
#[test]
fn a_zoom_requested_while_drawing_lands_on_the_next_frame() {
    let armed = Rc::new(Cell::new(false));
    let renders: Rc<RefCell<Vec<(f32, Size<Pixels>)>>> = Rc::new(RefCell::new(Vec::new()));
    // Created disarmed, because the harness draws a new window itself; arming afterwards puts
    // the two frames that matter under this test's own draws.
    let (mut app, any) = open_window({
        let armed = armed.clone();
        let renders = renders.clone();
        move |_, _| ZoomingView { armed, renders }
    });
    armed.set(true);
    let frames_before = renders.borrow().len();
    app.update_window(any, |_, window, cx| {
        let viewport = window.viewport_size();
        window.draw(cx).clear();
        assert_eq!(
            window.content_zoom(),
            1.0,
            "the frame in progress keeps its geometry"
        );
        assert!(
            window.invalidator.is_dirty(),
            "the zoom must request the frame that applies it"
        );
        window.draw(cx).clear();
        assert_eq!(window.content_zoom(), 2.0);
        assert_eq!(window.viewport_size(), viewport.map(|d| d / 2.0));
        let seen = renders.borrow();
        assert_eq!(
            &seen[frames_before..],
            &[(1.0, viewport), (2.0, viewport.map(|d| d / 2.0))],
            "the parking render saw the old geometry, the applying one the new"
        );
    })
    .unwrap();
}

/// An input handler with a caret at (10, 10)-(20, 20) in content space that records the point
/// it is asked about.
struct CaretHandler {
    seen_point: Rc<Cell<Option<Point<Pixels>>>>,
}

impl InputHandler for CaretHandler {
    fn selected_text_range(
        &mut self,
        _: bool,
        _: &mut Window,
        _: &mut App,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: 0..0,
            reversed: false,
        })
    }

    fn marked_text_range(&mut self, _: &mut Window, _: &mut App) -> Option<Range<usize>> {
        None
    }

    fn text_for_range(
        &mut self,
        _: Range<usize>,
        _: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut App,
    ) -> Option<String> {
        None
    }

    fn replace_text_in_range(
        &mut self,
        _: Option<Range<usize>>,
        _: &str,
        _: &mut Window,
        _: &mut App,
    ) {
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        _: Option<Range<usize>>,
        _: &str,
        _: Option<Range<usize>>,
        _: &mut Window,
        _: &mut App,
    ) {
    }

    fn unmark_text(&mut self, _: &mut Window, _: &mut App) {}

    fn bounds_for_range(
        &mut self,
        _: Range<usize>,
        _: &mut Window,
        _: &mut App,
    ) -> Option<Bounds<Pixels>> {
        Some(Bounds {
            origin: point(px(10.), px(10.)),
            size: size(px(10.), px(10.)),
        })
    }

    fn character_index_for_point(
        &mut self,
        point: Point<Pixels>,
        _: &mut Window,
        _: &mut App,
    ) -> Option<usize> {
        self.seen_point.set(Some(point));
        Some(0)
    }
}

/// Catches dropping either conversion in `platform.rs:PlatformInputHandler`: the IME candidate
/// window would float away from the caret at zoom, and a click into composed text would pick the
/// wrong character.
#[test]
fn ime_geometry_crosses_to_platform_space_and_back() {
    let (mut app, any) = open_window(|_, _| EmptyView);
    zoom(&mut app, any, 2.0);
    let seen_point = Rc::new(Cell::new(None));
    let mut handler = PlatformInputHandler::new(
        AsyncWindowContext::new_context(app.to_async(), any),
        Box::new(CaretHandler {
            seen_point: seen_point.clone(),
        }),
    );
    let platform_caret = Bounds {
        origin: point(px(20.), px(20.)),
        size: size(px(20.), px(20.)),
    };

    assert_eq!(handler.bounds_for_range(0..0), Some(platform_caret));
    assert_eq!(
        handler.ime_candidate_bounds(),
        Some(platform_caret),
        "composed from the converted range, so converted exactly once"
    );
    app.update_window(any, |_, window, cx| {
        assert_eq!(handler.selected_bounds(window, cx), Some(platform_caret));
    })
    .unwrap();

    assert_eq!(
        handler.character_index_for_point(point(px(40.), px(40.))),
        Some(0)
    );
    assert_eq!(seen_point.get(), Some(point(px(20.), px(20.))));
}

/// Catches `window.rs:apply_content_zoom` leaving a parked request behind: the next `draw` would
/// take the stale zoom and override the one applied since.
#[test]
fn an_immediate_zoom_supersedes_a_parked_one() {
    let armed = Rc::new(Cell::new(false));
    let renders: Rc<RefCell<Vec<(f32, Size<Pixels>)>>> = Rc::new(RefCell::new(Vec::new()));
    let (mut app, any) = open_window({
        let armed = armed.clone();
        let renders = renders.clone();
        move |_, _| ZoomingView { armed, renders }
    });
    armed.set(true);
    draw(&mut app, any);
    armed.set(false);
    zoom(&mut app, any, 1.5);
    draw(&mut app, any);
    app.update_window(any, |_, window, _| {
        assert_eq!(
            window.content_zoom(),
            1.5,
            "the zoom applied outside the draw wins over the one a render parked before it"
        );
    })
    .unwrap();
}

/// A 50 px square that counts its clicks, for the accessibility click fallback.
struct ClickProbe {
    clicks: Rc<Cell<usize>>,
}

impl Render for ClickProbe {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let clicks = self.clicks.clone();
        div()
            .id("click-probe")
            .size(px(50.))
            .on_click(move |_, _, _| clicks.set(clicks.get() + 1))
    }
}

/// Catches `window.rs:handle_a11y_action` handing a content-space centre to `dispatch_event`,
/// which divides it by the zoom again: a screen-reader click at zoom would land on the wrong
/// element, or on nothing.
#[test]
fn an_accessibility_click_lands_on_its_node_at_zoom() {
    let clicks = Rc::new(Cell::new(0));
    let (mut app, any) = open_window({
        let clicks = clicks.clone();
        move |_, _| ClickProbe { clicks }
    });
    zoom(&mut app, any, 2.0);
    draw(&mut app, any);
    app.update_window(any, |_, window, cx| {
        let node = accesskit::NodeId(7);
        window.a11y.node_bounds.insert(
            node,
            Bounds {
                origin: point(px(0.), px(0.)),
                size: size(px(50.), px(50.)),
            },
        );
        window.handle_a11y_action(
            accesskit::ActionRequest {
                action: accesskit::Action::Click,
                target_tree: accesskit::TreeId::ROOT,
                target_node: node,
                data: None,
            },
            cx,
        );
    })
    .unwrap();
    assert_eq!(
        clicks.get(),
        1,
        "the synthetic click must reach the node it was aimed at"
    );
}

/// A GPU canvas driver that records the factor and zoom of the frame info it receives.
struct FrameInfoProbe {
    seen: Rc<Cell<Option<(f32, f32)>>>,
}

impl GpuCanvasDriver for FrameInfoProbe {
    fn frame(&mut self, info: GpuFrameInfo) -> GpuFrameDecision {
        self.seen.set(Some((info.scale_factor, info.content_zoom)));
        GpuFrameDecision::Skip
    }

    fn prepare_gpu(&mut self, _: &mut GpuCanvasPrepareContext<'_>) -> anyhow::Result<()> {
        Ok(())
    }

    fn draw(&mut self, _: &mut GpuCanvasDrawContext<'_>) -> anyhow::Result<()> {
        Ok(())
    }
}

/// A root view that paints one 50 px GPU canvas.
struct CanvasView {
    handle: GpuCanvasHandle,
}

impl Render for CanvasView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        gpu_canvas(self.handle.clone()).size(px(50.))
    }
}

/// Catches dropping the `content_zoom` assignment where `window.rs:frame_gpu_canvases` builds the
/// frame info: a canvas that keeps device density (a chart) could no longer tell the zoom apart
/// from the platform factor and would silently zoom with the interface.
#[test]
fn gpu_canvas_frame_info_carries_the_zoom_beside_the_combined_factor() {
    let seen = Rc::new(Cell::new(None));
    let handle = GpuCanvasHandle::new(FrameInfoProbe { seen: seen.clone() });
    let (mut app, any) = open_window(move |_, _| CanvasView { handle });
    let platform_factor = platform_factor(&mut app, any);
    zoom(&mut app, any, 2.0);
    draw(&mut app, any);
    app.update_window(any, |_, window, _| {
        window.frame_gpu_canvases(false);
    })
    .unwrap();
    assert_eq!(seen.get(), Some((platform_factor * 2.0, 2.0)));
}

/// A GPU canvas that submits one retained text layer and counts its text prepares.
struct TextCanvas {
    presents: bool,
    fail: Rc<Cell<bool>>,
    prepares: Rc<Cell<u32>>,
    layer: GpuCanvasRetainedTextLayer,
}

impl TextCanvas {
    fn new(presents: bool) -> (Self, Rc<Cell<u32>>, Rc<Cell<bool>>) {
        let prepares = Rc::new(Cell::new(0));
        let fail = Rc::new(Cell::new(false));
        let canvas = Self {
            presents,
            fail: fail.clone(),
            prepares: prepares.clone(),
            layer: GpuCanvasRetainedTextLayer::default(),
        };
        (canvas, prepares, fail)
    }
}

impl GpuCanvasDriver for TextCanvas {
    fn frame(&mut self, _: GpuFrameInfo) -> GpuFrameDecision {
        if self.presents {
            GpuFrameDecision::RequestPresent
        } else {
            GpuFrameDecision::Skip
        }
    }

    fn prepare_text(&mut self, ctx: &mut GpuCanvasTextContext<'_>) -> anyhow::Result<()> {
        self.prepares.set(self.prepares.get() + 1);
        if self.fail.get() {
            anyhow::bail!("synthetic text failure");
        }
        ctx.draw_retained_text_layer(
            &mut self.layer,
            1,
            1,
            GpuCanvasTextTransform::translate_logical(point(px(0.), px(0.))),
            0..0,
            |_| Ok(()),
        )
    }

    fn draw(&mut self, _: &mut GpuCanvasDrawContext<'_>) -> anyhow::Result<()> {
        Ok(())
    }
}

/// A root view that paints each handle as a 50 px GPU canvas.
struct CanvasesView {
    handles: Vec<GpuCanvasHandle>,
}

impl Render for CanvasesView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().children(
            self.handles
                .iter()
                .map(|handle| gpu_canvas(handle.clone()).size(px(50.))),
        )
    }
}

/// Run the gpu canvases of `any` once and count the text layers the scene carries.
fn canvas_frame(app: &mut TestAppContext, any: AnyWindowHandle, force_present: bool) -> usize {
    app.update_window(any, |_, window, _| {
        window.frame_gpu_canvases(force_present);
        let scene = &window.rendered_frame.scene;
        scene.gpu_canvas_text_under_scene.retained_layers.len()
            + scene.gpu_canvas_text_over_scene.retained_layers.len()
    })
    .unwrap()
}

/// Catches `window.rs:frame_gpu_canvases` preparing every canvas's text whenever any canvas
/// presents (passing the window-wide request instead of the canvas's own): a live chart
/// beside a static one would re-lay-out the static labels every GPU-only frame.
#[test]
fn static_neighbour_not_reprepared() {
    let (live, live_prepares, _) = TextCanvas::new(true);
    let (still, still_prepares, _) = TextCanvas::new(false);
    let handles = vec![GpuCanvasHandle::new(live), GpuCanvasHandle::new(still)];
    let (mut app, any) = open_window(move |_, _| CanvasesView { handles });
    assert_eq!(canvas_frame(&mut app, any, true), 2);
    for _ in 0..20 {
        assert_eq!(
            canvas_frame(&mut app, any, false),
            2,
            "the static canvas's labels must stay in every frame"
        );
    }
    assert_eq!(live_prepares.get(), 21);
    assert_eq!(still_prepares.get(), 1);
}

/// Catches `window.rs:frame_gpu_canvases` keeping the cached text after a failed prepare
/// in a changed environment: after a zoom the labels would draw at the old geometry.
#[test]
fn err_with_changed_env_drops_text() {
    let (canvas, _, fail) = TextCanvas::new(false);
    let handles = vec![GpuCanvasHandle::new(canvas)];
    let (mut app, any) = open_window(move |_, _| CanvasesView { handles });
    assert_eq!(canvas_frame(&mut app, any, true), 1);
    fail.set(true);
    assert_eq!(
        canvas_frame(&mut app, any, true),
        1,
        "a failure in the same environment keeps the last good text"
    );
    zoom(&mut app, any, 2.0);
    draw(&mut app, any);
    assert_eq!(canvas_frame(&mut app, any, true), 0);
}

#[test]
fn gpu_only_skip_does_not_present() {
    let plan = gpu_canvas_frame_plan(false, false, false);
    assert!(!plan.draw_ui);
    assert!(plan.run_gpu_canvases);
    assert!(!plan.present);
}

#[test]
fn gpu_only_request_presents_same_tick() {
    let plan = gpu_canvas_frame_plan(false, false, true);
    assert!(!plan.draw_ui);
    assert!(plan.run_gpu_canvases);
    assert!(plan.present);
}

#[test]
fn ui_dirty_frame_runs_canvas_and_presents_even_if_canvas_skips() {
    let plan = gpu_canvas_frame_plan(true, false, false);
    assert!(plan.draw_ui);
    assert!(plan.run_gpu_canvases);
    assert!(plan.present);
}

#[test]
fn explicit_present_reason_presents_even_if_canvas_skips() {
    let plan = gpu_canvas_frame_plan(false, true, false);
    assert!(!plan.draw_ui);
    assert!(plan.run_gpu_canvases);
    assert!(plan.present);
}

#[test]
fn gpu_only_skip_does_not_prepare_text() {
    assert!(!gpu_canvas_prepare_text(false, false));
}

#[test]
fn explicit_present_prepares_text_even_if_canvases_skip() {
    assert!(gpu_canvas_prepare_text(true, false));
}

#[test]
fn any_canvas_present_request_prepares_the_window_text_frame() {
    assert!(gpu_canvas_prepare_text(false, true));
}

/// A container with one smaller child at a known offset, for the inspector's element map.
struct PickableView;

impl Render for PickableView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().id("outer").size(px(200.0)).child(
            div()
                .id("inner")
                .ml(px(10.0))
                .mt(px(20.0))
                .w(px(40.0))
                .h(px(30.0)),
        )
    }
}

/// The last named segment of every element in `any`'s inspector map, with its bounds, in the
/// order the map lists them.
fn inspector_map(app: &mut TestAppContext, any: AnyWindowHandle) -> Vec<(String, Bounds<Pixels>)> {
    app.update_window(any, |_, window, _| {
        window
            .inspector_hitboxes()
            .into_iter()
            .filter_map(|hitbox| {
                let name = hitbox.id.path.global_id.0.last()?.to_string();
                Some((name, hitbox.bounds))
            })
            .collect()
    })
    .unwrap()
}

/// Catches `inspector_hitboxes` reading `next_frame` instead of the frame just drawn: `next_frame`
/// is cleared when a draw begins, so every tool would read an empty map. Also catches the list
/// losing paint order, which makes a tool take the container for what sits on top of it, and the
/// map filling outside picking mode, which tools rely on to know picking must be turned on first.
#[test]
fn the_inspector_map_lists_every_div_of_a_picking_frame_in_paint_order() {
    let (mut app, any) = open_window(|_, _| PickableView);
    assert!(
        inspector_map(&mut app, any).is_empty(),
        "nothing registers while the inspector is not picking"
    );

    app.update_window(any, |_, window, cx| {
        window.toggle_inspector(cx);
        if let Some(inspector) = window.inspector.clone() {
            inspector.update(cx, |inspector, _| inspector.start_picking());
        }
    })
    .unwrap();
    draw(&mut app, any);

    let map = inspector_map(&mut app, any);
    let position = |name: &str| map.iter().position(|(seen, _)| seen == name);
    let (Some(outer), Some(inner)) = (position("outer"), position("inner")) else {
        panic!("both divs must be in the map, got {map:?}");
    };
    assert!(
        outer < inner,
        "the child is drawn above its parent: {map:?}"
    );
    assert_eq!(
        map[inner].1,
        Bounds::new(point(px(10.0), px(20.0)), size(px(40.0), px(30.0))),
        "the child's bounds are where the layout put it"
    );
}

/// A label inside a control, for the inspector's list of painted text.
struct LabelledView;

impl Render for LabelledView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("panel")
            .size(px(200.0))
            .child(div().id("save").ml(px(10.0)).mt(px(20.0)).child("Save"))
    }
}

/// Catches `inspector_texts` reading `next_frame`, which a draw clears, and the list filling
/// outside picking, which would make every debug frame pay for it. The path is what ties a label to
/// its control: read off the wrong stack, or taken after the element popped, it names the panel
/// instead, and the bounds without the alignment step put centred text in the wrong column.
#[test]
fn the_inspector_lists_painted_text_with_the_elements_around_it() {
    let (mut app, any) = open_window(|_, _| LabelledView);
    let texts = |app: &mut TestAppContext| {
        app.update_window(any, |_, window, _| window.inspector_texts().to_vec())
            .unwrap()
    };
    assert!(
        texts(&mut app).is_empty(),
        "nothing registers while the inspector is not picking"
    );

    app.update_window(any, |_, window, cx| {
        window.toggle_inspector(cx);
        if let Some(inspector) = window.inspector.clone() {
            inspector.update(cx, |inspector, _| inspector.start_picking());
        }
    })
    .unwrap();
    draw(&mut app, any);

    let texts = texts(&mut app);
    let save = texts
        .iter()
        .find(|text| text.text.as_ref() == "Save")
        .unwrap_or_else(|| panic!("the label must be listed, got {texts:?}"));
    let path: Vec<String> = save.path.iter().map(ToString::to_string).collect();
    assert_eq!(
        path.iter().rev().take(2).rev().cloned().collect::<Vec<_>>(),
        ["panel", "save"],
        "the text sits inside the control that draws it"
    );
    assert_eq!(
        save.bounds.origin,
        point(px(10.0), px(20.0)),
        "the text starts where the layout put its control"
    );
}
