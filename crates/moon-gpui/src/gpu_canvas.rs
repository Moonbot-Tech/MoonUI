use std::{
    borrow::Cow, cell::RefCell, ffi::c_void, marker::PhantomData, ptr::NonNull, rc::Rc, sync::Arc,
};

use refineable::Refineable as _;
use scheduler::Instant;

use crate::{
    App, AtlasKey, Bounds, ContentMask, DrawOrder, Element, ElementId, Font, GlobalElementId, Hsla,
    InspectorElementId, IntoElement, IsZero, LayoutId, MonochromeSprite, Pixels, PlatformAtlas,
    Point, PolychromeSprite, RenderGlyphParams, ScaledPixels, ShapedLine, SharedString, Style,
    StyleRefinement, Styled, SubpixelSprite, TextAlign, TextRenderingMode, TextRun,
    TransformationMatrix, Window, WindowBackgroundAppearance, WindowTextSystem, black, point, px,
    text_system::{SUBPIXEL_VARIANTS_X, SUBPIXEL_VARIANTS_Y},
    util::round_half_toward_zero,
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
        text_layer: GpuCanvasLayer::UnderScene,
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

/// A measured single-line text draw submitted through [`GpuCanvasTextContext`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GpuCanvasTextMetrics {
    /// Shaped line width in logical pixels.
    pub width: Pixels,
    /// Caller-provided line height in logical pixels.
    pub line_height: Pixels,
}

#[derive(Clone, Debug, PartialEq)]
struct GpuCanvasTextShapeKey {
    text: SharedString,
    font: Font,
    font_size: Pixels,
}

/// Retained single-line text cache for [`GpuCanvasTextContext`].
///
/// Keep one handle per logical label (axis label, marker caption, cursor readout,
/// etc.) and call [`GpuCanvasTextRun::draw`] every frame. The handle reshapes text
/// only when the UTF-8 string, font, or font size changes. Moving the label,
/// changing color, or drawing the same text every GPU frame reuses the shaped
/// line and only emits glyph sprites for the current frame.
#[derive(Clone, Debug, Default)]
pub struct GpuCanvasTextRun {
    key: Option<GpuCanvasTextShapeKey>,
    line: Option<ShapedLine>,
}

impl GpuCanvasTextRun {
    /// Drop the cached shaped line. The next draw will shape again.
    pub fn clear(&mut self) {
        self.key = None;
        self.line = None;
    }

    /// Returns true when this handle currently owns a shaped line.
    pub fn is_cached(&self) -> bool {
        self.line.is_some()
    }

    fn line(
        &mut self,
        text_system: &WindowTextSystem,
        text: impl AsRef<str>,
        font: Font,
        font_size: Pixels,
    ) -> &ShapedLine {
        let text = text.as_ref();
        let current = self.key.as_ref().is_some_and(|key| {
            key.text.as_ref() == text && key.font == font && key.font_size == font_size
        });
        if !current {
            let key = GpuCanvasTextShapeKey {
                text: SharedString::from(text),
                font,
                font_size,
            };
            let run = TextRun {
                len: key.text.len(),
                font: key.font.clone(),
                color: black(),
                background_color: None,
                underline: None,
                strikethrough: None,
            };
            self.line = Some(text_system.shape_line(key.text.clone(), key.font_size, &[run], None));
            self.key = Some(key);
        }
        self.line.as_ref().expect("shape cache populated above")
    }

    /// Draw this text run at `origin`, where `origin` is the top-left logical
    /// pixel of the line box.
    pub fn draw(
        &mut self,
        ctx: &mut GpuCanvasTextContext<'_>,
        origin: Point<Pixels>,
        text: impl AsRef<str>,
        font: Font,
        font_size: Pixels,
        line_height: Pixels,
        color: Hsla,
    ) -> anyhow::Result<GpuCanvasTextMetrics> {
        let line = self.line(&ctx.text_system, text, font, font_size);
        ctx.draw_shaped_line_with_color(origin, line, line_height, TextAlign::Left, None, color)
    }

