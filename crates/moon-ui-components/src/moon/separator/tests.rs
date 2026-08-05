//! Regression coverage for how `MoonSeparator` takes its size along the axis it spans.

use super::MoonSeparator;
use gpui::{AlignSelf, Styled as _, relative};

/// `separator.rs:MoonSeparator::line` must span a row by stretching, never by measuring a
/// percentage of it.
///
/// A vertical rule divides groups laid out in a ROW, and such a row is normally content-height. A
/// percentage resolved against an indefinite parent gives zero, so the plausible future edit —
/// restoring `line.h_full()` on the vertical arm because "a separator should be full height" —
/// makes every vertical separator disappear wherever the row does not carry an explicit height.
/// Nothing else catches that: the element still lays out, paints no pixels, and shifts nothing.
#[gpui::test]
fn a_vertical_separator_spans_its_row_by_stretching(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init);

    let style = cx.update(|cx| MoonSeparator::vertical().line(cx).style().clone());

    assert_eq!(style.align_self, Some(AlignSelf::Stretch));
    assert!(
        style.size.height.is_none(),
        "an explicit height would win over the stretch and reintroduce the collapse"
    );
}

/// `separator.rs:MoonSeparator::line` must NOT stretch the horizontal arm.
///
/// The two axes are not symmetric: a horizontal rule spans the MAIN axis of its column, which
/// `w_full` resolves correctly, while `align_self` would size it across the column instead.
/// Applying the vertical fix to both arms — the obvious "make it consistent" edit — leaves a
/// horizontal separator with an auto width, i.e. invisible for the mirrored reason.
#[gpui::test]
fn a_horizontal_separator_spans_its_column_by_percentage(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init);

    let style = cx.update(|cx| MoonSeparator::horizontal().line(cx).style().clone());

    assert_eq!(style.align_self, None);
    assert_eq!(style.size.width, Some(relative(1.).into()));
}
