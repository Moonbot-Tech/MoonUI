//! Regression coverage for data-table column sizing and overlay ownership.

// Do not use `super::*`: the glob would import the `gpui::test` macro, causing `#[test]` to
// recursively expand into itself.
use super::{MIN_COLUMN_WIDTH, MoonDataTable, MoonDataTableColumn, MoonDataTableWidthPolicy};

/// Build a test column with matching key and label.
fn col(key: &str, width: f32) -> MoonDataTableColumn {
    MoonDataTableColumn::new(key.to_string(), key.to_string(), width)
}

/// Collect computed widths for concise layout assertions.
fn widths(columns: &[MoonDataTableColumn]) -> Vec<f32> {
    columns.iter().map(|c| c.width).collect()
}

/// Catches scaling user-sized columns in `data_table.rs:MoonDataTable::auto_width_columns`, which
/// would overwrite a user's manual width in a wide viewport.
#[test]
fn auto_width_upscale_path_unchanged() {
    // In a wide viewport, untouched columns stretch while a user-sized column keeps its width.
    let mut fixed = col("a", 100.0);
    fixed.user_sized = true;
    let out = MoonDataTable::auto_width_columns(
        vec![fixed, col("b", 100.0)],
        500.0,
        0.0,
        MoonDataTableWidthPolicy::Fit,
    );
    assert_eq!(widths(&out), vec![100.0, 400.0]);
}

/// Catches skipping narrow-viewport scaling in
/// `data_table.rs:MoonDataTable::auto_width_columns`, which would hide columns beyond the visible
/// table width.
#[test]
fn auto_width_downscale_fits_narrow_viewport() {
    // In a narrow viewport, widths shrink to the available total without disappearing.
    let out = MoonDataTable::auto_width_columns(
        vec![col("a", 200.0), col("b", 200.0), col("c", 200.0)],
        300.0,
        0.0,
        MoonDataTableWidthPolicy::Fit,
    );
    let sum: f32 = widths(&out).iter().sum();
    assert!((sum - 300.0).abs() < 0.5, "sum={sum}");
    assert!(out.iter().all(|c| c.width >= MIN_COLUMN_WIDTH));
}

/// Catches removing the minimum-width water fill from
/// `data_table.rs:MoonDataTable::auto_width_columns`, which would collapse small columns below
/// their usable width.
#[test]
fn auto_width_downscale_respects_min_floor_water_fill() {
    // With 500 + 50 in 400 pixels, b pins at 40 and a takes the remainder.
    let out = MoonDataTable::auto_width_columns(
        vec![col("a", 500.0), col("b", 50.0)],
        400.0,
        0.0,
        MoonDataTableWidthPolicy::Fit,
    );
    assert_eq!(widths(&out), vec![360.0, MIN_COLUMN_WIDTH]);
}

/// Catches shrinking below `MIN_COLUMN_WIDTH` in
/// `data_table.rs:MoonDataTable::auto_width_columns`, which would make columns unreadable instead
/// of preserving scrollable overflow.
#[test]
fn auto_width_downscale_all_at_min_keeps_overflow() {
    // When even the minimums do not fit, every column stays at 40 and the remainder scrolls.
    let out = MoonDataTable::auto_width_columns(
        vec![col("a", 300.0), col("b", 300.0), col("c", 300.0)],
        100.0,
        0.0,
        MoonDataTableWidthPolicy::Fit,
    );
    assert_eq!(
        widths(&out),
        vec![MIN_COLUMN_WIDTH, MIN_COLUMN_WIDTH, MIN_COLUMN_WIDTH]
    );
}

/// Catches sizing an unmeasured viewport in `data_table.rs:MoonDataTable::auto_width_columns`,
/// which would collapse columns on the first frame.
#[test]
fn auto_width_unmeasured_viewport_keeps_base_widths() {
    // On the first frame the viewport is still zero, so base widths must remain untouched.
    let out = MoonDataTable::auto_width_columns(
        vec![col("a", 200.0), col("b", 120.0)],
        0.0,
        0.0,
        MoonDataTableWidthPolicy::Fit,
    );
    assert_eq!(widths(&out), vec![200.0, 120.0]);
}

