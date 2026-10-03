//! Pins the three documented `Output` orderings.
//!
//! A faster current run reports a positive shift, rows with no metadata sort
//! last, and Critical prints before Fluff.

use super::{FailKind, Importance, Output, PerfReport, TestMdata, Timings};
use std::num::NonZero;
use std::time::Duration;

/// Builds timings whose mean is exactly `mean_ms` milliseconds.
///
/// `Timings::iters_per_sec` reads `Duration::as_millis`, so this mean is an
/// exact millisecond count. The standard deviation is unused by the orderings
/// under test.
///
/// # Panics
/// Never panics.
fn timings(mean_ms: u64) -> Timings {
    Timings {
        mean: Duration::from_millis(mean_ms),
        stddev: Duration::from_millis(1),
    }
}

/// Builds metadata with `importance` and `weight`, leaving iterations unset.
///
/// `Output::success` writes the iteration count it is passed. `version` is the
/// current metadata version, which these tests never compare.
///
/// # Panics
/// Never panics.
fn mdata(importance: Importance, weight: u8) -> TestMdata {
    TestMdata {
        version: 0,
        iterations: None,
        importance,
        weight,
    }
}

/// Returns a non-zero iteration count.
///
/// # Panics
/// Panics when `n` is zero.
fn iters(n: usize) -> NonZero<usize> {
    NonZero::new(n).expect("iteration count is non-zero")
}

/// Records one successful benchmark named `name` and returns that run.
///
/// The iteration count is fixed at 10 so two runs with the same count cancel
/// it out of the shift ratio. Returns the single-row `Output`.
///
/// # Panics
/// Panics when the iteration count is zero. The count used here is 10.
fn one_success(name: &str, importance: Importance, weight: u8, mean_ms: u64) -> Output {
    let mut output = Output::blank();
    output.success(name, mdata(importance, weight), iters(10), timings(mean_ms));
    output
}

/// Returns `(max, mean, min)` for `importance` in `report`.
///
/// # Panics
/// Panics when that importance category is absent.
fn shift_of(report: &PerfReport, importance: Importance) -> (f64, f64, f64) {
    let delta = report
        .deltas
        .get(&importance)
        .expect("importance category is present");
    (delta.max, delta.mean, delta.min)
}

/// Returns test names in `output`'s current order.
///
/// # Panics
/// Never panics.
fn names_of(output: &Output) -> Vec<&str> {
    output
        .tests
        .iter()
        .map(|(name, ..)| name.as_str())
        .collect()
}

/// Shift of a run whose mean is `self_ms` against a baseline whose mean is `baseline_ms`.
///
/// Both runs use the same iteration count, so the iteration term in
/// `iters_per_sec` cancels and the documented shift is
/// `baseline_ms / self_ms - 1`.
fn expected_shift(self_ms: u64, baseline_ms: u64) -> f64 {
    // 100 and 200 are exact in f64. This is the documented ratio, not a call
    // into `compare_perf`.
    (baseline_ms as f64) / (self_ms as f64) - 1.
}

/// True when `actual` matches `expected` well inside one displayed tenth of a percent.
fn close_to(actual: f64, expected: f64) -> bool {
    (actual - expected).abs() < 1e-9
}

