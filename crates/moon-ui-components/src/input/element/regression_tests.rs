//! Single-line prepaint regressions, including nonzero glyph advances absent from NoopTextSystem.

use std::{cell::RefCell, rc::Rc, sync::Arc};

use gpui::{
    AppContext as _, Bounds, Context, Entity, FontId, GlyphId, InteractiveElement as _,
    IntoElement, Modifiers, ParentElement as _, Render, ShapedGlyph, ShapedLine, ShapedRun,
    Styled as _, TestAppContext, TextAlign, VisualTestContext, Window, point, px, size,
};

use super::TextElement;
use crate::{
    Root, h_flex,
    input::{InputState, display_map::LineLayout},
    moon::{MoonNumberInput, MoonSize, MoonTheme, ThemeMode},
};

/// Hosts the actual Xs number builder using caller-owned state and an optional unit.
struct NumberHarness {
    state: Entity<InputState>,
    unit: Option<&'static str>,
}

impl Render for NumberHarness {
    /// Return the compact numeric field in a content-sized mono row.
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let mut input = MoonNumberInput::new("scroll-regression")
            .state(&self.state)
            .range(0., 256.)
            .step(1.)
            .size(MoonSize::Xs);
        if let Some(unit) = self.unit {
            input = input.unit(unit);
        }
        h_flex()
            .min_w(px(120.))
            .font_family("Geist Mono")
            .debug_selector(|| "number-box".into())
            .child(input)
    }
}

/// Assert text geometry is inside the control before the persisted-offset clamp can hide a shift.
fn assert_visible(cx: &mut VisualTestContext, input: &Entity<InputState>) {
    let control = cx.debug_bounds("number-box").expect("number is rendered");
    cx.update(|_, cx| {
        let state = input.read(cx);
        assert_eq!(state.value().as_ref(), "10");
        assert_eq!(state.scroll_offset(), point(px(0.), px(0.)));
        let layout = state.last_layout.as_ref().expect("text is laid out");
        let painted = state.last_bounds.expect("text is painted");
        assert_eq!(painted.origin, state.input_bounds.origin);
        assert!(painted.top() >= control.top());
        assert!(painted.top() + layout.line_height <= control.bottom());
        let width = layout.lines[0].size(layout.line_height).width;
        let left = painted.left() + layout.alignment_offset(width);
        assert!(left >= state.input_bounds.left());
        assert!(left + width <= state.input_bounds.right());
    });
}

/// Catches applying a deferred y target to the sole line before persistence clamps it: the Xs
/// value disappears despite scroll_offset().y being zero, and must remain visible after blur.
#[gpui::test]
fn xs_number_ignores_deferred_vertical_scroll(cx: &mut TestAppContext) {
    cx.update(crate::init);
    for theme in [ThemeMode::Dark, ThemeMode::Light] {
        cx.update(|cx| MoonTheme::set_mode(theme, cx));
        for unit in [None, Some("ms")] {
            let slot = Rc::new(RefCell::new(None));
            let window = cx.add_window({
                let slot = slot.clone();
                move |window, cx| {
                    let state = cx.new(|cx| InputState::new(window, cx).default_value("10"));
                    *slot.borrow_mut() = Some(state.clone());
                    let view = cx.new(|_| NumberHarness { state, unit });
                    Root::new(view, window, cx).bordered(false)
                }
            });
            let input = slot.borrow().clone().expect("input is created");
            let mut visual = VisualTestContext::from_window(window.into(), cx);
            visual.run_until_parked();
            assert_visible(&mut visual, &input);
            let click = visual.update(|_, cx| input.read(cx).input_bounds.center());
            visual.simulate_click(click, Modifiers::none());
            visual.update(|window, cx| assert!(input.read(cx).focus_handle.is_focused(window)));
            assert_visible(&mut visual, &input);
            visual.simulate_keystrokes(if cfg!(target_os = "macos") {
                "cmd-a"
            } else {
                "ctrl-a"
            });
            visual.simulate_input("10");
            assert_visible(&mut visual, &input);
            visual.update(|_, cx| {
                input.update(cx, |state, cx| {
                    state.set_scroll_offset(point(px(0.), px(-16.)), cx);
                })
            });
            visual.run_until_parked();
            assert_visible(&mut visual, &input);
            visual.update(|window, _| window.blur());
            visual.run_until_parked();
            assert_visible(&mut visual, &input);
        }
    }
}