/// Catches omitting `no_grow` from the fixed-width set in
/// `data_table.rs:MoonDataTable::auto_width_columns`, which would stretch a deliberately fixed
/// column in a wide viewport.
#[test]
fn auto_width_no_grow_column_keeps_base_width_while_others_stretch() {
    // At 600 pixels, no_grow joins the fixed set and the remaining columns become 250/100/250.
    // The complete vector distinguishes every filter failure, including layouts where all three
    // columns still appear to have grown.
    let out = MoonDataTable::auto_width_columns(
        vec![col("a", 100.0), col("b", 100.0).no_grow(), col("c", 100.0)],
        600.0,
        0.0,
        MoonDataTableWidthPolicy::Fit,
    );
    assert_eq!(widths(&out), vec![250.0, 100.0, 250.0]);
    let sum: f32 = widths(&out).iter().sum();
    assert!((sum - 600.0).abs() < 0.5, "sum={sum}");
}

/// Catches exempting `no_grow` columns from the shrink path in
/// `data_table.rs:MoonDataTable::auto_width_columns`, which would push the table tail into
/// unnecessary horizontal overflow.
#[test]
fn auto_width_no_grow_column_still_shrinks_on_narrow_viewport() {
    // `no_grow` means "do not stretch", not "hold this width at any cost". The sibling wide
    // viewport test proves that the flag also participates in sizing.
    let out = MoonDataTable::auto_width_columns(
        vec![col("a", 200.0), col("b", 200.0).no_grow()],
        200.0,
        0.0,
        MoonDataTableWidthPolicy::Fit,
    );
    let sum: f32 = widths(&out).iter().sum();
    assert!((sum - 200.0).abs() < 0.5, "sum={sum}");
    assert!(
        out[1].width < 200.0,
        "no_grow column must shrink, got {}",
        out[1].width
    );
}

/// Catches removing the reciprocal flag reset from `data_table.rs:MoonDataTableColumn::fill` or
/// `no_grow`, which would allow contradictory sizing instructions and unpredictable widths.
#[test]
fn no_grow_and_fill_are_mutually_exclusive() {
    // `fill` renders as min_w + flex_1, so the last builder call must win in both directions.
    let a = col("a", 100.0).fill().no_grow();
    assert!(a.no_grow && !a.fill);
    let b = col("b", 100.0).no_grow().fill();
    assert!(b.fill && !b.no_grow);
}

/// Catches routing `MoonDataTableWidthPolicy::Preserve` through the fit downscale branch, which
/// would compress Report columns and leave no horizontal overflow for the scrollbar to navigate.
#[test]
fn preserve_width_policy_keeps_overflowing_declared_widths() {
    let out = MoonDataTable::auto_width_columns(
        vec![col("a", 240.0), col("b", 180.0), col("c", 120.0)],
        300.0,
        0.0,
        MoonDataTableWidthPolicy::Preserve,
    );

    assert_eq!(widths(&out), vec![240.0, 180.0, 120.0]);
}

/// Catches restoring the hard-coded `MoonScrollbarVisibility::Hover` render argument, which would
/// make a consumer's always-visible scrollbar request compile but have no visible effect.
#[test]
fn horizontal_scrollbar_visibility_reaches_the_overlay() {
    let source = include_str!("../data_table.rs");
    let implementation = source.split("#[cfg(test)]").next().unwrap_or(source);
    let overlay_call = implementation
        .rsplit_once("moon_scrollbar_overlay_with_palette(")
        .expect("MoonDataTable must render its shared scrollbar overlay")
        .1;
    let overlay_args = overlay_call
        .split_once(") {")
        .expect("the scrollbar overlay call must feed its optional rendered child")
        .0;

    assert!(
        overlay_args.contains("horizontal_scrollbar_visibility")
            && !overlay_args.contains("MoonScrollbarVisibility::Hover"),
        "MoonDataTable must forward the builder-selected horizontal scrollbar visibility"
    );
}

/// Catches replacing `data_table.rs:MoonDataTable` root-owned context-menu dispatch with a local
/// child overlay, which would put menus behind neighboring panels and break outside dismissal.
#[test]
fn data_table_context_menu_uses_root_owned_overlay_layer() {
    let source = include_str!("../data_table.rs");
    let implementation = source.split("#[cfg(test)]").next().unwrap_or(source);

    assert!(
        implementation.contains("open_moon_context_menu_with_dismiss(")
            && !implementation.contains("MoonContextMenu::new("),
        "MoonDataTable must open context menus through the Root-owned window layer, not render local menu overlays as table children"
    );
}