/// Catches dividing baseline iterations/sec by the current run again.
///
/// That ratio is negative when the current run is faster, and the report then
/// draws an up arrow on a regression.
///
/// # Panics
/// Panics if a category is missing. The assertion message includes the report.
#[test]
fn faster_self_reports_a_positive_shift() {
    const FAST_MS: u64 = 100;
    const SLOW_MS: u64 = 200;
    let faster = one_success("bench", Importance::Critical, 50, FAST_MS);
    let slower = one_success("bench", Importance::Critical, 50, SLOW_MS);

    let improved_shift = expected_shift(FAST_MS, SLOW_MS);
    let improved = faster.clone().compare_perf(slower.clone());
    let (max, mean, min) = shift_of(&improved, Importance::Critical);
    assert!(
        close_to(max, improved_shift)
            && close_to(mean, improved_shift)
            && close_to(min, improved_shift),
        "faster self must be +{improved_shift}, got max={max} mean={mean} min={min}"
    );
    let improved_text = improved.to_string();
    let improved_pct = format!("↑ {:.1}%", improved_shift.abs() * 100.);
    assert!(
        improved_text.contains(&improved_pct),
        "a faster run must display an up arrow ({improved_pct}): {improved_text}"
    );
    assert!(
        !improved_text.contains('↓'),
        "a faster run must not display a down arrow: {improved_text}"
    );

    let regressed_shift = expected_shift(SLOW_MS, FAST_MS);
    let regressed = slower.compare_perf(faster);
    let (max, mean, min) = shift_of(&regressed, Importance::Critical);
    assert!(
        close_to(max, regressed_shift)
            && close_to(mean, regressed_shift)
            && close_to(min, regressed_shift),
        "slower self must be {regressed_shift}, got max={max} mean={mean} min={min}"
    );
    let regressed_text = regressed.to_string();
    let regressed_pct = format!("↓ {:.1}%", regressed_shift.abs() * 100.);
    assert!(
        regressed_text.contains(&regressed_pct),
        "a slower run must display a down arrow ({regressed_pct}): {regressed_text}"
    );
    assert!(
        !regressed_text.contains('↑'),
        "a slower run must not display an up arrow: {regressed_text}"
    );
}

/// Catches sorting a metadata row after a row that has none.
///
/// The table would then lead with an unparsed benchmark and hide the real
/// results below it. Two metadata-less rows also stay in name order at the end.
#[test]
fn rows_without_metadata_sort_last() {
    let mut output = Output::blank();
    output.failure("zeta", None, None, FailKind::BadMetadata);
    output.success(
        "fluff-bench",
        mdata(Importance::Fluff, 1),
        iters(4),
        timings(100),
    );
    output.failure("beta", None, None, FailKind::Triage);
    output.sort();
    assert_eq!(
        names_of(&output),
        ["fluff-bench", "beta", "zeta"],
        "a Fluff row still comes before every metadata-less row, and those rows stay alphabetical"
    );

    let mut unsorted = Output::blank();
    unsorted.failure("no-meta", None, None, FailKind::BadMetadata);
    unsorted.success(
        "has-meta",
        mdata(Importance::Fluff, 1),
        iters(4),
        timings(100),
    );
    let text = unsorted.to_string();
    let meta_at = text
        .find("has-meta")
        .expect("display includes the metadata row");
    let none_at = text
        .find("no-meta")
        .expect("display includes the metadata-less row");
    assert!(
        meta_at < none_at,
        "display must print the metadata row first: {text}"
    );
}

/// Catches sorting `Importance` in derived ascending order, and reversing weight with it.
///
/// Fluff is 0 and Critical is 4, so ascending order prints the least important
/// benchmark at the top. Weight stays low-to-high inside one importance.
/// Display sorts its own copy, so the table matches even when the caller does not.
#[test]
fn critical_prints_before_fluff() {
    let mut output = Output::blank();
    output.success(
        "fluff-bench",
        mdata(Importance::Fluff, 80),
        iters(4),
        timings(100),
    );
    output.success(
        "critical-high-weight",
        mdata(Importance::Critical, 90),
        iters(4),
        timings(100),
    );
    output.success(
        "critical-low-weight",
        mdata(Importance::Critical, 10),
        iters(4),
        timings(100),
    );
    output.sort();
    assert_eq!(
        names_of(&output),
        ["critical-low-weight", "critical-high-weight", "fluff-bench"]
    );

    let mut unsorted = Output::blank();
    unsorted.success(
        "fluff-bench",
        mdata(Importance::Fluff, 80),
        iters(4),
        timings(100),
    );
    unsorted.success(
        "critical-bench",
        mdata(Importance::Critical, 10),
        iters(4),
        timings(100),
    );
    let text = unsorted.to_string();
    let critical_at = text
        .find("critical-bench")
        .expect("display includes the critical row");
    let fluff_at = text
        .find("fluff-bench")
        .expect("display includes the fluff row");
    assert!(
        critical_at < fluff_at,
        "Critical must print above Fluff: {text}"
    );
}
