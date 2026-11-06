//! Real-surface regression for a retained canvas bake followed by a scene-buffer retry.

mod native_window;

use super::{GpuContext, WgpuRenderer, WgpuSurfaceConfig};
use gpui::{
    Bounds, ContentMask, DevicePixels, GpuCanvasDrawContext, GpuCanvasDriver, GpuCanvasHandle,
    GpuCanvasLayer, GpuCanvasPrepareContext, GpuFrameDecision, GpuFrameInfo, PaintGpuCanvas, Quad,
    RawGpuAccess, ScaledPixels, Scene, Underline, point, size,
};
use std::{cell::RefCell, rc::Rc, sync::Arc, time::Duration};

/// A chart-like owner that marks a retained bake clean as soon as it records the GPU commands.
struct RetainedBake {
    texture: wgpu::Texture,
    dirty: bool,
}

impl GpuCanvasDriver for RetainedBake {
    fn frame(&mut self, _info: GpuFrameInfo) -> GpuFrameDecision {
        GpuFrameDecision::Skip
    }

    fn prepare_gpu(&mut self, ctx: &mut GpuCanvasPrepareContext<'_>) -> anyhow::Result<()> {
        if self.dirty {
            let RawGpuAccess::Wgpu(raw) = ctx.gpu else {
                unreachable!()
            };
            // The raw API lends the renderer's current encoder for this synchronous callback.
            let encoder =
                unsafe { &mut *raw.command_encoder.as_ptr().cast::<wgpu::CommandEncoder>() };
            let view = self.texture.create_view(&Default::default());
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("retained bake before overflow"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 1.0,
                            g: 0.25,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                ..Default::default()
            });
            self.dirty = false;
        }
        Ok(())
    }

    fn draw(&mut self, _ctx: &mut GpuCanvasDrawContext<'_>) -> anyhow::Result<()> {
        Ok(())
    }
}

/// Read the completed retained texture, rather than treating a CPU clean flag as a successful bake.
fn read_pixel(device: &wgpu::Device, queue: &wgpu::Queue, texture: &wgpu::Texture) -> [u8; 4] {
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("retry pixel readback"),
        size: 256,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256),
                rows_per_image: None,
            },
        },
        wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
    );
    queue.submit([encoder.finish()]);
    let (sent, received) = std::sync::mpsc::channel();
    readback
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            sent.send(result).unwrap();
        });
    device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(Duration::from_secs(10)),
        })
        .unwrap();
    received
        .recv_timeout(Duration::from_secs(10))
        .unwrap()
        .unwrap();
    let mapped = readback.slice(..).get_mapped_range();
    mapped[..4].try_into().unwrap()
}

/// Dropping the failed scene encoder loses the only bake, even though the larger retry succeeds.
#[test]
#[ignore = "requires a native GPU surface and DISPLAY on Linux; run explicitly with --ignored"]
fn scene_overflow_preserves_the_already_prepared_canvas_texture() {
    let window = native_window::TestWindow::new();
    let context: GpuContext = Rc::new(RefCell::new(None));
    let mut renderer = WgpuRenderer::new(
        context,
        &window,
        WgpuSurfaceConfig {
            size: size(DevicePixels(64), DevicePixels(64)),
            transparent: false,
            preferred_present_mode: None,
        },
        None,
    )
    .unwrap();
    let device = Arc::clone(&renderer.resources().device);
    let queue = Arc::clone(&renderer.resources().queue);
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("retained chart bake"),
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let bounds = Bounds::new(
        point(ScaledPixels(0.0), ScaledPixels(0.0)),
        size(ScaledPixels(64.0), ScaledPixels(64.0)),
    );
    let mut scene = Scene::default();
    scene.insert_gpu_canvas(
        GpuCanvasLayer::UnderScene,
        PaintGpuCanvas {
            order: Default::default(),
            bounds,
            content_mask: ContentMask { bounds },
            text_layer: GpuCanvasLayer::UnderScene,
            driver: GpuCanvasHandle::new(RetainedBake {
                texture: texture.clone(),
                dirty: true,
            }),
        },
    );
    scene.insert_primitive(Quad {
        bounds,
        content_mask: ContentMask { bounds },
        ..Default::default()
    });
    scene.insert_primitive(Underline {
        order: Default::default(),
        pad: 0,
        bounds,
        content_mask: ContentMask { bounds },
        color: gpui::black(),
        thickness: ScaledPixels(1.0),
        wavy: 0,
    });
    scene.finish();
    // The first batch uses the old buffer; the second overflows. Its lifetime must survive grow.
    let initial_capacity = std::mem::size_of::<Quad>() as u64;
    renderer.instance_buffer_capacity = initial_capacity;
    assert!(renderer.draw(&scene));
    assert!(
        renderer.instance_buffer_capacity > initial_capacity,
        "the scene must actually retry"
    );
    assert_eq!(read_pixel(&device, &queue, &texture), [255, 64, 0, 255]);
}