/// Supply fixed glyph advances for ASCII digits, independent of platform fonts.
/// Returns the same shaped-line representation consumed by production cursor layout.
fn numeric_line(text: &str, width: f32) -> ShapedLine {
    let mut line = ShapedLine::default();
    let layout = Arc::get_mut(&mut line).expect("new shaped layout is uniquely owned");
    layout.width = px(width);
    layout.len = text.len();
    layout.runs = vec![ShapedRun {
        font_id: FontId(0),
        glyphs: (0..text.len())
            .map(|index| ShapedGlyph {
                id: GlyphId(0),
                index,
                position: point(px(index as f32 * width / text.len() as f32), px(0.)),
                is_emoji: false,
            })
            .collect(),
    }];
    line.text = text.to_string().into();
    line
}

/// Catches stale horizontal follow hiding a fitting centered value after a shorter edit:
/// in a 60px value box, two 7px glyphs must occupy x=23..37 and the cursor must be at x=37.
/// Nonzero synthetic advances cover the real arithmetic that NoopTextSystem cannot exercise.
#[gpui::test]
fn centered_short_value_resets_stale_scroll_before_paint(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let slot = Rc::new(RefCell::new(None));
    let window = cx.add_window({
        let slot = slot.clone();
        move |window, cx| {
            let input = cx.new(|cx| InputState::new(window, cx).default_value("10"));
            *slot.borrow_mut() = Some(input.clone());
            Root::new(input, window, cx).bordered(false)
        }
    });
    let input = slot.borrow().clone().expect("input is created");
    let mut visual = VisualTestContext::from_window(window.into(), cx);
    visual.run_until_parked();
    visual.update(|window, cx| {
        let mut layout = input
            .read(cx)
            .last_layout
            .clone()
            .expect("initial layout exists");
        layout.line_height = px(16.);
        layout.content_width = px(60.);
        layout.text_align = TextAlign::Center;
        layout.lines = Rc::new(vec![
            LineLayout::new().lines(smallvec::smallvec![numeric_line("10", 14.)]),
        ]);
        input.update(cx, |state, _| {
            state.selected_range = (2..2).into();
            state.last_selected_range = None;
            state.deferred_scroll_offset = None;
            state.scroll_handle.set_offset(point(px(-100.), px(0.)));
        });
        let mut bounds = Bounds::new(point(px(0.), px(0.)), size(px(60.), px(16.)));
        let (cursor, offset, _) = TextElement::new(input.clone()).layout_cursor(
            &layout,
            &mut bounds,
            size(px(14.), px(16.)),
            window,
            cx,
        );
        assert_eq!(
            offset.x,
            px(0.),
            "fitting centered value must not inherit old scroll"
        );
        assert_eq!(bounds.left() + layout.alignment_offset(px(14.)), px(23.));
        assert_eq!(cursor.expect("cursor is laid out").left(), px(37.));
        // Twenty 7px digits still follow the 2px cursor: 140px - 58px = 82px scroll.
        let long_value = "01234567890123456789";
        layout.lines = Rc::new(vec![
            LineLayout::new().lines(smallvec::smallvec![numeric_line(long_value, 140.)]),
        ]);
        input.update(cx, |state, cx| {
            state.set_value(long_value, window, cx);
            state.last_selected_range = None;
            state.deferred_scroll_offset = None;
        });
        let mut bounds = Bounds::new(point(px(0.), px(0.)), size(px(60.), px(16.)));
        let (cursor, offset, _) = TextElement::new(input.clone()).layout_cursor(
            &layout,
            &mut bounds,
            size(px(140.), px(16.)),
            window,
            cx,
        );
        assert_eq!(offset.x, px(-82.));
        assert_eq!(
            cursor.expect("long value cursor is laid out").left(),
            px(58.)
        );
        // A 93px value fits a 100px box even though the left-aligned scroll extent includes
        // a 10px safety margin. It must keep its first glyph at zero and its cursor at 93.
        layout.text_align = TextAlign::Left;
        layout.content_width = px(100.);
        layout.lines = Rc::new(vec![
            LineLayout::new().lines(smallvec::smallvec![numeric_line("10", 93.)]),
        ]);
        input.update(cx, |state, cx| {
            state.set_value("10", window, cx);
            state.last_selected_range = None;
            state.deferred_scroll_offset = None;
        });
        let mut bounds = Bounds::new(point(px(0.), px(0.)), size(px(100.), px(16.)));
        let (cursor, offset, _) = TextElement::new(input.clone()).layout_cursor(
            &layout,
            &mut bounds,
            size(px(103.), px(16.)),
            window,
            cx,
        );
        assert_eq!(
            offset.x,
            px(0.),
            "cursor safety padding must not hide fitting text"
        );
        assert_eq!(cursor.expect("left cursor is laid out").left(), px(93.));
    });
}
