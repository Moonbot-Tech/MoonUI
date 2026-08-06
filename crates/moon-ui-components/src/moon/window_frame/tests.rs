//! Regression tests for reusable window-frame title behavior.

/// Removing `.truncate()` from `window_frame.rs:MoonWindowFrame::title_cluster` lets a narrow
/// window title wrap onto a second line and overrun the fixed-height title bar.
#[test]
fn title_cluster_shrinks_and_truncates_without_wrapping() {
    let source = include_str!("../window_frame.rs");
    let method = source
        .split_once("pub fn title_cluster(")
        .and_then(|(_, tail)| tail.split_once("pub fn drag_handle("))
        .map(|(body, _)| body)
        .expect("title_cluster must remain before drag_handle");

    assert!(
        method.contains(".min_w_0()") && method.contains(".truncate()"),
        "title_cluster must shrink and apply GPUI's single-line ellipsis primitive"
    );
}
