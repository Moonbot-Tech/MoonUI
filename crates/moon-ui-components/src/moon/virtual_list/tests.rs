//! Regression coverage for the range `MoonVirtualList` reports to `on_visible_range`.

use std::cell::RefCell;
use std::ops::Range;
use std::rc::Rc;

use super::MoonVirtualList;

/// Catches wiring the observer back into the item renderer in `virtual_list.rs`, the way it was
/// wired before: `uniform_list` renders `item_to_measure_index..+1` to MEASURE a row, from both
/// `request_layout` and `prepaint`, so an observer inside the renderer receives `0..1` twice per
/// frame before the real range. A consumer that evicts row state outside the reported range then
/// evicts every row but the first on every frame — in MoonTerminal's Connections tab that blurred
/// the core-name field one frame after the click, so the row could not be renamed at all.
#[gpui::test]
fn visible_range_observer_never_sees_the_measured_row(cx: &mut gpui::TestAppContext) {
    let ranges = observed_ranges(cx, 50, 200.0, 0.0, false, None, None);

    assert_eq!(
        ranges.len(),
        1,
        "the observer must run once per drawn frame, got {ranges:?}"
    );
    // 200 px of viewport over 20 px rows: rows 0..10 are the ones actually drawn.
    assert_eq!(ranges[0], 0..10);
}

/// Catches reporting `visible_range` instead of the range handed to the item renderer when the
/// list is flipped: with 50 rows the flipped list draws items `0..10` while its unflipped window
/// is `40..50`, so a consumer indexing its own data by the reported range would address the wrong
/// end of the list entirely.
#[gpui::test]
fn flipped_list_reports_the_range_it_renders(cx: &mut gpui::TestAppContext) {
    let ranges = observed_ranges(cx, 50, 200.0, 0.0, true, None, None);

    assert_eq!(
        ranges.len(),
        1,
        "the observer must run once per drawn frame, got {ranges:?}"
    );
    assert_eq!(ranges[0], 0..10);
}

/// Catches reporting the first N rows instead of the range under the scroll offset — the case the
/// two unscrolled tests above cannot separate. A consumer evicting row state outside the report
/// would then evict every row the user has actually scrolled to.
#[gpui::test]
fn scrolled_list_reports_the_rows_under_the_offset(cx: &mut gpui::TestAppContext) {
    let ranges = observed_ranges(cx, 50, 200.0, 0.0, false, Some(12), None);

    assert_eq!(
        ranges.len(),
        1,
        "the observer must report once per prepaint, got {ranges:?}"
    );
    // Row 12 pulled to the top of a 200 px viewport of 20 px rows: rows 12..22 are drawn.
    assert_eq!(ranges[0], 12..22);
}

/// Catches leaving the report inside the `item_count > 0` branch of `uniform_list.rs`: a list that
/// drops to zero rows would then never report, and a consumer would keep focus and open popups
/// pinned to rows that no longer exist. Drives the real transition — a populated frame, then the
/// frame that empties it — because that second frame is the whole reason the branch exists.
#[gpui::test]
fn emptied_list_still_reports_its_empty_range(cx: &mut gpui::TestAppContext) {
    use gpui::AppContext as _;

    cx.update(crate::init);
    let ranges = Rc::new(RefCell::new(Vec::new()));
    let sink = ranges.clone();
    let window = cx.add_window(move |_, _| ListHarness {
        item_count: 50,
        height: 200.0,
        padding: 0.0,
        y_flipped: false,
        scroll: super::MoonVirtualListScrollHandle::new(),
        ranges: sink,
    });
    cx.update_window(window.into(), |_, window, cx| {
        window.draw(cx).clear();
    })
    .unwrap();
    assert_eq!(
        ranges.borrow().last().cloned(),
        Some(0..10),
        "the populated frame must report its rows first"
    );

    ranges.borrow_mut().clear();
    window
        .update(cx, |view, _window, _cx| view.item_count = 0)
        .unwrap();
    cx.update_window(window.into(), |_, window, cx| {
        window.draw(cx).clear();
    })
    .unwrap();

    assert_eq!(*ranges.borrow(), vec![0..0]);
}

