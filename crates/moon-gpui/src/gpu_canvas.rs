use std::{cell::RefCell, ffi::c_void, marker::PhantomData, ptr::NonNull, rc::Rc};

use refineable::Refineable as _;
use scheduler::Instant;

use crate::{
    App, Bounds, ContentMask, DrawOrder, Element, ElementId, GlobalElementId, InspectorElementId,
    IntoElement, LayoutId, Pixels, ScaledPixels, Style, StyleRefinement, Styled, Window,
};

/// Construct a retained native GPU canvas element.
///
/// The element receives layout, clipping, lifetime, and window ownership from the
/// GPUI tree. Its driver decides whether a GPU-only frame should be presented
/// without marking GPUI views dirty.
pub fn gpu_canvas<D>(driver: D) -> GpuCanvas
where
    D: Into<GpuCanvasHandle>,
{
    GpuCanvas {
        driver: driver.into(),
        layer: GpuCanvasLayer::UnderScene,
        style: StyleRefinement::default(),
    }
}

/// The compositing phase for a [`GpuCanvas`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GpuCanvasLayer {
    /// Draw before ordinary GPUI scene primitives.
    UnderScene,
    /// Draw after ordinary GPUI scene primitives.
    OverScene,
}

/// CPU-side decision returned by [`GpuCanvasDriver::frame`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GpuFrameDecision {
    /// This canvas does not need a present for this tick.
    Skip,
    /// This canvas wants the current platform tick to acquire, draw, and present.
    RequestPresent,
}

impl GpuFrameDecision {
    pub(crate) fn requests_present(self) -> bool {
        matches!(self, Self::RequestPresent)
    }
}

/// CPU-only frame information passed before GPUI clears or presents.
#[derive(Clone, Copy, Debug)]
pub struct GpuFrameInfo {
    /// Monotonic timestamp for the current platform tick.
    pub now: Instant,
    /// Canvas bounds in logical pixels.
    pub bounds: Bounds<Pixels>,
    /// Window scale factor used to convert logical pixels to device pixels.
    pub scale_factor: f32,
    /// Whether the platform currently expects a present to be possible.
    pub presentable: bool,
}

/// Backend that produced a [`RawGpuAccess`] callback context.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GpuBackend {
    /// Windows Direct3D 11.
    D3d11,
    /// Apple Metal.
    Metal,
    /// The wgpu backend used by Linux and headless GPU rendering.
    Wgpu,
}

/// Opaque frame-scoped native GPU access.
///
/// All pointers are borrowed for the duration of the callback only. Consumers
/// must not store them, release them, or present the swapchain/drawable/surface.
#[derive(Clone, Copy, Debug)]
pub enum RawGpuAccess<'a> {
    /// Direct3D 11 callback context.
    D3d11(D3d11RawAccess<'a>),
    /// Metal callback context.
    Metal(MetalRawAccess<'a>),
    /// wgpu callback context.
    Wgpu(WgpuRawAccess<'a>),
}

impl RawGpuAccess<'_> {
    /// Return the backend kind for this raw context.
    pub fn backend(&self) -> GpuBackend {
        match self {
            Self::D3d11(_) => GpuBackend::D3d11,
            Self::Metal(_) => GpuBackend::Metal,
            Self::Wgpu(_) => GpuBackend::Wgpu,
        }
    }

    /// Backbuffer width in physical pixels.
    pub fn width(&self) -> u32 {
        match self {
            Self::D3d11(access) => access.width,
            Self::Metal(access) => access.width,
            Self::Wgpu(access) => access.width,
        }
    }

    /// Backbuffer height in physical pixels.
    pub fn height(&self) -> u32 {
        match self {
            Self::D3d11(access) => access.height,
            Self::Metal(access) => access.height,
            Self::Wgpu(access) => access.height,
        }
    }

    /// Monotonic generation bumped by the backend when native resources are recreated.
    pub fn device_generation(&self) -> u64 {
        match self {
            Self::D3d11(access) => access.device_generation,
            Self::Metal(access) => access.device_generation,
            Self::Wgpu(access) => access.device_generation,
        }
    }
}

/// Opaque borrowed Direct3D 11 handles for a canvas callback.
#[derive(Clone, Copy, Debug)]
pub struct D3d11RawAccess<'a> {
    /// Monotonic generation bumped when GPUI recreates D3D resources.
    pub device_generation: u64,
    /// `ID3D11Device*`.
    pub device: NonNull<c_void>,
    /// `ID3D11DeviceContext*`.
    pub context: NonNull<c_void>,
    /// Current `ID3D11RenderTargetView*`.
    pub render_target: NonNull<c_void>,
    /// Numeric `DXGI_FORMAT`.
    pub render_target_format: u64,
    /// Backbuffer width in physical pixels.
    pub width: u32,
    /// Backbuffer height in physical pixels.
    pub height: u32,
    /// Prevents storing borrowed backend handles as `'static`.
    pub _marker: PhantomData<&'a ()>,
}

