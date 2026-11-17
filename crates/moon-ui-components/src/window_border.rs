// From:
// https://github.com/zed-industries/zed/blob/56daba28d40301ee4c05546fadb691d070b7b2b6/crates/gpui/examples/window_shadow.rs
use gpui::{
    AnyElement, App, Bounds, CursorStyle, Decorations, Edges, Hsla, InteractiveElement as _,
    IntoElement, MouseButton, ParentElement, Pixels, Point, RenderOnce, ResizeEdge, Size,
    Styled as _, Tiling, Window, div, point, prelude::FluentBuilder as _, px,
};

use crate::ActiveTheme;

#[cfg(not(target_os = "linux"))]
pub(crate) const SHADOW_SIZE: Pixels = px(0.0);
#[cfg(target_os = "linux")]
pub(crate) const SHADOW_SIZE: Pixels = px(12.0);
const BORDER_SIZE: Pixels = px(1.0);
/// Half-width of the resize hit band on each side of the visible frame (inner border).
const RESIZE_HIT_SIZE: Pixels = px(4.0);
pub(crate) const BORDER_RADIUS: Pixels = px(0.0);

/// Create a new window border.
pub fn window_border() -> WindowBorder {
    WindowBorder::new()
}

/// Renders a custom window border and shadow on Linux.
#[derive(IntoElement)]
pub struct WindowBorder {
    shadow_size: Pixels,
    resize_hit_size: Pixels,
    children: Vec<AnyElement>,
}

impl Default for WindowBorder {
    fn default() -> Self {
        Self {
            shadow_size: SHADOW_SIZE,
            resize_hit_size: RESIZE_HIT_SIZE,
            children: Vec::new(),
        }
    }
}

impl WindowBorder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the shadow size for typical Linux client-side decorations.
    ///
    /// Default: [`SHADOW_SIZE`]
    pub fn shadow_size(mut self, size: impl Into<Pixels>) -> Self {
        self.shadow_size = size.into();
        self
    }

    /// Set the resize hit band half-width around the visible inner frame edge.
    ///
    /// Default: [`RESIZE_HIT_SIZE`]
    pub fn resize_hit_size(mut self, size: impl Into<Pixels>) -> Self {
        self.resize_hit_size = size.into();
        self
    }
}

/// Per-side inset of the visible frame from the outer window bounds.
fn client_frame_insets(shadow_size: Pixels, tiling: &Tiling) -> Edges<Pixels> {
    let mut insets = Edges::all(shadow_size);
    if tiling.top {
        insets.top = px(0.0);
    }
    if tiling.bottom {
        insets.bottom = px(0.0);
    }
    if tiling.left {
        insets.left = px(0.0);
    }
    if tiling.right {
        insets.right = px(0.0);
    }
    insets
}

/// Get the window paddings.
pub fn window_paddings(window: &Window) -> Edges<Pixels> {
    let shadow_size = window.client_inset().unwrap_or(SHADOW_SIZE);
    match window.window_decorations() {
        Decorations::Server => Edges::all(px(0.0)),
        Decorations::Client { tiling } => client_frame_insets(shadow_size, &tiling),
    }
}

impl ParentElement for WindowBorder {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for WindowBorder {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let decorations = window.window_decorations();
        // Keep the platform client inset stable. When the window is tiled on all sides we stop drawing
        // shadow padding, but `set_client_inset` must still use the full shadow size. Clearing it
        // makes the first resize after restore double-count the shadow in `compute_outer_size`, and
        // the window jumps larger.
        let platform_inset = self.shadow_size;
        let visual_shadow = match decorations {
            Decorations::Client { tiling }
                if tiling.top && tiling.bottom && tiling.left && tiling.right =>
            {
                px(0.0)
            }
            _ => self.shadow_size,
        };
        let resize_hit_size = self.resize_hit_size;
        if matches!(decorations, Decorations::Client { .. }) {
            window.set_client_inset(platform_inset);
        }
        // Content space, like `mouse_position()` and the hit zones laid out below; the platform
        // window size would be off by the content zoom.
        let window_size = window.viewport_size();