    /// Draw this text run anchored at `anchor`.
    ///
    /// `anchor_x` and `anchor_y` are normalized alignment factors:
    /// `0.0` means the left/top edge, `0.5` means center, and `1.0` means
    /// right/bottom.
    pub fn draw_aligned(
        &mut self,
        ctx: &mut GpuCanvasTextContext<'_>,
        anchor: Point<Pixels>,
        text: impl AsRef<str>,
        font: Font,
        font_size: Pixels,
        line_height: Pixels,
        color: Hsla,
        anchor_x: f32,
        anchor_y: f32,
    ) -> anyhow::Result<GpuCanvasTextMetrics> {
        let line = self.line(&ctx.text_system, text, font, font_size);
        let origin = point(
            anchor.x - line.width() * anchor_x,
            anchor.y - line_height * anchor_y,
        );
        ctx.draw_shaped_line_with_color(origin, line, line_height, TextAlign::Left, None, color)
    }
}

/// Text sprites generated for GPU-only canvas frames.
///
/// These sprites are produced by the same GPUI text system and sprite atlas as
/// ordinary scene text, but are owned by the retained GPU canvas frame instead
/// of requiring a GPUI view repaint.
#[derive(Clone, Debug, Default)]
pub struct GpuCanvasTextFrame {
    /// Monochrome glyph sprites.
    pub monochrome_sprites: Vec<MonochromeSprite>,
    /// Subpixel glyph sprites.
    pub subpixel_sprites: Vec<SubpixelSprite>,
    /// Emoji/color glyph sprites.
    pub polychrome_sprites: Vec<PolychromeSprite>,
}

impl GpuCanvasTextFrame {
    /// Remove all queued glyph sprites.
    pub fn clear(&mut self) {
        self.monochrome_sprites.clear();
        self.subpixel_sprites.clear();
        self.polychrome_sprites.clear();
    }

    /// Returns true when no text sprites were queued.
    pub fn is_empty(&self) -> bool {
        self.monochrome_sprites.is_empty()
            && self.subpixel_sprites.is_empty()
            && self.polychrome_sprites.is_empty()
    }

    pub(crate) fn finish(&mut self) {
        self.monochrome_sprites.sort_by_key(|sprite| {
            (
                sprite.order,
                atlas_texture_sort_key(sprite.tile.texture_id),
                sprite.tile.tile_id,
            )
        });
        self.subpixel_sprites.sort_by_key(|sprite| {
            (
                sprite.order,
                atlas_texture_sort_key(sprite.tile.texture_id),
                sprite.tile.tile_id,
            )
        });
        self.polychrome_sprites.sort_by_key(|sprite| {
            (
                sprite.order,
                atlas_texture_sort_key(sprite.tile.texture_id),
                sprite.tile.tile_id,
            )
        });
    }

    pub(crate) fn append(&mut self, mut other: GpuCanvasTextFrame) {
        self.monochrome_sprites
            .append(&mut other.monochrome_sprites);
        self.subpixel_sprites.append(&mut other.subpixel_sprites);
        self.polychrome_sprites
            .append(&mut other.polychrome_sprites);
    }
}

fn atlas_texture_sort_key(id: crate::AtlasTextureId) -> (u32, u32) {
    (id.kind as u32, id.index)
}

