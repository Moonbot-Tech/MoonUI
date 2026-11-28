use std::time::{Duration, Instant};

use super::ResizeDiag;

const MS: Duration = Duration::from_millis(1);

#[test]
fn a_summary_covers_one_period_and_the_counters_start_over() {
    let start = Instant::now();
    let mut diag = ResizeDiag::default();

    diag.size_applied(start, 2 * MS, 10 * MS);
    diag.frame(false);
    diag.answered(5 * MS);
    diag.size_applied(start + 400 * MS, 4 * MS, 30 * MS);
    diag.frame(true);

    // The period is still running: nothing to print yet.
    assert_eq!(diag.take_summary(start + 999 * MS), None);

    let summary = diag
        .take_summary(start + 1000 * MS)
        .expect("the period ended");
    assert_eq!(summary.applied, 2);
    assert_eq!(summary.frames, 2);
    assert_eq!(summary.frames_dropped, 1);
    assert_eq!(summary.server.mean_ms(), 3.0);
    assert_eq!(summary.server.longest_ms(), 4.0);
    assert_eq!(summary.surface.mean_ms(), 20.0);
    assert_eq!(summary.surface.longest_ms(), 30.0);
    assert_eq!(summary.answer_wait.count(), 1);
    assert_eq!(summary.answer_wait.longest_ms(), 5.0);

    // The resize is over: no second line, however long the window lives.
    assert_eq!(diag.take_summary(start + 5000 * MS), None);
}

#[test]
fn frames_outside_a_resize_are_not_counted() {
    let start = Instant::now();
    let mut diag = ResizeDiag::default();

    // A window that is only drawing never opens a period and never prints.
    diag.frame(true);
    diag.frame(true);
    assert_eq!(diag.take_summary(start + 3000 * MS), None);

    diag.sizes_skipped(start, 3);
    diag.frame(true);
    let summary = diag
        .take_summary(start + 1000 * MS)
        .expect("the period ended");
    assert_eq!(summary.skipped, 3);
    assert_eq!(summary.frames, 1);
}

/// A deferred size counts as applied but costs no surface time; the rebuild that ends the burst
/// is the one surface entry, so the line tells a drag that rebuilt once from one that rebuilt on
/// every size.
#[test]
fn deferred_sizes_leave_the_surface_to_the_rebuild() {
    let start = Instant::now();
    let mut diag = ResizeDiag::default();

    diag.size_applied(start, MS, 4 * MS);
    diag.size_deferred(start + 16 * MS, 2 * MS);
    diag.size_deferred(start + 32 * MS, 3 * MS);
    diag.surface_rebuilt(start + 200 * MS, 6 * MS);

    let summary = diag
        .take_summary(start + 1000 * MS)
        .expect("the period ended");
    assert_eq!(summary.applied, 3);
    assert_eq!(summary.deferred, 2);
    assert_eq!(summary.server.mean_ms(), 2.0);
    assert_eq!(summary.surface.count(), 2);
    assert_eq!(summary.surface.mean_ms(), 5.0);
}
