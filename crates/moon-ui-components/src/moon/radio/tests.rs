//! Regression coverage for MoonRadio behavior and reviewed geometry.

use std::{cell::RefCell, rc::Rc};

use gpui::{
    Bounds, Context, InteractiveElement, IntoElement, KeyUpEvent, Keystroke, Modifiers,
    ParentElement, Pixels, Render, Styled, TestAppContext, VisualTestContext, Window, div, point,
    px, size,
};

use super::super::{MoonCheckbox, MoonThemeConfig, foundation::MoonSize};
use super::{MoonRadio, MoonRadioSize, RadioMetrics, moon_radio_click_value};

/// Catches `radio.rs:RadioMetrics::resolve` drifting from the checkbox tiers it shares: every tier
/// must resolve to the circle, text and spacing of the checkbox of the same size (16px circle,
/// 14/20 text, 8px gap for `Sm`; 20px, 16/24, 12px for `Md`), snapping `Xs` to `Sm` and `Lg` and
/// above to `Md`, with a 6 or 8px dot.
#[test]
fn radio_tiers_match_checkbox_tiers() {
    let tokens = MoonThemeConfig::moon_terminal().dark;
    for (tier, box_px, font_px, line_px, gap_px, dot_px) in [
        (MoonSize::Xs, 16., 14., 20., 8., 6.),
        (MoonSize::Sm, 16., 14., 20., 8., 6.),
        (MoonSize::Md, 20., 16., 24., 12., 8.),
        (MoonSize::Lg, 20., 16., 24., 12., 8.),
        (MoonSize::Xl, 20., 16., 24., 12., 8.),
        (MoonSize::Xxl, 20., 16., 24., 12., 8.),
    ] {
        let metrics = RadioMetrics::resolve(tier.into(), &tokens);
        assert_eq!(
            metrics.choice,
            crate::checkbox::MoonCheckboxMetrics::resolve(
                super::super::checkbox::tier_size(tier),
                &tokens
            ),
            "{tier:?} radio must share the {tier:?} checkbox metrics"
        );
        assert_eq!(metrics.choice.box_size, px(box_px));
        assert_eq!(metrics.choice.font_size, px(font_px));
        assert_eq!(metrics.choice.line_height, px(line_px));
        assert_eq!(metrics.choice.gap, px(gap_px));
        assert_eq!(metrics.dot_size, px(dot_px));
    }
}

/// Catches making `radio.rs:moon_radio_click_value` select disabled radios or ignore enabled
/// clicks, which would let unavailable choices change or leave available choices unselected.
#[test]
fn radio_click_value_respects_disabled_state() {
    assert_eq!(moon_radio_click_value(false), Some(true));
    assert_eq!(moon_radio_click_value(true), None);
}

const WRAPPING_LABEL: &str = "Close every open position when the stop price is reached";
const DESCRIPTION: &str = "Sent at market";

/// A checkbox over a radio with the same wrapping label and description in a narrow column.
struct CheckboxAndRadioHarness {
    size: MoonSize,
}

impl Render for CheckboxAndRadioHarness {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(180.))
            .flex()
            .flex_col()
            .child(
                MoonCheckbox::new("checkbox")
                    .size(self.size)
                    .label(WRAPPING_LABEL)
                    .description(DESCRIPTION)
                    .checked(true),
            )
            .child(
                MoonRadio::new("radio")
                    .size(self.size)
                    .label(WRAPPING_LABEL)
                    .description(DESCRIPTION)
                    .checked(true),
            )
    }
}

