//! Monochrome SVG icons whose strokes can render at a chosen width, so one authored file serves
//! every size.

use std::{collections::HashMap, sync::Arc};

use gpui::{
    App, Bounds, Element, ElementId, GlobalElementId, Hitbox, InspectorElementId,
    InteractiveElement, Interactivity, IntoElement, LayoutId, Pixels, SharedString,
    StyleRefinement, Styled, TransformationMatrix, Window,
};

/// Stroke-to-size ratios are rounded to millionths before they name a variant, so float noise
/// from UI zoom does not split one visual stroke across several cache entries.
const STROKE_RATIO_SCALE: f32 = 1_000_000.0;

/// An SVG icon loaded by asset path and tinted with its text colour, like GPUI's `svg()`, that can
/// also draw every stroke at a chosen rendered width.
///
/// The stroke width is rewritten against the icon's `viewBox` for the size the element is laid out
/// at, so a 12px and a 14px icon can share one file and still draw a 1.67px and a 2px stroke.
/// Rewritten files are cached per app, once per icon and stroke-to-size ratio.
pub(crate) struct MoonSvg {
    interactivity: Interactivity,
    path: SharedString,
    stroke_width: Option<Pixels>,
}

/// Creates a [`MoonSvg`] for the icon at `path` in the app's asset source.
///
/// Load icons by asset path from the embedded MoonAssets source, never by absolute file path, so
/// they also render in distributed builds.
#[track_caller]
pub(crate) fn moon_svg(path: impl Into<SharedString>) -> MoonSvg {
    MoonSvg {
        interactivity: Interactivity::new(),
        path: path.into(),
        stroke_width: None,
    }
}

impl MoonSvg {
    /// Draws every stroke `width` wide at the element's laid-out width. Without it the icon keeps
    /// the stroke authored in its file.
    pub(crate) fn stroke_width(mut self, width: Pixels) -> Self {
        self.stroke_width = Some(width);
        self
    }
}

impl Element for MoonSvg {
    type RequestLayoutState = ();
    type PrepaintState = Option<Hitbox>;

    fn id(&self) -> Option<ElementId> {
        self.interactivity.element_id.clone()
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        self.interactivity.source_location()
    }

    fn request_layout(
        &mut self,
        global_id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let layout_id = self.interactivity.request_layout(
            global_id,
            inspector_id,
            window,
            cx,
            |style, window, cx| window.request_layout(style, None, cx),
        );
        (layout_id, ())
    }