/// Opaque borrowed Metal handles for a canvas callback.
#[derive(Clone, Copy, Debug)]
pub struct MetalRawAccess<'a> {
    /// Monotonic generation bumped when GPUI recreates Metal resources.
    pub device_generation: u64,
    /// `MTLDevice*`.
    pub device: NonNull<c_void>,
    /// Current `MTLCommandBuffer*`.
    pub command_buffer: NonNull<c_void>,
    /// Current phase `MTLRenderCommandEncoder*` when drawing, or `None` during prepare.
    pub command_encoder: Option<NonNull<c_void>>,
    /// Current drawable/target `MTLTexture*`.
    pub render_target: NonNull<c_void>,
    /// Numeric `MTLPixelFormat`.
    pub render_target_format: u64,
    /// Backbuffer width in physical pixels.
    pub width: u32,
    /// Backbuffer height in physical pixels.
    pub height: u32,
    /// Prevents storing borrowed backend handles as `'static`.
    pub _marker: PhantomData<&'a ()>,
}

/// Opaque borrowed wgpu handles for a canvas callback.
#[derive(Clone, Copy, Debug)]
pub struct WgpuRawAccess<'a> {
    /// Monotonic generation bumped when GPUI recreates wgpu resources.
    pub device_generation: u64,
    /// `wgpu::Device*`.
    pub device: NonNull<c_void>,
    /// `wgpu::Queue*`.
    pub queue: NonNull<c_void>,
    /// Current `wgpu::CommandEncoder*`.
    pub command_encoder: NonNull<c_void>,
    /// Current `wgpu::RenderPass*` during draw, or `None` during prepare.
    pub render_pass: Option<NonNull<c_void>>,
    /// Current `wgpu::TextureView*`.
    pub render_target: NonNull<c_void>,
    /// `wgpu::TextureFormat*`.
    pub render_target_format: NonNull<c_void>,
    /// Backbuffer width in physical pixels.
    pub width: u32,
    /// Backbuffer height in physical pixels.
    pub height: u32,
    /// Prevents storing borrowed backend handles as `'static`.
    pub _marker: PhantomData<&'a ()>,
}

/// Context passed to [`GpuCanvasDriver::prepare_gpu`].
#[derive(Clone, Copy, Debug)]
pub struct GpuCanvasPrepareContext<'a> {
    /// Raw backend access for this frame.
    pub gpu: RawGpuAccess<'a>,
    /// Canvas bounds in scaled/device pixels.
    pub bounds: Bounds<ScaledPixels>,
    /// Current rectangular content mask in scaled/device pixels.
    pub content_mask: ContentMask<ScaledPixels>,
    /// Canvas compositing layer.
    pub layer: GpuCanvasLayer,
}

/// Context passed to [`GpuCanvasDriver::draw`].
#[derive(Clone, Copy, Debug)]
pub struct GpuCanvasDrawContext<'a> {
    /// Raw backend access for this draw pass.
    pub gpu: RawGpuAccess<'a>,
    /// Canvas bounds in scaled/device pixels.
    pub bounds: Bounds<ScaledPixels>,
    /// Current rectangular content mask in scaled/device pixels.
    pub content_mask: ContentMask<ScaledPixels>,
    /// Canvas compositing layer.
    pub layer: GpuCanvasLayer,
}