/// Returns the bounds of a control's box, label and description, found by `selectors` in that
/// order, each placed relative to the box's origin.
fn parts_from_box(cx: &mut VisualTestContext, selectors: [&'static str; 3]) -> [Bounds<Pixels>; 3] {
    let parts = selectors.map(|selector| {
        cx.debug_bounds(selector)
            .unwrap_or_else(|| panic!("{selector} must render"))
    });
    let box_origin = parts[0].origin;
    parts.map(|part| Bounds::new(part.origin - box_origin, part.size))
}

/// Catches a radio laying out its text differently from the checkbox of the same size: the circle
/// must match the box in size and the label and description must sit at the same offsets from it,
/// so a wrapping label keeps the circle on its first line and the description stacks the same way.
#[gpui::test]
fn radio_text_lays_out_like_the_checkbox_of_its_size(cx: &mut TestAppContext) {
    cx.update(crate::init);
    for (tier, line_px) in [(MoonSize::Sm, 20.), (MoonSize::Md, 24.)] {
        let window = cx.add_window(move |_, _| CheckboxAndRadioHarness { size: tier });
        let mut cx = VisualTestContext::from_window(window.into(), cx);
        cx.run_until_parked();

        let checkbox = parts_from_box(
            &mut cx,
            ["checkbox:box", "checkbox:label", "checkbox:description"],
        );
        let radio = parts_from_box(&mut cx, ["radio:box", "radio:label", "radio:description"]);
        assert!(
            radio[1].size.height >= px(line_px * 2.),
            "{tier:?} label must wrap for this case to mean anything"
        );
        assert_eq!(
            radio, checkbox,
            "{tier:?} radio must lay out like the checkbox"
        );
    }
}

/// Catches the radio's focus ring changing layout or losing its reviewed geometry: on keyboard focus
/// the radio must draw a ring whose outer edge sits 4px outside the circle on every side, as the
/// checkbox does, while the circle and label keep exactly their unfocused bounds.
#[gpui::test]
fn radio_focus_ring_surrounds_the_circle_without_moving_layout(cx: &mut TestAppContext) {
    cx.update(crate::init);
    for tier in [MoonSize::Sm, MoonSize::Md] {
        let window = cx.add_window(move |_, _| CheckboxAndRadioHarness { size: tier });
        let mut cx = VisualTestContext::from_window(window.into(), cx);
        cx.run_until_parked();

        let circle_before = cx.debug_bounds("radio:box").expect("circle must render");
        let label_before = cx.debug_bounds("radio:label").expect("label must render");
        assert!(cx.debug_bounds("radio:focus-ring").is_none());

        // The checkbox comes first in the harness, so the second stop is the radio.
        cx.update(|window, cx| {
            window.focus_next(cx);
            window.focus_next(cx);
        });
        cx.run_until_parked();

        assert!(cx.debug_bounds("checkbox:focus-ring").is_none());
        let ring = cx
            .debug_bounds("radio:focus-ring")
            .expect("focused radio must draw its ring");
        assert_eq!(cx.debug_bounds("radio:box"), Some(circle_before));
        assert_eq!(cx.debug_bounds("radio:label"), Some(label_before));
        assert_eq!(ring.origin, circle_before.origin - point(px(4.), px(4.)));
        assert_eq!(ring.size, circle_before.size + size(px(8.), px(8.)));
    }
}

/// A group of three labelled radios whose middle one is disabled, recording every change.
struct RadioGroupHarness {
    changes: Rc<RefCell<Vec<(&'static str, bool)>>>,
}

impl Render for RadioGroupHarness {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().children([("first", false), ("second", true), ("third", false)].map(
            |(id, disabled)| {
                let changes = self.changes.clone();
                MoonRadio::new(id)
                    .label(id)
                    .disabled(disabled)
                    .on_change(move |value, _, _| changes.borrow_mut().push((id, *value)))
            },
        ))
    }
}

/// Catches a radio that swallows focus on press or can't be used from the keyboard: pressing an
/// enabled radio must focus it (its listener stops the press before GPUI's own focus-on-press),
/// Tab must move on past a disabled radio to the next enabled one, and Space on the focused radio
/// must select it.
#[gpui::test]
fn pressing_a_radio_focuses_it_and_the_keyboard_selects(cx: &mut TestAppContext) {
    use gpui::AppContext as _;

    cx.update(crate::init);
    let changes = Rc::new(RefCell::new(Vec::new()));
    let window = cx.add_window({
        let changes = changes.clone();
        move |window, cx| {
            let view = cx.new(|_| RadioGroupHarness { changes });
            crate::Root::new(view, window, cx).bordered(false)
        }
    });
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();
    assert!(cx.debug_bounds("first:focus-ring").is_none());

    let label = cx.debug_bounds("first:label").expect("label must render");
    cx.simulate_click(label.center(), Modifiers::none());
    cx.run_until_parked();
    assert!(cx.debug_bounds("first:focus-ring").is_some());
    assert_eq!(&*changes.borrow(), &[("first", true)]);

    cx.simulate_keystrokes("tab");
    cx.run_until_parked();
    assert!(cx.debug_bounds("first:focus-ring").is_none());
    assert!(cx.debug_bounds("second:focus-ring").is_none());
    assert!(cx.debug_bounds("third:focus-ring").is_some());

    // GPUI clicks a focused element when Space is released; `simulate_keystrokes` only presses.
    cx.simulate_keystrokes("space");
    cx.simulate_event(KeyUpEvent {
        keystroke: Keystroke::parse("space").expect("space must parse"),
    });
    cx.run_until_parked();
    assert_eq!(&*changes.borrow(), &[("first", true), ("third", true)]);
}

/// A radio with an optional label in a flex row that shrinks the probe to the radio.
struct ProbedRadioHarness {
    size: MoonSize,
    label: Option<&'static str>,
}

impl Render for ProbedRadioHarness {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let radio = MoonRadio::new("probed").size(self.size).checked(true);
        crate::h_flex().child(div().debug_selector(|| "probed-control".into()).child(
            match self.label {
                Some(label) => radio.label(label),
                None => radio,
            },
        ))
    }
}

