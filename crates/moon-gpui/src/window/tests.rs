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

use crate::{
    AnyWindowHandle, App, AppContext as _, AsyncWindowContext, Bounds, Context, FileDropEvent,
    GpuCanvasDrawContext, GpuCanvasDriver, GpuCanvasHandle, GpuCanvasPrepareContext,
    GpuFrameDecision, GpuFrameInfo, InputEvent as _, InputHandler, InteractiveElement as _,
    IntoElement, MouseMoveEvent, Pixels, PlatformInput, PlatformInputHandler, Point, Render,
    ScrollDelta, ScrollWheelEvent, Size, StatefulInteractiveElement as _, Styled as _,
    TestAppContext, TouchPhase, UTF16Selection, Window, div, gpu_canvas, point, px, size,
};

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