        div()
            .id("window-backdrop")
            .bg(gpui::transparent_black())
            .map(|div| match decorations {
                Decorations::Server => div,
                // A block, like the server-decorated branch, and the frame below fills it with a
                // percentage size. This wrapper used to be a flex column with the frame as its
                // growing item, and under an X11 window manager that left every row of the window
                // that sizes itself with `w_full` at the width of its own content, in every window
                // of the application, from the moment a second window opened until the first
                // resize: a 1198 px root with a 639 px title bar, measured, while a row of
                // automatic width in the same column kept all 1198 px. Titlebar controls then
                // sit in the middle of the window, under the drag overlay computed for its full
                // width, and do not take a click. Why the flex wrapper loses the width is not
                // established; that it does, and that the block one does not, is.
                Decorations::Client { tiling, .. } => div
                    .overflow_hidden()
                    .bg(gpui::transparent_black())
                    .when(!(tiling.top || tiling.right), |div| {
                        div.rounded_tr(BORDER_RADIUS)
                    })
                    .when(!(tiling.top || tiling.left), |div| {
                        div.rounded_tl(BORDER_RADIUS)
                    })
                    .when(!tiling.top, |div| div.pt(visual_shadow))
                    .when(!tiling.bottom, |div| div.pb(visual_shadow))
                    .when(!tiling.left, |div| div.pl(visual_shadow))
                    .when(!tiling.right, |div| div.pr(visual_shadow)),
            })
            .size_full()
            .child(
                div()
                    .cursor(CursorStyle::default())
                    .map(|div| match decorations {
                        Decorations::Server => div.size_full(),
                        Decorations::Client { tiling } => div
                            // The backdrop's content box, which is the window less the shadow
                            // padding; the borders below are inside it.
                            .size_full()
                            .overflow_hidden()
                            .when(!(tiling.top || tiling.right), |div| {
                                div.rounded_tr(BORDER_RADIUS)
                            })
                            .when(!(tiling.top || tiling.left), |div| {
                                div.rounded_tl(BORDER_RADIUS)
                            })
                            .border_color(cx.theme().window_border)
                            .when(!tiling.top, |div| div.border_t(BORDER_SIZE))
                            .when(!tiling.bottom, |div| div.border_b(BORDER_SIZE))
                            .when(!tiling.left, |div| div.border_l(BORDER_SIZE))
                            .when(!tiling.right, |div| div.border_r(BORDER_SIZE))
                            .when(!tiling.is_tiled(), |div| {
                                div.shadow(vec![gpui::BoxShadow {
                                    color: Hsla {
                                        h: 0.,
                                        s: 0.,
                                        l: 0.,
                                        a: 0.3,
                                    },
                                    blur_radius: visual_shadow / 2.,
                                    spread_radius: px(0.),
                                    offset: point(px(0.0), px(0.0)),
                                    inset: false,
                                }])
                            }),
                    })
                    .on_mouse_move(|_e, _, cx| {
                        cx.stop_propagation();
                    })
                    .bg(gpui::transparent_black())
                    .children(self.children),
            )
            .when(matches!(decorations, Decorations::Client { .. }), |this| {
                let Decorations::Client { tiling, .. } = decorations else {
                    return this;
                };
                // Placed explicitly at the frame's corner, which is where `resize_hit_zones`
                // measures its zones from. In a block an absolute child with no insets is put
                // where it would have stood in the flow, below the frame, and the zones with it.
                let insets = client_frame_insets(platform_inset, &tiling);
                this.child(
                    div()
                        .absolute()
                        .top(insets.top)
                        .left(insets.left)
                        .size_full()
                        .children(resize_hit_zones(
                            window_size,
                            platform_inset,
                            resize_hit_size,
                            &tiling,
                        )),
                )
            })
    }
}

fn cursor_style_for_resize_edge(edge: ResizeEdge) -> CursorStyle {
    match edge {
        ResizeEdge::Top | ResizeEdge::Bottom => CursorStyle::ResizeUpDown,
        ResizeEdge::Left | ResizeEdge::Right => CursorStyle::ResizeLeftRight,
        ResizeEdge::TopLeft | ResizeEdge::BottomRight => CursorStyle::ResizeUpLeftDownRight,
        ResizeEdge::TopRight | ResizeEdge::BottomLeft => CursorStyle::ResizeUpRightDownLeft,
    }
}

/// One zone per resize edge and corner, on a narrow band around the visible frame: it sets the
/// cursor and starts the resize. `.cursor()` updates immediately on hitbox changes without
/// `window.refresh()` (PR #617).
///
/// A press starts ONE resize, and nothing else about the press changes: it still reaches
/// whatever lies under the band, as it always did, so a menu or a dialog open in the window is
/// still dismissed by it. The zones do not overlap — see [`resize_zone_rects`] — so exactly one
/// handler runs. The backdrop used to hit-test the same band a second time in a handler of its
/// own; that one is gone.
fn resize_hit_zones(
    window_size: Size<Pixels>,
    shadow_size: Pixels,
    hit_size: Pixels,
    tiling: &Tiling,
) -> Vec<AnyElement> {
    // Overlay is laid out in the padded content box; convert from window coords.
    let insets = client_frame_insets(shadow_size, tiling);
    let frame_origin = point(insets.left, insets.top);

    resize_zone_rects(window_size, shadow_size, hit_size, tiling)
        .into_iter()
        .map(|(edge, bounds)| {
            let origin = bounds.origin - frame_origin;
            div()
                .absolute()
                .left(origin.x)
                .top(origin.y)
                .w(bounds.size.width)
                .h(bounds.size.height)
                .cursor(cursor_style_for_resize_edge(edge))
                .on_mouse_down(MouseButton::Left, move |_, window, _| {
                    window.start_window_resize(edge);
                })
                .into_any_element()
        })
        .collect()
}

