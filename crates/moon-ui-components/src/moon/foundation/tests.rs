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

/// Catches the shared `sm` shadow drifting from the reviewed design, which every component that
/// asks for it inherits at once: two layers, `shadow_sm_01` 1px down with a 3px blur and no
/// spread, then `shadow_sm_02` 1px down with a 2px blur pulled in by 1px, neither offset
/// sideways. Every length follows the UI zoom, so a shadow under a zoomed control grows with it
/// rather than staying a hairline; reading the colours from anywhere but the roles would also
/// stop a colour mode from restyling it.
#[test]
fn shadow_sm_layers_match_the_reviewed_design() {
    use super::moon_shadow_sm;
    use crate::moon::{MoonColors, MoonThemeConfig};
    use gpui::px;

    for (scale, ui) in [(1.0, 1.0), (1.5, 1.5)] {
        let tokens = MoonThemeConfig::moon_terminal().with_ui_scale(scale).dark;
        let roles = MoonColors::DARK;
        let layers = moon_shadow_sm(roles, &tokens);

        assert_eq!(layers.len(), 2);
        assert_eq!(layers[0].color, roles.shadow_sm_01.into());
        assert_eq!(layers[0].offset, gpui::point(px(0.), px(ui)));
        assert_eq!(layers[0].blur_radius, px(3. * ui));
        assert_eq!(layers[0].spread_radius, px(0.));

        assert_eq!(layers[1].color, roles.shadow_sm_02.into());
        assert_eq!(layers[1].offset, gpui::point(px(0.), px(ui)));
        assert_eq!(layers[1].blur_radius, px(2. * ui));
        assert_eq!(layers[1].spread_radius, px(-ui));

        assert!(layers.iter().all(|layer| !layer.inset));
    }
}

/// Evaluates `cubic-bezier(x1, y1, x2, y2)` at `time` the slow, obvious way: walk the curve's own
/// parameter by bisection until its x reaches `time`, then read that point's y. This is the
/// definition the easing has to match, written out separately from the solver that has to be fast.
fn bezier_by_bisection(controls: [f32; 4], time: f32) -> f32 {
    let [x1, y1, x2, y2] = controls;
    let x = |s: f32| 3.0 * (1.0 - s).powi(2) * s * x1 + 3.0 * (1.0 - s) * s * s * x2 + s.powi(3);
    let y = |s: f32| 3.0 * (1.0 - s).powi(2) * s * y1 + 3.0 * (1.0 - s) * s * s * y2 + s.powi(3);

    let (mut low, mut high) = (0.0_f32, 1.0_f32);
    for _ in 0..60 {
        let mid = (low + high) * 0.5;
        if x(mid) < time {
            low = mid;
        } else {
            high = mid;
        }
    }
    y((low + high) * 0.5)
}

/// Catches `moon_cubic_bezier` drifting from the CSS timing function it names, which every
/// animation that asks for one inherits: it must pin both ends, never travel backwards, and match
/// the curve evaluated directly. Its Newton-Raphson solve is the part at risk — a wrong derivative
/// or a bad fallback still returns plausible numbers, so the easing is compared against
/// [`bezier_by_bisection`] rather than against remembered values. The diagonal curve is checked
/// too, since it has a closed form: it is the identity.
#[test]
fn cubic_bezier_reproduces_the_curve_it_names() {
    use super::moon_cubic_bezier;

    let diagonal = moon_cubic_bezier(0.25, 0.25, 0.75, 0.75);
    for step in 0..=10 {
        let time = step as f32 / 10.0;
        assert!(
            (diagonal(time) - time).abs() < 1e-3,
            "the diagonal curve must be the identity, but {time} eased to {}",
            diagonal(time)
        );
    }

    for controls in [
        [0.4, 0.0, 0.2, 1.0],
        [0.6, 0.0, 1.0, 0.4],
        [0.0, 0.9, 1.0, 0.1],
    ] {
        let [x1, y1, x2, y2] = controls;
        let easing = moon_cubic_bezier(x1, y1, x2, y2);
        assert_eq!(easing(0.0), 0.0, "{controls:?} must start at rest");
        assert_eq!(easing(1.0), 1.0, "{controls:?} must arrive exactly");

        let mut previous = 0.0;
        for step in 0..=100 {
            let time = step as f32 / 100.0;
            let eased = easing(time);
            assert!(
                (eased - bezier_by_bisection(controls, time)).abs() < 1e-3,
                "{controls:?} at {time} eased to {eased}, not the curve's own {}",
                bezier_by_bisection(controls, time)
            );
            assert!(
                eased >= previous,
                "{controls:?} must never travel backwards"
            );
            assert!((0.0..=1.0).contains(&eased));
            previous = eased;
        }
    }
}