/// Catches a textless radio reserving text space (the text column and its 8 or 12px gap) for a
/// missing or empty label, or a checked dot drifting from its reviewed size or off the circle's
/// centre: the radio must be exactly its 16 or 20px circle, with a centred 6 or 8px dot.
#[gpui::test]
fn radio_without_text_is_its_circle_with_a_centred_dot(cx: &mut TestAppContext) {
    cx.update(crate::init);
    for (tier, box_px, dot_px) in [(MoonSize::Sm, 16., 6.), (MoonSize::Md, 20., 8.)] {
        for label in [None, Some("")] {
            let window = cx.add_window(move |_, _| ProbedRadioHarness { size: tier, label });
            let mut cx = VisualTestContext::from_window(window.into(), cx);
            cx.run_until_parked();

            let control = cx
                .debug_bounds("probed-control")
                .expect("probe must render");
            let circle = cx.debug_bounds("probed:box").expect("circle must render");
            let dot = cx
                .debug_bounds("probed:dot")
                .expect("checked radio must draw its dot");
            assert_eq!(
                control.size,
                size(px(box_px), px(box_px)),
                "{tier:?} radio with label {label:?} must be exactly its circle"
            );
            assert_eq!(dot.size, size(px(dot_px), px(dot_px)));
            assert_eq!(dot.center(), circle.center());
        }
    }
}