/// GPUI text painter available during [`GpuCanvasDriver::prepare_text`].
///
/// It uses the same shaping, glyph rasterization, sprite atlas, subpixel
/// selection, and sprite data structures as ordinary GPUI text. The produced
/// sprites are drawn by the platform renderer in the same GPU phase as the
/// owning canvas, without dirtying the GPUI view tree.
pub struct GpuCanvasTextContext<'a> {
    pub(crate) text_system: Arc<WindowTextSystem>,
    pub(crate) sprite_atlas: Arc<dyn PlatformAtlas>,
    pub(crate) bounds: Bounds<Pixels>,
    pub(crate) scale_factor: f32,
    pub(crate) content_mask: ContentMask<ScaledPixels>,
    pub(crate) background_appearance: WindowBackgroundAppearance,
    pub(crate) subpixel_rendering_supported: bool,
    pub(crate) text_rendering_mode: TextRenderingMode,
    pub(crate) order: DrawOrder,
    pub(crate) canvas_layer: GpuCanvasLayer,
    pub(crate) text_layer: GpuCanvasLayer,
    pub(crate) frame: &'a mut GpuCanvasTextFrame,
}

impl<'a> GpuCanvasTextContext<'a> {
    pub(crate) fn new(
        text_system: Arc<WindowTextSystem>,
        sprite_atlas: Arc<dyn PlatformAtlas>,
        bounds: Bounds<Pixels>,
        scale_factor: f32,
        content_mask: ContentMask<ScaledPixels>,
        background_appearance: WindowBackgroundAppearance,
        subpixel_rendering_supported: bool,
        text_rendering_mode: TextRenderingMode,
        order: DrawOrder,
        canvas_layer: GpuCanvasLayer,
        text_layer: GpuCanvasLayer,
        frame: &'a mut GpuCanvasTextFrame,
    ) -> Self {
        Self {
            text_system,
            sprite_atlas,
            bounds,
            scale_factor,
            content_mask,
            background_appearance,
            subpixel_rendering_supported,
            text_rendering_mode,
            order,
            canvas_layer,
            text_layer,
            frame,
        }
    }

    /// The layer used by the native canvas itself.
    pub fn canvas_layer(&self) -> GpuCanvasLayer {
        self.canvas_layer
    }

    /// Canvas bounds in logical pixels for the current frame.
    pub fn bounds(&self) -> Bounds<Pixels> {
        self.bounds
    }

    /// Window scale factor used for the current frame.
    pub fn scale_factor(&self) -> f32 {
        self.scale_factor
    }

    /// Effective text clip in scaled/device pixels.
    pub fn content_mask(&self) -> ContentMask<ScaledPixels> {
        self.content_mask
    }

    /// The layer where text emitted through this context will be composited.
    pub fn text_layer(&self) -> GpuCanvasLayer {
        self.text_layer
    }

