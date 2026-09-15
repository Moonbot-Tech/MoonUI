//! Regression coverage for the coordinator-reviewed control scale and tier snapping.

use super::MoonSize;

/// Catches changing any tier's metric tuple, which would misalign consumers of the shared
/// scale. Expected rows come from the reviewed SCALE design table, independently of the match.
#[test]
fn control_metrics_match_reviewed_scale() {
    for (tier, expected) in [
        (MoonSize::Xs, [20., 12., 16., 4., 6., 4.]),
        (MoonSize::Sm, [24., 14., 20., 4., 8., 8.]),
        (MoonSize::Md, [32., 16., 24., 6., 12., 12.]),
        (MoonSize::Lg, [40., 18., 28., 8., 16., 12.]),
        (MoonSize::Xl, [48., 20., 28., 8., 20., 16.]),
        (MoonSize::Xxl, [56., 24., 32., 10., 24., 16.]),
    ] {
        let metrics = tier.control_metrics();
        assert_eq!(
            [
                metrics.height,
                metrics.font_size,
                metrics.line_height,
                metrics.radius,
                metrics.pad_x,
                metrics.gap,
            ],
            expected,
            "{tier:?} must match the reviewed design"
        );
    }
}

/// Catches lowering a larger tier's height below its predecessor, which makes increasing a
/// control's size shrink it instead.
#[test]
fn control_heights_increase_with_tier() {
    let tiers = [
        MoonSize::Xs,
        MoonSize::Sm,
        MoonSize::Md,
        MoonSize::Lg,
        MoonSize::Xl,
        MoonSize::Xxl,
    ];
    for pair in tiers.windows(2) {
        assert!(pair[0].control_metrics().height < pair[1].control_metrics().height);
    }
}

/// Catches lowering Sm to 23px, making the intended terminal tier miss WCAG 2.2 SC 2.5.8's
/// height floor, or raising Xs to 24px and losing the deliberately undersized dense-strip tier.
#[test]
fn sm_meets_wcag_2_5_8_height_floor_while_xs_requires_exception() {
    assert!(MoonSize::Sm.control_metrics().height >= 24.);
    assert!(MoonSize::Xs.control_metrics().height < 24.);
}

/// Catches replacing nearest-distance selection with the first supported tier: controls would
/// ignore exact matches, unsupported extremes, and closer tiers in unordered component lists.
#[test]
fn nearest_selects_supported_tiers_by_ordinal_distance() {
    use MoonSize::{Lg, Md, Sm, Xl, Xs, Xxl};

    for (requested, expected) in [(Xs, Sm), (Sm, Sm), (Md, Md), (Lg, Md), (Xl, Md), (Xxl, Md)] {
        assert_eq!(requested.nearest(&[Md, Sm]), expected);
    }
    assert_eq!(Sm.nearest(&[Xxl, Md, Xl]), Md);
    assert_eq!(Xl.nearest(&[Xs, Sm, Xxl]), Xxl);
    assert_eq!(Xs.nearest(&[Lg]), Lg);
    assert_eq!(Xxl.nearest(&[Sm]), Sm);
    assert_eq!(Md.nearest(&[Xxl, Md, Xs, Md]), Md);
}

/// Catches removing the smaller-tier tie key or using pixel distance: an equidistant requested
/// tier would unexpectedly grow, or change with the component's supported-list order.
#[test]
fn nearest_breaks_ties_down_independent_of_order() {
    use MoonSize::{Lg, Md, Sm, Xl, Xs, Xxl};

    for supported in [[Md, Xs], [Xs, Md]] {
        assert_eq!(Sm.nearest(&supported), Xs);
    }
    for supported in [[Xxl, Sm], [Sm, Xxl]] {
        assert_eq!(Lg.nearest(&supported), Sm);
    }
    assert_eq!(Md.nearest(&[Lg, Sm, Lg]), Sm);
    assert_eq!(Xl.nearest(&[Xxl, Lg]), Lg);
}

/// Catches silently accepting an empty supported list and returning an unsupported tier, which
/// would conceal a component configuration error until rendering.
#[test]
#[should_panic(expected = "MoonSize::nearest requires at least one supported tier")]
fn nearest_rejects_empty_supported_list() {
    MoonSize::Sm.nearest(&[]);
}
