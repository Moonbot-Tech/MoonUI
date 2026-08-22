//! Deterministic column ordering and width fitting for [`super::MoonDataTable`].

use std::collections::HashMap;

use super::{MIN_COLUMN_WIDTH, MoonDataTableColumn, MoonDataTableState, MoonDataTableWidthPolicy};

/// Resolve retained order, fixed-left precedence, and user widths for rendering.
///
/// Args:
///     columns: Current caller-provided column descriptors in source order.
///     state: Retained table state containing drag order and resized widths.
///
/// Returns:
///     Render-ordered descriptors suitable for width fitting and cell permutation.
pub(super) fn ordered_columns(
    columns: Vec<MoonDataTableColumn>,
    state: &MoonDataTableState,
) -> Vec<MoonDataTableColumn> {
    let mut ordered = if state.column_order.is_empty() {
        columns
    } else {
        let mut by_key = columns
            .iter()
            .cloned()
            .map(|column| (column.key.to_string(), column))
            .collect::<HashMap<_, _>>();
        let mut ordered = Vec::new();
        for key in &state.column_order {
            if let Some(column) = by_key.remove(key.as_ref()) {
                ordered.push(column);
            }
        }
        for column in columns {
            if by_key.remove(column.key.as_ref()).is_some() {
                ordered.push(column);
            }
        }
        ordered
    };

    ordered.sort_by_key(|column| !column.fixed_left);
    ordered
        .into_iter()
        .map(|mut column| {
            if let Some(width) = state.column_widths.get(column.key.as_ref()) {
                column.width = *width;
                column.user_sized = true;
            }
            column
        })
        .collect()
}

/// Compute render widths without mutating retained user widths.
///
/// Args:
///     columns: Ordered columns carrying declared or retained widths.
///     viewport_width: Measured horizontal viewport width.
///     row_header_width: Width reserved for the optional row header.
///     width_policy: Fit or preserve behavior for narrow viewports.
///
/// Returns:
///     Render-only column copies with the selected sizing policy applied.
pub(super) fn auto_width_columns(
    mut columns: Vec<MoonDataTableColumn>,
    viewport_width: f32,
    row_header_width: f32,
    width_policy: MoonDataTableWidthPolicy,
) -> Vec<MoonDataTableColumn> {
    let available = (viewport_width - row_header_width).max(0.0);
    // Keep base widths on the first frame before the viewport canvas has measured itself.
    if available <= 0.0 {
        return columns;
    }
    // User-sized and author-fixed columns keep their widths while untouched columns share
    // extra space. When every column is fixed, no stretching is needed.
    let is_pinned = |column: &MoonDataTableColumn| column.user_sized || column.no_grow;
    let fixed: f32 = columns
        .iter()
        .filter(|column| is_pinned(column))
        .map(|column| column.width)
        .sum();
    let flex: f32 = columns
        .iter()
        .filter(|column| !is_pinned(column))
        .map(|column| column.width)
        .sum();
    let remaining = (available - fixed).max(0.0);
    if flex > 0.0 && remaining > flex {
        let scale = remaining / flex;
        for column in columns.iter_mut().filter(|column| !is_pinned(column)) {
            column.width *= scale;
            column.fill = false;
        }
    } else if fixed + flex > available && width_policy == MoonDataTableWidthPolicy::Fit {
        // Fit mode scales render copies only; retained user widths remain unchanged and return
        // when the viewport widens. Preserve mode leaves the excess to horizontal scrolling.
        downscale_columns_to_available(&mut columns, available);
    }
    columns
}

/// Proportionally shrink render-only columns with a water-fill minimum.
///
/// Columns that would fall below [`MIN_COLUMN_WIDTH`] pin there and redistribute the remaining
/// deficit; if even all minimums overflow, the horizontal scroller owns the remainder.
///
/// Args:
///     columns: Render-only columns to resize.
///     available: Width available after the optional row header.
fn downscale_columns_to_available(columns: &mut [MoonDataTableColumn], available: f32) {
    let n = columns.len();
    if n == 0 {
        return;
    }
    if available <= n as f32 * MIN_COLUMN_WIDTH {
        for column in columns.iter_mut() {
            column.width = MIN_COLUMN_WIDTH;
            column.fill = false;
        }
        return;
    }
    let mut pinned = vec![false; n];
    let mut scale = 1.0_f32;
    // Each pass either pins at least one new column or finalizes the scale, so at most `n + 1`
    // passes are required.
    for _ in 0..=n {
        let pinned_sum = pinned.iter().filter(|p| **p).count() as f32 * MIN_COLUMN_WIDTH;
        let flex_base: f32 = columns
            .iter()
            .zip(&pinned)
            .filter(|(_, p)| !**p)
            .map(|(c, _)| c.width)
            .sum();
        if flex_base <= 0.0 {
            scale = 1.0;
            break;
        }
        scale = ((available - pinned_sum) / flex_base).max(0.0);
        let mut changed = false;
        for (column, pin) in columns.iter().zip(pinned.iter_mut()) {
            if !*pin && column.width * scale < MIN_COLUMN_WIDTH {
                *pin = true;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    for (column, pin) in columns.iter_mut().zip(&pinned) {
        column.width = if *pin {
            MIN_COLUMN_WIDTH
        } else {
            (column.width * scale).max(MIN_COLUMN_WIDTH)
        };
        column.fill = false;
    }
}
