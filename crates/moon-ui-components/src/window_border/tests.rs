use gpui::{Bounds, Pixels, ResizeEdge, Tiling, px, size};

use super::resize_zone_rects;

const WIDTH: f32 = 1200.0;
const HEIGHT: f32 = 675.0;
const HIT: f32 = 4.0;

fn zones(shadow: f32, tiling: Tiling) -> Vec<(ResizeEdge, Bounds<Pixels>)> {
    resize_zone_rects(size(px(WIDTH), px(HEIGHT)), px(shadow), px(HIT), &tiling)
}

fn untiled() -> Tiling {
    Tiling::default()
}

fn tiled_left_and_right() -> Tiling {
    Tiling {
        left: true,
        right: true,
        ..Tiling::default()
    }
}

/// The zones a point lies in, with the far edges of a rectangle left out so that two rectangles
/// that only touch do not both claim the point between them.
fn edges_at(zones: &[(ResizeEdge, Bounds<Pixels>)], x: f32, y: f32) -> Vec<ResizeEdge> {
    zones
        .iter()
        .filter(|(_, bounds)| {
            let left = f32::from(bounds.origin.x);
            let top = f32::from(bounds.origin.y);
            let right = left + f32::from(bounds.size.width);
            let bottom = top + f32::from(bounds.size.height);
            x >= left && x < right && y >= top && y < bottom
        })
        .map(|(edge, _)| *edge)
        .collect()
}

/// Walk the visible part of the band along all four sides and require exactly one zone under
/// every point. Two is the old fault — a strip reaching into a corner, which sends the window
/// manager more than one resize request for one press — and none is a gap where the window
/// cannot be grabbed at all.
fn assert_one_zone_everywhere_on_the_band(shadow: f32) {
    let zones = zones(shadow, untiled());
    // One pixel inside the visible frame edge, on each side.
    let (left, top) = (shadow + 1.0, shadow + 1.0);
    let (right, bottom) = (WIDTH - shadow - 1.0, HEIGHT - shadow - 1.0);
    let mut x = left;
    while x <= right {
        for y in [top, bottom] {
            let found = edges_at(&zones, x, y);
            assert_eq!(
                found.len(),
                1,
                "at ({x}, {y}) with shadow {shadow}: {found:?}"
            );
        }
        x += 1.0;
    }
    let mut y = top;
    while y <= bottom {
        for x in [left, right] {
            let found = edges_at(&zones, x, y);
            assert_eq!(
                found.len(),
                1,
                "at ({x}, {y}) with shadow {shadow}: {found:?}"
            );
        }
        y += 1.0;
    }
}

#[test]
fn every_point_of_the_band_lies_in_exactly_one_zone() {
    assert_one_zone_everywhere_on_the_band(0.0);
    assert_one_zone_everywhere_on_the_band(12.0);
}

/// The corner belongs to the corner zone, not to either of the strips that meet there: a strip
/// answering first would resize the window along one axis only.
#[test]
fn a_corner_is_the_corner_zone() {
    let zones = zones(0.0, untiled());
    assert_eq!(edges_at(&zones, WIDTH - 1.0, 1.0), [ResizeEdge::TopRight]);
    assert_eq!(
        edges_at(&zones, 1.0, HEIGHT - 1.0),
        [ResizeEdge::BottomLeft]
    );
    assert_eq!(edges_at(&zones, WIDTH / 2.0, 1.0), [ResizeEdge::Top]);
    assert_eq!(
        edges_at(&zones, WIDTH - 1.0, HEIGHT / 2.0),
        [ResizeEdge::Right]
    );
}

/// Next to a tiled side there is no corner, so the strip has to cover the end of its side
/// itself; trimmed there as well it would leave the last pixels of the edge dead.
#[test]
fn a_strip_reaches_the_end_of_its_side_next_to_a_tiled_side() {
    let zones = zones(12.0, tiled_left_and_right());
    let edges: Vec<ResizeEdge> = zones.iter().map(|(edge, _)| *edge).collect();
    assert_eq!(edges, [ResizeEdge::Top, ResizeEdge::Bottom]);
    assert_eq!(edges_at(&zones, 0.0, 13.0), [ResizeEdge::Top]);
    assert_eq!(edges_at(&zones, WIDTH - 1.0, 13.0), [ResizeEdge::Top]);
}

#[test]
fn a_window_tiled_on_every_side_has_no_zones() {
    let tiling = Tiling {
        top: true,
        bottom: true,
        left: true,
        right: true,
    };
    assert!(zones(12.0, tiling).is_empty());
}