/// Catches gating the report on the PADDED viewport height instead of on the element's own.
/// Taffy floors the list at its own padding, so a padded list is 40 px tall with a padded box of
/// exactly zero — the boundary a `> 0` test on that box fails — while the content mask still
/// shows a row, and a consumer would be told nothing while the user looks straight at it.
#[gpui::test]
fn padded_list_still_reports_the_row_it_draws(cx: &mut gpui::TestAppContext) {
    // 20 px of padding floors the element at 40 px and its padded box at 0, yet scrolling row 12
    // to the top leaves first = floor((240 - 20) / 20) = 11 and last = ceil((240 + 0) / 20) = 12.
    let ranges = observed_ranges(cx, 50, 30.0, 20.0, false, Some(12), None);

    assert_eq!(ranges, vec![11..12]);
}

/// Catches reporting `0..0` for a list that still holds rows but is laid out at zero height — a
/// collapsed dock panel, a splitter mid-drag, the frame before first sizing. That report is
/// indistinguishable from the emptied-list one above, so a consumer would blur the row the user is
/// typing into every frame its container is squeezed shut.
#[gpui::test]
fn collapsed_list_reports_nothing_at_all(cx: &mut gpui::TestAppContext) {
    let aligned = observed_ranges(cx, 50, 0.0, 0.0, false, None, None);
    assert!(aligned.is_empty(), "expected silence, got {aligned:?}");

    // Off a row boundary the computed range is NOT empty — floor(250/20) = 12 and
    // ceil(250/20) = 13 straddle a row that has no pixels on screen — so an emptiness test alone
    // would let a collapsed list report `12..13` and evict every other row.
    let straddling = observed_ranges(cx, 50, 0.0, 0.0, false, None, Some(-250.0));
    assert!(
        straddling.is_empty(),
        "expected silence off a row boundary, got {straddling:?}"
    );
}

/// Root view drawing one virtual list of 20-pixel rows and recording every reported range.
struct ListHarness {
    item_count: usize,
    height: f32,
    padding: f32,
    y_flipped: bool,
    scroll: super::MoonVirtualListScrollHandle,
    ranges: Rc<RefCell<Vec<Range<usize>>>>,
}

impl gpui::Render for ListHarness {
    /// Draw the list inside a fixed box so the visible row count is a property of the test, not of
    /// the test window's size.
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        use gpui::{ParentElement as _, Styled as _};

        let sink = self.ranges.clone();
        gpui::div()
            .w(gpui::px(300.0))
            .h(gpui::px(self.height))
            .flex()
            .child(
                MoonVirtualList::new("probe", self.item_count, 20.0, |ix, _window, _cx| {
                    gpui::div().child(format!("row {ix}"))
                })
                // No border and no surface: an unpadded caller's viewport is then exactly the
                // height it passes in, instead of that height minus a one-pixel outline.
                .border(false)
                .surface(false)
                .padding(self.padding)
                .track_scroll(&self.scroll)
                .y_flipped(self.y_flipped)
                .on_visible_range(move |range, _window, _cx| sink.borrow_mut().push(range)),
            )
    }
}

/// Draw one frame of the harness and return the ranges its observer was handed.
fn observed_ranges(
    cx: &mut gpui::TestAppContext,
    item_count: usize,
    height: f32,
    padding: f32,
    y_flipped: bool,
    scroll_to_row: Option<usize>,
    scroll_offset_y: Option<f32>,
) -> Vec<Range<usize>> {
    use gpui::AppContext as _;

    cx.update(crate::init);
    let ranges = Rc::new(RefCell::new(Vec::new()));
    let sink = ranges.clone();
    let scroll = super::MoonVirtualListScrollHandle::new();
    let scroll_for_view = scroll.clone();
    let window = cx.add_window(move |_, _| ListHarness {
        item_count,
        height,
        padding,
        y_flipped,
        scroll: scroll_for_view,
        ranges: sink,
    });
    if let Some(first_row) = scroll_to_row {
        scroll.scroll_to_item(first_row, gpui::ScrollStrategy::Top);
    }
    if let Some(offset_y) = scroll_offset_y {
        scroll
            .0
            .borrow()
            .base_handle
            .set_offset(gpui::point(gpui::px(0.0), gpui::px(offset_y)));
    }
    // Opening the window already draws a frame; drop what it reported so the assertions below
    // describe exactly one frame.
    ranges.borrow_mut().clear();
    cx.update_window(window.into(), |_, window, cx| {
        window.draw(cx).clear();
    })
    .unwrap();

    let observed = ranges.borrow().clone();
    observed
}
