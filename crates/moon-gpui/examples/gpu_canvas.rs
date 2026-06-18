#![cfg_attr(target_family = "wasm", no_main)]

use std::{
    cell::Cell,
    rc::Rc,
    time::{Duration, Instant},
};

use anyhow::Result;
use gpui::{
    App, Bounds, Context, GpuCanvasDrawContext, GpuCanvasDriver, GpuFrameDecision, GpuFrameInfo,
    Window, WindowBounds, WindowOptions, div, gpu_canvas, prelude::*, px, rgb, size,
};
use gpui_platform::application;

#[derive(Clone)]
struct ClockCanvas {
    next_present_after: Rc<Cell<Instant>>,
}

impl ClockCanvas {
    fn new() -> Self {
        Self {
            next_present_after: Rc::new(Cell::new(Instant::now())),
        }
    }
}

impl GpuCanvasDriver for ClockCanvas {
    fn frame(&mut self, info: GpuFrameInfo) -> GpuFrameDecision {
        if !info.presentable || info.now < self.next_present_after.get() {
            return GpuFrameDecision::Skip;
        }

        self.next_present_after
            .set(info.now + Duration::from_millis(500));
        GpuFrameDecision::RequestPresent
    }

    fn draw(&mut self, _ctx: &mut GpuCanvasDrawContext<'_>) -> Result<()> {
        Ok(())
    }
}

struct GpuCanvasExample {
    canvas: ClockCanvas,
}

impl Render for GpuCanvasExample {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(rgb(0x202020))
            .child(gpu_canvas(self.canvas.clone()))
            .child(
                div()
                    .absolute()
                    .top_4()
                    .left_4()
                    .text_color(rgb(0xffffff))
                    .child(
                        "gpu_canvas requests one present every 500 ms without dirtying the view",
                    ),
            )
    }
}

fn run_example() {
    application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(640.), px(360.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| {
                cx.new(|_| GpuCanvasExample {
                    canvas: ClockCanvas::new(),
                })
            },
        )
        .unwrap();
        cx.activate(true);
    });
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    run_example();
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    gpui_platform::web_init();
    run_example();
}