/// Retained state object used by a [`GpuCanvas`] element.
pub trait GpuCanvasDriver: 'static {
    /// Decide whether this platform tick needs an actual present.
    fn frame(&mut self, info: GpuFrameInfo) -> GpuFrameDecision;

    /// Prepare GPU resources for a frame that is actually going to present.
    fn prepare_gpu(&mut self, _ctx: &mut GpuCanvasPrepareContext<'_>) -> anyhow::Result<()> {
        Ok(())
    }

    /// Draw the current canvas pixels in the provided backend phase.
    fn draw(&mut self, ctx: &mut GpuCanvasDrawContext<'_>) -> anyhow::Result<()>;
}

/// Cloneable handle stored in retained GPUI scenes.
#[derive(Clone)]
pub struct GpuCanvasHandle(Rc<RefCell<dyn GpuCanvasDriver>>);

impl GpuCanvasHandle {
    /// Construct a handle from a retained driver.
    pub fn new<D>(driver: D) -> Self
    where
        D: GpuCanvasDriver,
    {
        Self(Rc::new(RefCell::new(driver)))
    }

    pub(crate) fn frame(&self, info: GpuFrameInfo) -> GpuFrameDecision {
        match self.0.try_borrow_mut() {
            Ok(mut driver) => driver.frame(info),
            Err(error) => {
                log::error!("failed to frame gpu canvas: {error}");
                GpuFrameDecision::Skip
            }
        }
    }

    #[doc(hidden)]
    pub fn prepare_gpu(&self, ctx: &mut GpuCanvasPrepareContext<'_>) -> anyhow::Result<()> {
        self.0
            .try_borrow_mut()
            .map_err(|_| anyhow::anyhow!("gpu canvas driver is already borrowed"))?
            .prepare_gpu(ctx)
    }

    #[doc(hidden)]
    pub fn draw(&self, ctx: &mut GpuCanvasDrawContext<'_>) -> anyhow::Result<()> {
        self.0
            .try_borrow_mut()
            .map_err(|_| anyhow::anyhow!("gpu canvas driver is already borrowed"))?
            .draw(ctx)
    }
}

impl<D> From<D> for GpuCanvasHandle
where
    D: GpuCanvasDriver,
{
    fn from(driver: D) -> Self {
        Self::new(driver)
    }
}

impl<D> From<Rc<RefCell<D>>> for GpuCanvasHandle
where
    D: GpuCanvasDriver,
{
    fn from(driver: Rc<RefCell<D>>) -> Self {
        Self(driver)
    }
}

/// A GPU canvas record stored in a retained scene.
#[derive(Clone)]
pub struct PaintGpuCanvas {
    /// Draw order assigned by the scene.
    pub order: DrawOrder,
    /// Canvas bounds in scaled/device pixels.
    pub bounds: Bounds<ScaledPixels>,
    /// Rectangular content mask in scaled/device pixels.
    pub content_mask: ContentMask<ScaledPixels>,
    /// Retained driver handle.
    pub driver: GpuCanvasHandle,
}

/// A phase-composited native GPU element.
pub struct GpuCanvas {
    driver: GpuCanvasHandle,
    layer: GpuCanvasLayer,
    style: StyleRefinement,
}

impl GpuCanvas {
    /// Draw this canvas after ordinary GPUI scene primitives.
    pub fn over(mut self) -> Self {
        self.layer = GpuCanvasLayer::OverScene;
        self
    }
}

impl IntoElement for GpuCanvas {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for GpuCanvas {
    type RequestLayoutState = Style;
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.refine(&self.style);
        let layout_id = window.request_layout(style.clone(), [], cx);
        (layout_id, style)
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Style,
        _window: &mut Window,
        _cx: &mut App,
    ) {
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        style: &mut Style,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let layer = self.layer;
        let driver = self.driver.clone();
        style.paint(bounds, window, cx, move |window, _cx| {
            window.paint_gpu_canvas(layer, bounds, driver);
        });
    }
}

impl Styled for GpuCanvas {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}