    /// Draw a single-line text run at `origin`, where `origin` is the top-left
    /// logical pixel of the line box.
    pub fn draw_text(
        &mut self,
        origin: Point<Pixels>,
        text: impl AsRef<str>,
        font: Font,
        font_size: Pixels,
        line_height: Pixels,
        color: Hsla,
    ) -> anyhow::Result<GpuCanvasTextMetrics> {
        let text = SharedString::from(text.as_ref());
        let run = TextRun {
            len: text.len(),
            font,
            color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let line = self.text_system.shape_line(text, font_size, &[run], None);
        let width = line.width;
        self.draw_shaped_line(origin, &line, line_height, TextAlign::Left, None)?;
        Ok(GpuCanvasTextMetrics { width, line_height })
    }

    /// Draw a single-line text run anchored at `anchor`.
    ///
    /// `anchor_x` and `anchor_y` are normalized alignment factors:
    /// `0.0` means the left/top edge, `0.5` means center, and `1.0` means
    /// right/bottom.
    pub fn draw_text_aligned(
        &mut self,
        anchor: Point<Pixels>,
        text: impl AsRef<str>,
        font: Font,
        font_size: Pixels,
        line_height: Pixels,
        color: Hsla,
        anchor_x: f32,
        anchor_y: f32,
    ) -> anyhow::Result<GpuCanvasTextMetrics> {
        let text = SharedString::from(text.as_ref());
        let run = TextRun {
            len: text.len(),
            font,
            color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let line = self.text_system.shape_line(text, font_size, &[run], None);
        let width = line.width;
        let origin = point(
            anchor.x - width * anchor_x,
            anchor.y - line_height * anchor_y,
        );
        self.draw_shaped_line(origin, &line, line_height, TextAlign::Left, None)?;
        Ok(GpuCanvasTextMetrics { width, line_height })
    }

    /// Draw an already-shaped single line using the same glyph placement rules
    /// as GPUI's ordinary text path.
    pub fn draw_shaped_line(
        &mut self,
        origin: Point<Pixels>,
        line: &crate::ShapedLine,
        line_height: Pixels,
        align: TextAlign,
        align_width: Option<Pixels>,
    ) -> anyhow::Result<GpuCanvasTextMetrics> {
        self.draw_shaped_line_inner(origin, line, line_height, align, align_width, None)
    }

    /// Draw an already-shaped single line with one color, ignoring color runs
    /// stored in the shaped line. This is the fast path used by
    /// [`GpuCanvasTextRun`] when a label moves or changes color without changing
    /// its glyph layout.
    pub fn draw_shaped_line_with_color(
        &mut self,
        origin: Point<Pixels>,
        line: &crate::ShapedLine,
        line_height: Pixels,
        align: TextAlign,
        align_width: Option<Pixels>,
        color: Hsla,
    ) -> anyhow::Result<GpuCanvasTextMetrics> {
        self.draw_shaped_line_inner(origin, line, line_height, align, align_width, Some(color))
    }

    fn draw_shaped_line_inner(
        &mut self,
        origin: Point<Pixels>,
        line: &crate::ShapedLine,
        line_height: Pixels,
        align: TextAlign,
        align_width: Option<Pixels>,
        color_override: Option<Hsla>,
    ) -> anyhow::Result<GpuCanvasTextMetrics> {
        let layout = &line.layout;
        let mut glyph_origin = point(
            match (align, align_width) {
                (TextAlign::Center, Some(width)) => origin.x + (width - layout.width) / 2.,
                (TextAlign::Right, Some(width)) => origin.x + width - layout.width,
                _ => origin.x,
            },
            origin.y,
        );
        let padding_top = (line_height - layout.ascent - layout.descent) / 2.;
        let baseline_offset = point(px(0.), padding_top + layout.ascent);
        let mut prev_glyph_position = Point::default();
        let mut color = color_override.unwrap_or_else(black);
        let mut run_end = 0usize;
        let mut decoration_runs = line.decoration_runs.iter();

        for run in &layout.runs {
            let glyph_size = self
                .text_system
                .bounding_box(run.font_id, layout.font_size)
                .size;
            for glyph in &run.glyphs {
                glyph_origin.x += glyph.position.x - prev_glyph_position.x;
                prev_glyph_position = glyph.position;

                if color_override.is_none() && glyph.index >= run_end {
                    let mut style_run = decoration_runs.next();
                    while let Some(run) = style_run {
                        if glyph.index < run_end + (run.len as usize) {
                            break;
                        }
                        run_end += run.len as usize;
                        style_run = decoration_runs.next();
                    }
                    if let Some(style_run) = style_run {
                        run_end += style_run.len as usize;
                        color = style_run.color;
                    } else {
                        run_end = layout.len;
                    }
                }

                let glyph_bounds = Bounds {
                    origin: glyph_origin,
                    size: glyph_size,
                };
                if !glyph_bounds
                    .scale(self.scale_factor)
                    .intersects(&self.content_mask.bounds)
                {
                    continue;
                }

                let vertical_offset = point(px(0.0), glyph.position.y);
                let origin = glyph_origin + baseline_offset + vertical_offset;
                if glyph.is_emoji {
                    self.push_emoji(origin, run.font_id, glyph.id, layout.font_size)?;
                } else {
                    self.push_glyph(origin, run.font_id, glyph.id, layout.font_size, color)?;
                }
            }
        }

        Ok(GpuCanvasTextMetrics {
            width: layout.width,
            line_height,
        })
    }

    fn push_glyph(
        &mut self,
        origin: Point<Pixels>,
        font_id: crate::FontId,
        glyph_id: crate::GlyphId,
        font_size: Pixels,
        color: Hsla,
    ) -> anyhow::Result<()> {
        let glyph_origin = origin.scale(self.scale_factor);
        let quantized_origin = Point::new(
            round_half_toward_zero(glyph_origin.x.0 * SUBPIXEL_VARIANTS_X as f32)
                / SUBPIXEL_VARIANTS_X as f32,
            round_half_toward_zero(glyph_origin.y.0 * SUBPIXEL_VARIANTS_Y as f32)
                / SUBPIXEL_VARIANTS_Y as f32,
        );
        let subpixel_variant = Point::new(
            (quantized_origin.x.fract() * SUBPIXEL_VARIANTS_X as f32) as u8,
            (quantized_origin.y.fract() * SUBPIXEL_VARIANTS_Y as f32) as u8,
        );
        let integer_origin = quantized_origin.map(|c| ScaledPixels(c.trunc()));
        let subpixel_rendering = self.should_use_subpixel_rendering(font_id, font_size);
        let dilation = self.text_system.glyph_dilation_for_color(color);
        let params = RenderGlyphParams {
            font_id,
            glyph_id,
            font_size,
            subpixel_variant,
            scale_factor: self.scale_factor,
            is_emoji: false,
            subpixel_rendering,
            dilation,
        };

        let raster_bounds = self.text_system.raster_bounds(&params)?;
        if raster_bounds.is_zero() {
            return Ok(());
        }

        let tile = self
            .sprite_atlas
            .get_or_insert_with(&AtlasKey::from(params.clone()), &mut || {
                let (size, bytes) = self.text_system.rasterize_glyph(&params)?;
                Ok(Some((size, Cow::Owned(bytes))))
            })?
            .expect("Callback above only errors or returns Some");
        let bounds = Bounds {
            origin: integer_origin + raster_bounds.origin.map(Into::into),
            size: tile.bounds.size.map(Into::into),
        };

        if subpixel_rendering {
            self.frame.subpixel_sprites.push(SubpixelSprite {
                order: self.order,
                pad: 0,
                bounds,
                content_mask: self.content_mask,
                color,
                tile,
                transformation: TransformationMatrix::unit(),
            });
        } else {
            self.frame.monochrome_sprites.push(MonochromeSprite {
                order: self.order,
                pad: 0,
                bounds,
                content_mask: self.content_mask,
                color,
                tile,
                transformation: TransformationMatrix::unit(),
            });
        }
        Ok(())
    }

    fn push_emoji(
        &mut self,
        origin: Point<Pixels>,
        font_id: crate::FontId,
        glyph_id: crate::GlyphId,
        font_size: Pixels,
    ) -> anyhow::Result<()> {
        let glyph_origin = origin.scale(self.scale_factor);
        let integer_origin = glyph_origin.map(|c| ScaledPixels(round_half_toward_zero(c.0)));
        let params = RenderGlyphParams {
            font_id,
            glyph_id,
            font_size,
            subpixel_variant: Default::default(),
            scale_factor: self.scale_factor,
            is_emoji: true,
            subpixel_rendering: false,
            dilation: 0,
        };

        let raster_bounds = self.text_system.raster_bounds(&params)?;
        if raster_bounds.is_zero() {
            return Ok(());
        }

        let tile = self
            .sprite_atlas
            .get_or_insert_with(&AtlasKey::from(params.clone()), &mut || {
                let (size, bytes) = self.text_system.rasterize_glyph(&params)?;
                Ok(Some((size, Cow::Owned(bytes))))
            })?
            .expect("Callback above only errors or returns Some");

        self.frame.polychrome_sprites.push(PolychromeSprite {
            order: self.order,
            pad: 0,
            grayscale: false,
            bounds: Bounds {
                origin: integer_origin + raster_bounds.origin.map(Into::into),
                size: tile.bounds.size.map(Into::into),
            },
            corner_radii: Default::default(),
            content_mask: self.content_mask,
            tile,
            opacity: 1.0,
        });
        Ok(())
    }

    fn should_use_subpixel_rendering(&self, font_id: crate::FontId, font_size: Pixels) -> bool {
        if self.background_appearance != WindowBackgroundAppearance::Opaque {
            return false;
        }
        if !self.subpixel_rendering_supported {
            return false;
        }
        let mode = match self.text_rendering_mode {
            TextRenderingMode::PlatformDefault => self
                .text_system
                .recommended_rendering_mode(font_id, font_size),
            mode => mode,
        };
        mode == TextRenderingMode::Subpixel
    }
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

    /// Monotonic generation bumped when the backend replaces its native device/context
    /// after device loss or equivalent recovery.
    ///
    /// This is not a resize or render-target generation. Consumers should use
    /// `width`, `height`, and `render_target_format` to react to target changes.
    /// On Metal this value may remain stable for the renderer lifetime because
    /// Metal devices do not have the same device-lost recovery path as D3D/wgpu.
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
    /// Monotonic generation bumped when GPUI recreates D3D resources after device loss.
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
    /// Metal renderer/device generation.
    ///
    /// This normally remains stable for the renderer lifetime; target texture
    /// changes from resize are reported through `width`, `height`, and
    /// `render_target_format` instead.
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
    /// Monotonic generation bumped when GPUI recreates wgpu resources after device loss.
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

    /// Prepare text sprites for a frame that is going to present.
    ///
    /// The default implementation emits no text. Implementations that need
    /// GPUI-quality text in a GPU-only frame should queue it here.
    fn prepare_text(&mut self, _ctx: &mut GpuCanvasTextContext<'_>) -> anyhow::Result<()> {
        Ok(())
    }

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

    pub(crate) fn prepare_text(&self, ctx: &mut GpuCanvasTextContext<'_>) -> anyhow::Result<()> {
        self.0
            .try_borrow_mut()
            .map_err(|_| anyhow::anyhow!("gpu canvas driver is already borrowed"))?
            .prepare_text(ctx)
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
    /// Layer used for text submitted by this canvas.
    pub text_layer: GpuCanvasLayer,
    /// Retained driver handle.
    pub driver: GpuCanvasHandle,
}

/// A phase-composited native GPU element.
pub struct GpuCanvas {
    driver: GpuCanvasHandle,
    layer: GpuCanvasLayer,
    text_layer: GpuCanvasLayer,
    style: StyleRefinement,
}

impl GpuCanvas {
    /// Draw this canvas after ordinary GPUI scene primitives.
    pub fn over(mut self) -> Self {
        self.layer = GpuCanvasLayer::OverScene;
        self.text_layer = GpuCanvasLayer::OverScene;
        self
    }

    /// Composite text emitted by this canvas before ordinary GPUI scene primitives.
    pub fn text_under(mut self) -> Self {
        self.text_layer = GpuCanvasLayer::UnderScene;
        self
    }

    /// Composite text emitted by this canvas after ordinary GPUI scene primitives.
    pub fn text_over(mut self) -> Self {
        self.text_layer = GpuCanvasLayer::OverScene;
        self
    }

    /// Set the canvas and text layers independently.
    pub fn text_layer(mut self, layer: GpuCanvasLayer) -> Self {
        self.text_layer = layer;
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
        let text_layer = self.text_layer;
        let driver = self.driver.clone();
        style.paint(bounds, window, cx, move |window, _cx| {
            window.paint_gpu_canvas(layer, text_layer, bounds, driver);
        });
    }
}

impl Styled for GpuCanvas {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}
