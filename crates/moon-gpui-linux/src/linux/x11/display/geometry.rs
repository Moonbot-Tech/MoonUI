//! Monitor arithmetic for the X11 backend.
//!
//! Deliberately free of every X and toolkit type: the crate only builds for Linux, and these are
//! the rules that decide where a window opens. Depending on nothing but `std`, this file and its
//! tests can be compiled on their own on a host the crate itself does not build for.

/// Marks a display id as naming one RandR monitor rather than a whole X screen.
///
/// A whole-screen id is the bare X screen index, as it was before monitors were told apart. The
/// tag keeps the two apart: without it a monitor id would read as a screen index in the thousands
/// and window creation would ask the server for an X screen that does not exist.
const MONITOR_TAG: u64 = 1 << 63;
/// Position of the X screen index inside a monitor id.
const SCREEN_SHIFT: u32 = 32;
/// Width of the X screen index inside a monitor id.
const SCREEN_MASK: u64 = 0x7fff_ffff;

/// A rectangle in logical pixels, root-window coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Rect {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) width: f32,
    pub(crate) height: f32,
}

/// Build the id of one RandR monitor.
///
/// The monitor's name atom is the identity: it stays the same for as long as the X server runs,
/// while the monitor's position in the server's list moves whenever another one is plugged in.
///
/// # Arguments
///
/// * `x_screen_index` - Index of the X screen the monitor belongs to.
/// * `name_atom` - The monitor's RandR name atom.
///
/// # Returns
///
/// The raw display id, which [`screen_index_of`] maps back to `x_screen_index`.
pub(crate) fn monitor_display_id(x_screen_index: usize, name_atom: u32) -> u64 {
    MONITOR_TAG | ((x_screen_index as u64 & SCREEN_MASK) << SCREEN_SHIFT) | u64::from(name_atom)
}

/// Read the X screen index out of a display id of either kind.
///
/// # Arguments
///
/// * `raw_display_id` - A monitor id from [`monitor_display_id`] or a bare X screen index.
///
/// # Returns
///
/// The index of the X screen the display belongs to.
pub(crate) fn screen_index_of(raw_display_id: u64) -> usize {
    if raw_display_id & MONITOR_TAG != 0 {
        ((raw_display_id >> SCREEN_SHIFT) & SCREEN_MASK) as usize
    } else {
        raw_display_id as usize
    }
}

/// Convert a monitor rectangle from the server's device pixels to logical pixels.
///
/// Origin and size are both divided: window coordinates on X11 are global, so a monitor's origin
/// lives in the same logical space as the windows placed on it.
///
/// # Arguments
///
/// * `x`, `y` - The monitor's origin on the X screen, in device pixels.
/// * `width`, `height` - The monitor's size, in device pixels.
/// * `scale_factor` - The backend's scale factor. A value that is not a positive finite number
///   is read as `1.0`: it comes from user configuration (`Xft.dpi`), and dividing by it unchecked
///   would hand out an infinite or mirrored monitor.
///
/// # Returns
///
/// The logical rectangle, or `None` for a monitor with no area.
pub(crate) fn logical_rect(
    x: i16,
    y: i16,
    width: u16,
    height: u16,
    scale_factor: f32,
) -> Option<Rect> {
    if width == 0 || height == 0 {
        return None;
    }
    let scale = if scale_factor.is_finite() && scale_factor > 0.0 {
        scale_factor
    } else {
        1.0
    };
    Some(Rect {
        x: f32::from(x) / scale,
        y: f32::from(y) / scale,
        width: f32::from(width) / scale,
        height: f32::from(height) / scale,
    })
}

/// Pick the monitor a window belongs to.
///
/// The monitor holding the largest part of the window wins, the earlier one on a tie — callers
/// list the primary monitor first. A window that touches no monitor at all (dragged off every
/// edge, or saved on a monitor since unplugged) belongs to the nearest one, so it always has a
/// display rather than none.
///
/// # Arguments
///
/// * `window` - The window rectangle.
/// * `monitors` - The monitors of the window's X screen, in the same coordinate space.
///
/// # Returns
///
/// The index into `monitors`, or `None` when the list is empty.
pub(crate) fn containing(window: Rect, monitors: &[Rect]) -> Option<usize> {
    let mut best: Option<(usize, f32)> = None;
    for (index, monitor) in monitors.iter().enumerate() {
        let area = overlap_area(window, *monitor);
        if area > 0.0 && best.is_none_or(|(_, best_area)| area > best_area) {
            best = Some((index, area));
        }
    }
    if let Some((index, _)) = best {
        return Some(index);
    }

    let center = (
        window.x + window.width / 2.0,
        window.y + window.height / 2.0,
    );
    monitors
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            distance_squared(center, **a).total_cmp(&distance_squared(center, **b))
        })
        .map(|(index, _)| index)
}

/// Area two rectangles share, zero when they do not overlap.
fn overlap_area(a: Rect, b: Rect) -> f32 {
    let width = (a.x + a.width).min(b.x + b.width) - a.x.max(b.x);
    let height = (a.y + a.height).min(b.y + b.height) - a.y.max(b.y);
    if width > 0.0 && height > 0.0 {
        width * height
    } else {
        0.0
    }
}

/// Squared distance from a point to the nearest point of a rectangle.
fn distance_squared(point: (f32, f32), rect: Rect) -> f32 {
    let dx = point.0 - point.0.clamp(rect.x, rect.x + rect.width);
    let dy = point.1 - point.1.clamp(rect.y, rect.y + rect.height);
    dx * dx + dy * dy
}

#[cfg(test)]
mod tests;