    fn prepaint(
        &mut self,
        global_id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<Hitbox> {
        self.interactivity.prepaint(
            global_id,
            inspector_id,
            bounds,
            bounds.size,
            window,
            cx,
            |_, _, hitbox, _, _| hitbox,
        )
    }

    /// Paints a rewritten stroke when possible, logging rewrite failures and using authored strokes.
    fn paint(
        &mut self,
        global_id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        hitbox: &mut Option<Hitbox>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let path = &self.path;
        let stroke_width = self.stroke_width;
        self.interactivity.paint(
            global_id,
            inspector_id,
            bounds,
            hitbox.as_ref(),
            window,
            cx,
            |style, window, cx| {
                let Some(color) = style.text.color else {
                    return;
                };
                let (key, bytes) = match stroke_width {
                    None => (path.clone(), None),
                    Some(stroke) => match stroked_svg(path, stroke, bounds.size.width, cx) {
                        Some((key, bytes)) => (key, Some(bytes)),
                        None => {
                            log::error!(
                                "failed to rewrite svg stroke {path}; painting authored stroke"
                            );
                            (path.clone(), None)
                        }
                    },
                };
                if let Err(error) = window.paint_svg(
                    bounds,
                    key,
                    bytes.as_deref(),
                    TransformationMatrix::default(),
                    color,
                    cx,
                ) {
                    log::error!("failed to paint svg {path}: {error:?}");
                }
            },
        )
    }
}

impl IntoElement for MoonSvg {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Styled for MoonSvg {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.interactivity.base_style
    }
}

impl InteractiveElement for MoonSvg {
    fn interactivity(&mut self) -> &mut Interactivity {
        &mut self.interactivity
    }
}

/// Stroke-rewritten icon files by icon path and rounded stroke-to-size ratio, holding the sprite
/// atlas key and bytes to paint, so each file is rewritten once per ratio rather than on every
/// paint.
#[derive(Default)]
struct StrokedSvgCache(HashMap<(SharedString, u32), (SharedString, Arc<[u8]>)>);

impl gpui::Global for StrokedSvgCache {}

/// Returns the stroke-to-size ratio of strokes `stroke` wide drawn at `drawn_width`, in millionths.
///
/// The ratio, not the pixel width, names a stroke variant: UI zoom scales the stroke and the drawn
/// size together, and the sprite atlas key already carries the drawn size. Returns `None` for a
/// negative stroke or an empty or non-finite width, which cannot be drawn.
fn stroke_ratio(stroke: Pixels, drawn_width: Pixels) -> Option<u32> {
    let ratio = stroke.as_f32() / drawn_width.as_f32();
    (ratio.is_finite() && ratio >= 0.0).then(|| (ratio * STROKE_RATIO_SCALE).round() as u32)
}

/// Returns the sprite atlas key for `path` drawn at the stroke `ratio` from [`stroke_ratio`].
///
/// The atlas caches rasterized icons by key and size, so every stroke variant of one file needs
/// its own key or it would reuse the sprite of whichever stroke painted first.
fn stroke_atlas_key(path: &str, ratio: u32) -> SharedString {
    SharedString::from(format!("{path}#stroke={ratio}"))
}

/// Returns the sprite atlas key and SVG bytes that draw `path` with strokes `stroke` wide at
/// `drawn_width`, rewriting and caching the file on first use.
///
/// Returns `None` when the drawn width is unusable, or the icon cannot be loaded from the asset
/// source, is not UTF-8, or has no parsable `viewBox`; the caller falls back to authored strokes.
fn stroked_svg(
    path: &SharedString,
    stroke: Pixels,
    drawn_width: Pixels,
    cx: &mut App,
) -> Option<(SharedString, Arc<[u8]>)> {
    let ratio = stroke_ratio(stroke, drawn_width)?;
    let cache_key = (path.clone(), ratio);
    if let Some(entry) = cx
        .try_global::<StrokedSvgCache>()
        .and_then(|cache| cache.0.get(&cache_key))
    {
        return Some(entry.clone());
    }

    let source = cx.asset_source().load(path).ok()??;
    let svg = with_rendered_stroke(
        std::str::from_utf8(&source).ok()?,
        ratio as f32 / STROKE_RATIO_SCALE,
        1.0,
    )?;
    let entry: (SharedString, Arc<[u8]>) = (stroke_atlas_key(path, ratio), svg.into_bytes().into());
    cx.default_global::<StrokedSvgCache>()
        .0
        .insert(cache_key, entry.clone());
    Some(entry)
}

/// Rewrites every `stroke-width` in `svg` so the stroke renders `stroke` wide when the SVG is
/// drawn `drawn_size` wide, scaling against the SVG's own `viewBox`.
///
/// Returns `None` when `svg` has no parsable `viewBox` width or an unterminated `stroke-width`.
pub(crate) fn with_rendered_stroke(svg: &str, stroke: f32, drawn_size: f32) -> Option<String> {
    let view_box_width: f32 = svg
        .split_once("viewBox=\"")?
        .1
        .split('"')
        .next()?
        .split(|character: char| character == ',' || character.is_ascii_whitespace())
        .filter(|part| !part.is_empty())
        .nth(2)?
        .parse()
        .ok()?;
    let width = format!("{:.4}", stroke * view_box_width / drawn_size);

    let mut rewritten = String::with_capacity(svg.len());
    let mut rest = svg;
    while let Some((before, after)) = rest.split_once("stroke-width=\"") {
        rewritten.push_str(before);
        rewritten.push_str("stroke-width=\"");
        rewritten.push_str(&width);
        rewritten.push('"');
        rest = after.split_once('"')?.1;
    }
    rewritten.push_str(rest);
    Some(rewritten)
}

#[cfg(test)]
mod tests;