/// Catches changing tier metrics or applying legacy font scaling, which enlarges tier labels.
#[test]
fn radio_metrics_match_designer_reference() {
    use crate::moon::{MoonSize, MoonThemeTokens};
    let mut tokens = MoonThemeTokens::default();
    tokens.scale.ui = 2.0;
    tokens.scale.font = 3.0;
    tokens.scale.font_delta = 6.0;
    for (tier, outer, inner, font, line, gap, description_gap) in [
        (MoonSize::Xs, 32.0, 12.0, 28.0, 40.0, 16.0, 0.0),
        (MoonSize::Sm, 32.0, 12.0, 28.0, 40.0, 16.0, 0.0),
        (MoonSize::Md, 40.0, 16.0, 32.0, 48.0, 24.0, 4.0),
        (MoonSize::Lg, 40.0, 16.0, 32.0, 48.0, 24.0, 4.0),
        (MoonSize::Xl, 40.0, 16.0, 32.0, 48.0, 24.0, 4.0),
        (MoonSize::Xxl, 40.0, 16.0, 32.0, 48.0, 24.0, 4.0),
    ] {
        let metrics = MoonRadio::new("tier").size(tier).metrics(&tokens);
        assert_eq!(
            (
                f32::from(metrics.choice.box_size),
                f32::from(metrics.dot_size)
            ),
            (outer, inner)
        );
        assert_eq!(
            (
                f32::from(metrics.choice.font_size),
                f32::from(metrics.choice.line_height)
            ),
            (font, line)
        );
        assert_eq!(
            (
                f32::from(metrics.choice.gap),
                f32::from(metrics.choice.description_gap)
            ),
            (gap, description_gap)
        );
        tokens.scale.tier = tier;
        assert_eq!(
            MoonRadio::new("density").metrics(&tokens).choice.box_size,
            px(outer)
        );
    }
}

/// Catches ignoring explicit tiers or scaling Custom like a tier, breaking pinned radio sizes.
#[test]
fn radio_tiers_and_custom_scaling_survive() {
    use crate::moon::{MoonSize, MoonThemeTokens};
    let mut tokens = MoonThemeTokens::default();
    tokens.scale.ui = 2.0;
    tokens.scale.font = 3.0;
    tokens.scale.font_delta = 6.0;
    assert_eq!(
        MoonRadio::new("sm")
            .size(MoonRadioSize::Tier(MoonSize::Sm))
            .metrics(&tokens)
            .choice
            .box_size,
        px(32.0)
    );
    assert_eq!(
        MoonRadio::new("md")
            .size(MoonRadioSize::Tier(MoonSize::Md))
            .metrics(&tokens)
            .choice
            .box_size,
        px(40.0)
    );
    let metrics = MoonRadio::new("custom")
        .size(MoonRadioSize::Custom {
            dot_size: 14.0,
            font_size: 10.0,
            line_height: 13.0,
            gap: 7.0,
        })
        .metrics(&tokens);
    assert_eq!(
        (
            f32::from(metrics.choice.box_size),
            f32::from(metrics.choice.font_size),
            f32::from(metrics.choice.line_height)
        ),
        (28.0, 36.0, 45.0)
    );
}

/// Renders a described choice to expose the mark and text geometry.
struct DescribedRadio(crate::moon::MoonSize);

impl gpui::Render for DescribedRadio {
    /// Returns the radio whose geometry the description regression probes.
    fn render(
        &mut self,
        _: &mut gpui::Window,
        _: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        MoonRadio::new("described")
            .size(self.0)
            .label("Label")
            .description("Description")
    }
}

/// Catches centring the mark on the whole stack or removing its tier gap, visibly misaligning rows.
#[gpui::test]
fn radio_description_aligns_with_label(cx: &mut gpui::TestAppContext) {
    use crate::moon::{MoonSize, MoonTheme, ThemeMode};
    cx.update(crate::init);
    for mode in [ThemeMode::Dark, ThemeMode::Light] {
        cx.update(|cx| MoonTheme::set_mode(mode, cx));
        for (tier, gap) in [(MoonSize::Sm, 0.0), (MoonSize::Md, 2.0)] {
            let window = cx.add_window(move |_, _| DescribedRadio(tier));
            let mut visual = gpui::VisualTestContext::from_window(window.into(), cx);
            visual.run_until_parked();
            let mark = visual.debug_bounds("described:box").expect("mark");
            let label = visual.debug_bounds("described:label").expect("label");
            let description = visual
                .debug_bounds("described:description")
                .expect("description");
            assert_eq!(mark.center().y, label.center().y);
            assert_eq!(description.top() - label.bottom(), gpui::px(gap));
        }
    }
}