/// The rectangle of each resize zone, in window coordinates: a band of `hit_size` on either side
/// of the visible frame edge, cut into corner squares and the edge strips between them.
///
/// No two rectangles overlap. An edge strip stops where a corner square begins and runs to the
/// end of its side only where there is no corner, which is next to a tiled side. Each strip used
/// to run the full length of its side, so every corner lay under two strips as well as under its
/// own square, and one press there reached all three handlers and the backdrop's besides: four
/// move-resize requests to the window manager for one gesture, in the directions corner, side,
/// side, corner.
///
/// # Arguments
///
/// * `window_size` - Size of the window, in the space the zones are laid out in.
/// * `shadow_size` - Width of the shadow margin between the window edge and the visible frame.
/// * `hit_size` - Half-width of the band.
/// * `tiling` - Sides the window manager holds against a screen edge or another window; a tiled
///   side has no zone and no margin.
///
/// # Returns
///
/// The zones, edge strips before corner squares; empty for a window tiled on every side.
fn resize_zone_rects(
    window_size: Size<Pixels>,
    shadow_size: Pixels,
    hit_size: Pixels,
    tiling: &Tiling,
) -> Vec<(ResizeEdge, Bounds<Pixels>)> {
    if tiling.top && tiling.bottom && tiling.left && tiling.right {
        return Vec::new();
    }

    let insets = client_frame_insets(shadow_size, tiling);
    let inner_left = insets.left;
    let inner_right = window_size.width - insets.right;
    let inner_top = insets.top;
    let inner_bottom = window_size.height - insets.bottom;
    let band = hit_size + hit_size;

    let top_left = !tiling.top && !tiling.left;
    let top_right = !tiling.top && !tiling.right;
    let bottom_left = !tiling.bottom && !tiling.left;
    let bottom_right = !tiling.bottom && !tiling.right;
    // What a strip gives up at an end where a corner square stands.
    let trim = |corner: bool| if corner { band } else { px(0.0) };
    // A strip along one axis: from the band's outer start, less the corner at each end. Never
    // negative, for a window smaller than two corners.
    let strip = |start: Pixels, end: Pixels, corner_at_start: bool, corner_at_end: bool| {
        let from = start - hit_size + trim(corner_at_start);
        let length = (end + hit_size - trim(corner_at_end) - from).max(px(0.0));
        (from, length)
    };

    let mut zones = Vec::new();
    let mut push_zone = |edge: ResizeEdge, origin: Point<Pixels>, size: Size<Pixels>| {
        zones.push((edge, Bounds { origin, size }));
    };

    if !tiling.top {
        let (x, width) = strip(inner_left, inner_right, top_left, top_right);
        push_zone(
            ResizeEdge::Top,
            point(x, inner_top - hit_size),
            Size::new(width, band),
        );
    }
    if !tiling.bottom {
        let (x, width) = strip(inner_left, inner_right, bottom_left, bottom_right);
        push_zone(
            ResizeEdge::Bottom,
            point(x, inner_bottom - hit_size),
            Size::new(width, band),
        );
    }
    if !tiling.left {
        let (y, height) = strip(inner_top, inner_bottom, top_left, bottom_left);
        push_zone(
            ResizeEdge::Left,
            point(inner_left - hit_size, y),
            Size::new(band, height),
        );
    }
    if !tiling.right {
        let (y, height) = strip(inner_top, inner_bottom, top_right, bottom_right);
        push_zone(
            ResizeEdge::Right,
            point(inner_right - hit_size, y),
            Size::new(band, height),
        );
    }

    if top_left {
        push_zone(
            ResizeEdge::TopLeft,
            point(inner_left - hit_size, inner_top - hit_size),
            Size::new(band, band),
        );
    }
    if top_right {
        push_zone(
            ResizeEdge::TopRight,
            point(inner_right - hit_size, inner_top - hit_size),
            Size::new(band, band),
        );
    }
    if bottom_left {
        push_zone(
            ResizeEdge::BottomLeft,
            point(inner_left - hit_size, inner_bottom - hit_size),
            Size::new(band, band),
        );
    }
    if bottom_right {
        push_zone(
            ResizeEdge::BottomRight,
            point(inner_right - hit_size, inner_bottom - hit_size),
            Size::new(band, band),
        );
    }

    zones
}

#[cfg(test)]
mod tests;
