//! Regression coverage for how a separator takes its size along the axis it spans.

use super::Separator;
use gpui::{AlignSelf, Styled as _};

/// `separator.rs:Separator::vertical` must span the flex line by stretching, never by measuring a
/// percentage of its parent.
///
/// A vertical rule divides groups laid out in a ROW, and such a row is normally content-height. A
/// percentage resolved against an indefinite parent gives zero, so the plausible future edit —
/// reinstating `div().h_full()` on the vertical base because "a separator should be full height" —
/// makes every vertical separator in an `h_flex().items_center()` row silently render as nothing,
/// with no panic and no layout shift to notice it by.
#[test]
fn a_vertical_separator_spans_its_row_by_stretching() {
    let mut sep = Separator::vertical();
    let style = sep.base.style();

    assert_eq!(style.align_self, Some(AlignSelf::Stretch));
    assert!(
        style.size.height.is_none(),
        "an explicit height on the base would win over the stretch and reintroduce the collapse"
    );
}

/// `separator.rs:Separator::horizontal` must NOT be converted to the same stretch.
///
/// The two axes are not symmetric: a horizontal rule spans the MAIN axis of the column it sits in,
/// which `w_full` resolves correctly, while `align_self` would size it across the column instead.
/// Applying the vertical fix to both arms — the obvious "make it consistent" edit — leaves a
/// horizontal separator inside a row with an auto width, i.e. invisible for the mirrored reason.
#[test]
fn a_horizontal_separator_is_not_stretched_across_its_column() {
    let mut sep = Separator::horizontal();

    assert_eq!(sep.base.style().align_self, None);
}
