//! Regression coverage for segment colors, fitting, rendering, and interactions.

use std::{cell::RefCell, rc::Rc};

use gpui::{
    Context, InteractiveElement as _, IntoElement, Modifiers, ParentElement as _, Point, Render,
    ScrollDelta, ScrollWheelEvent, StatefulInteractiveElement as _, Styled as _, VisualTestContext,
    Window, div, px,
};

// Do not use `super::*`: the glob would import the `gpui::test` macro, causing `#[test]` to
// recursively expand into itself.
use super::{
    LABEL_CONTRAST_FLOOR, MoonAccent, MoonPalette, MoonSegmentDensity, MoonSegmentItem,
    MoonSegmentSelection, MoonSegmentedControl, SEGMENT_GAP, SEGMENT_LABEL_SIZE, SEGMENT_PAD_X,
    SegmentCellChrome, SegmentCellInteraction, SegmentFillFrame, contrast_ratio, fit_segment_item,
    segment_cell_chrome, segment_cell_interaction, segment_fill_frame,
};
use crate::moon::{MoonScale, MoonTheme, MoonThemeTokens};

/// Accent variants whose semantic colors must remain distinct and readable.
const ACCENTS: [MoonAccent; 4] = [
    MoonAccent::Amber,
    MoonAccent::Blue,
    MoonAccent::Green,
    MoonAccent::Red,
];

/// Catches changing the luminance arithmetic in `segment.rs:contrast_ratio`, which would make the
/// segment label fallback accept colors below the published WCAG contrast thresholds.
#[test]
fn contrast_ratio_matches_the_published_anchors() {
    assert!((contrast_ratio(0x000000, 0xFFFFFF) - 21.0).abs() < 0.01);
    assert!((contrast_ratio(0x808080, 0x808080) - 1.0).abs() < 0.01);
    assert!((contrast_ratio(0x767676, 0xFFFFFF) - 4.54).abs() < 0.01);
    assert!((contrast_ratio(0x595959, 0xFFFFFF) - 7.00).abs() < 0.01);
    // The published formula is symmetric and must identify the brighter color itself.
    assert_eq!(
        contrast_ratio(0x000000, 0xFFFFFF),
        contrast_ratio(0xFFFFFF, 0x000000)
    );
}

/// Catches making label text follow the accent hue unconditionally in
/// `segment.rs:MoonAccent::colors`, which would render several labels below the readability floor.
#[test]
fn every_accent_label_clears_the_contrast_floor_in_both_themes() {
    for p in [MoonPalette::TERMINAL, MoonPalette::LIGHT] {
        for accent in ACCENTS {
            let ratio = contrast_ratio(accent.colors(p).text, p.panel);
            assert!(
                ratio >= LABEL_CONTRAST_FLOOR,
                "{accent:?} label is {ratio:.2}:1 on the panel (is_light={}), under the \
                 {LABEL_CONTRAST_FLOOR}:1 floor",
                p.is_light()
            );
        }
    }
}

/// Catches mapping two variants to the same underline in `segment.rs:MoonAccent::colors`, which
/// would make differently accented segment strips indistinguishable.
#[test]
fn each_accent_keeps_its_own_underline() {
    for p in [MoonPalette::TERMINAL, MoonPalette::LIGHT] {
        for (i, a) in ACCENTS.iter().enumerate() {
            for b in &ACCENTS[i + 1..] {
                assert_ne!(
                    a.colors(p).underline,
                    b.colors(p).underline,
                    "{a:?} and {b:?} share an underline (is_light={})",
                    p.is_light()
                );
            }
        }
    }
}

/// Catches bypassing `self.accent` in `segment.rs:MoonSegmentedControl::render_with_theme`, which
/// would render a configured segment strip with the palette-wide default accent.
#[test]
fn render_resolves_colours_from_the_configured_accent() {
    let source = include_str!("../segment.rs");
    let implementation = source.split("#[cfg(test)]").next().unwrap_or(source);

    assert!(
        implementation.contains("self.accent.colors(")
            && implementation.contains("accent_underline_colored("),
        "selected-segment colours must be resolved from self.accent, not from a palette-wide role"
    );
}

/// `segment.rs:fit_segment_item` must preserve the exact maximum boundary and truncate a
/// one-character overflow without changing the final cell width. Removing the max-label fit lets
/// one anomalous preset stretch the toolbar and invalidates its pre-render row budget.
#[test]
fn fitted_item_preserves_the_boundary_and_ellipsizes_one_past_it() {
    let measure = |text: &str| text.chars().count() as f32;
    let chrome = 5.0;
    let min = 8.0;
    let max = 15.0;

    let exact = fit_segment_item("", "1234567890", min, max, chrome, measure, measure);
    let overflow = fit_segment_item("", "1234567890X", min, max, chrome, measure, measure);

    assert_eq!(exact, ("1234567890".to_string(), max));
    assert_eq!(overflow, ("123456789\u{2026}".to_string(), max));

    for palette in [MoonPalette::TERMINAL, MoonPalette::LIGHT] {
        for (ui, font, font_delta) in [(0.5, 1.75, 0.0), (2.5, 0.75, 4.0), (2.5, 0.25, 0.0)] {
            let mut tokens = MoonThemeTokens {
                palette,
                ..MoonThemeTokens::default()
            };
            tokens.scale = MoonScale {
                ui,
                font,
                font_delta,
                tier: Default::default(),
                zoom: 1.0,
            };
            let min = tokens.font_width(34.0);
            let max = tokens.font_width(104.0);
            let chrome = tokens.ui(SEGMENT_PAD_X) * 2.0 + tokens.ui(SEGMENT_GAP);
            let text_scale = tokens.font(SEGMENT_LABEL_SIZE) / SEGMENT_LABEL_SIZE;
            let measure = |text: &str| text.chars().count() as f32 * 7.0 * text_scale;
            let effective_min = min.max((chrome + measure("\u{2026}")).ceil());
            let effective_max = max.max(effective_min);
            let (label, width) = fit_segment_item(
                "",
                "a deliberately overlong preset value for scale coverage",
                min,
                max,
                chrome,
                measure,
                measure,
            );

            assert!(width >= effective_min && width <= effective_max);
            assert!(
                chrome + measure(&label) <= width,
                "fitted label overflowed at ui={ui}, font={font}, delta={font_delta}"
            );
            assert!(label.ends_with('\u{2026}'));
        }
    }
}

/// `segment.rs:segment_cell_interaction` must make disabled and replacement cells inert. Allowing
/// either state through dispatches a double-click or Ctrl+wheel to the preset while the user is
/// editing it, or while no trading core is available.
#[test]
fn disabled_and_replaced_cells_expose_no_native_interactions() {
    let active = SegmentCellInteraction {
        click: true,
        stop_click: false,
        scroll: true,
        tooltip: true,
    };
    let inert = SegmentCellInteraction {
        click: false,
        stop_click: false,
        scroll: false,
        tooltip: false,
    };
    let stopped = SegmentCellInteraction {
        stop_click: true,
        ..inert
    };

    assert_eq!(
        segment_cell_interaction(false, false, true, true, true),
        active
    );
    assert_eq!(
        segment_cell_interaction(true, false, true, true, true),
        stopped
    );
    assert_eq!(
        segment_cell_interaction(false, true, true, true, true),
        inert
    );
}

/// `segment.rs:segment_cell_chrome` must remove normal label padding from an inline replacement.
/// Reusing normal chrome for a replacement squeezes a 34px preset editor to only 12px at the
/// default scale, making the value effectively uneditable.
#[test]
fn replaced_cell_releases_its_full_width_to_the_inline_editor() {
    let tokens = MoonThemeTokens::default();
    let normal = segment_cell_chrome(&tokens, false, MoonSegmentDensity::Regular, true);
    let replaced = segment_cell_chrome(&tokens, true, MoonSegmentDensity::Regular, true);

    assert!(normal.gap > 0.0 && normal.pad_x > 0.0);
    assert_eq!(
        replaced,
        SegmentCellChrome {
            gap: 0.0,
            pad_x: 0.0,
        }
    );
}

/// Root view with active, disabled, and inline-replaced segment cells inside a clickable ancestor.
struct SegmentInteractionHarness {
    events: Rc<RefCell<Vec<(&'static str, usize)>>>,
}

impl Render for SegmentInteractionHarness {
    /// Render the three cell states with native click and scroll callbacks.
    ///
    /// Args:
    ///     _window: Test window.
    ///     _cx: Test view context.
    ///
    /// Returns:
    ///     The rendered ancestor and segmented control.
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let click_events = self.events.clone();
        let scroll_events = self.events.clone();
        let parent_events = self.events.clone();
        let segment = MoonSegmentedControl::new("segment-interaction-harness")
            .items([
                MoonSegmentItem::new("", "active").width(80.0),
                MoonSegmentItem::new("", "disabled")
                    .width(90.0)
                    .disabled(true),
                MoonSegmentItem::new("", "replaced").width(100.0),
            ])
            .on_click(move |index, _, _, _| {
                click_events.borrow_mut().push(("click", index));
            })
            .on_scroll(move |index, _, _, _| {
                scroll_events.borrow_mut().push(("scroll", index));
            })
            .replace_item(
                2,
                div()
                    .debug_selector(|| "moon-segment-editor".to_string())
                    .w_full()
                    .h_full(),
            );

        div()
            .id("segment-interaction-parent")
            .on_click(move |_, _, _| {
                parent_events.borrow_mut().push(("parent", usize::MAX));
            })
            .child(segment)
    }
}

/// `segment.rs:MoonSegmentedControl::render_with_theme` must render each resolved width, release
/// replacement content from label padding, dispatch the active index, and stop a disabled cell
/// before a clickable ancestor. Bypassing either plan would squeeze inline editors or let disabled
/// presets react to user input.
#[gpui::test]
fn rendered_cells_preserve_width_and_gate_native_interactions(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init);
    let events = Rc::new(RefCell::new(Vec::new()));
    let window = cx.add_window({
        let events = events.clone();
        move |_, _| SegmentInteractionHarness { events }
    });
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();

    let cells: Vec<_> = [
        "moon-segment-cell:0",
        "moon-segment-cell:1",
        "moon-segment-cell:2",
    ]
    .into_iter()
    .map(|selector| {
        cx.debug_bounds(selector)
            .expect("every segment cell must register rendered bounds")
    })
    .collect();
    assert_eq!(cells[0].size.width, px(80.0));
    assert_eq!(cells[1].size.width, px(90.0));
    assert_eq!(cells[2].size.width, px(100.0));
    let editor = cx
        .debug_bounds("moon-segment-editor")
        .expect("inline replacement must render");
    assert_eq!(editor.size.width, cells[2].size.width);

    cx.simulate_click(cells[0].center(), Modifiers::default());
    assert!(
        events.borrow().contains(&("click", 0)),
        "active cell must dispatch its native index"
    );
    events.borrow_mut().clear();

    cx.simulate_event(ScrollWheelEvent {
        position: cells[0].center(),
        delta: ScrollDelta::Lines(Point { x: 0.0, y: 1.0 }),
        ..Default::default()
    });
    assert_eq!(events.borrow().as_slice(), [("scroll", 0)]);
    events.borrow_mut().clear();

    cx.simulate_click(cells[1].center(), Modifiers::default());
    assert!(
        events.borrow().is_empty(),
        "disabled cell must neither dispatch nor bubble to its clickable ancestor"
    );

    cx.simulate_click(cells[2].center(), Modifiers::default());
    assert!(
        !events.borrow().iter().any(|event| event.0 == "click"),
        "inline replacement must not dispatch the segment click handler"
    );
}

/// Root view containing one previously fitted segment item.
struct FittedSegmentHarness {
    item: MoonSegmentItem,
}

impl Render for FittedSegmentHarness {
    /// Render the fitted item without recomputing its width.
    ///
    /// Args:
    ///     _window: Test window.
    ///     _cx: Test view context.
    ///
    /// Returns:
    ///     The rendered segmented control.
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        MoonSegmentedControl::new("fitted-segment-harness").item(self.item.clone())
    }
}

/// `segment.rs:MoonSegmentItem::fit_width` must expand a font-scaled ceiling when high UI scale
/// makes the component's own chrome wider, and the rendered cell must consume exactly that
/// reported width. Clamping back to the requested ceiling would clip the preset or shift adjacent
/// toolbar controls.
#[gpui::test]
fn fitted_segment_width_survives_high_ui_low_font_render(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init);
    let scale = MoonScale {
        ui: 2.5,
        font: 0.25,
        font_delta: 0.0,
        tier: Default::default(),
        zoom: 1.0,
    };
    let item = cx.update(|cx| {
        MoonTheme::global_mut(cx).scale = scale;
        MoonSegmentItem::new("", "a long preset").fit_width(cx, 34.0, 104.0)
    });
    let resolved = item.resolved_width();
    let mut tokens = MoonThemeTokens::default();
    tokens.scale = scale;
    let chrome = tokens.ui(SEGMENT_PAD_X) * 2.0 + tokens.ui(SEGMENT_GAP);

    assert!(resolved >= chrome.ceil());
    assert!(
        resolved > tokens.font_width(104.0),
        "effective width must exceed an impossible font-scaled ceiling"
    );

    let window = cx.add_window(move |_, _| FittedSegmentHarness { item: item.clone() });
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
    let cell = cx
        .debug_bounds("moon-segment-cell:0")
        .expect("fitted segment cell must render");

    assert_eq!(cell.size.width, px(resolved));
}

/// Catches flipping the `MoonSegmentedControl::new` default to the filled style, which would
/// repaint every existing toolbar strip that never asked for a selection style.
#[test]
fn a_new_control_keeps_the_underline_selection() {
    assert_eq!(
        MoonSegmentSelection::default(),
        MoonSegmentSelection::Underline
    );
    assert_eq!(
        MoonSegmentedControl::new("default-selection").selection,
        MoonSegmentSelection::Underline
    );
}

/// Catches `segment.rs:MoonAccent::fill_colors` measuring the label against the panel instead of
/// the fill, which would draw an illegible label on the selected Blue or Amber cell of the size and
/// sell strips. Green and Red have no palette role that clears the floor on their dark fills, so
/// for them the test pins the documented fallback: the highest-contrast candidate.
#[test]
fn filled_label_clears_the_contrast_floor_on_its_fill_in_both_themes() {
    for p in [MoonPalette::TERMINAL, MoonPalette::LIGHT] {
        for accent in ACCENTS {
            let (fill, label) = accent.fill_colors(p);
            assert_eq!(
                fill,
                accent.colors(p).underline,
                "{accent:?} fill must be the accent hue"
            );
            let ratio = contrast_ratio(label, fill);
            if matches!(accent, MoonAccent::Blue | MoonAccent::Amber) {
                assert!(
                    ratio >= LABEL_CONTRAST_FLOOR,
                    "{accent:?} filled label is {ratio:.2}:1 on its fill (is_light={}), under                      the {LABEL_CONTRAST_FLOOR}:1 floor",
                    p.is_light()
                );
            }
            for candidate in [p.on_accent, p.text, p.panel, p.shell] {
                assert!(
                    ratio >= LABEL_CONTRAST_FLOOR || ratio >= contrast_ratio(candidate, fill),
                    "{accent:?} filled label is not the most legible candidate (is_light={})",
                    p.is_light()
                );
            }
        }
    }
}

/// Catches `segment.rs:segment_fill_frame` outlining underline strips, or joining filled items
/// that were given a gap, either of which changes how an existing strip is drawn.
#[test]
fn only_a_gapless_filled_strip_joins_into_one_outline() {
    assert_eq!(
        segment_fill_frame(MoonSegmentSelection::Underline, 0.0),
        SegmentFillFrame::None
    );
    assert_eq!(
        segment_fill_frame(MoonSegmentSelection::Underline, 4.0),
        SegmentFillFrame::None
    );
    assert_eq!(
        segment_fill_frame(MoonSegmentSelection::Filled, 0.0),
        SegmentFillFrame::JoinedStrip
    );
    assert_eq!(
        segment_fill_frame(MoonSegmentSelection::Filled, 4.0),
        SegmentFillFrame::SeparateCells
    );
}

/// Root view with a gapless filled strip of three fixed-width items.
struct FilledStripHarness;

impl Render for FilledStripHarness {
    /// Render a filled strip whose middle item is selected.
    ///
    /// Args:
    ///     _window: Test window.
    ///     _cx: Test view context.
    ///
    /// Returns:
    ///     The rendered segmented control.
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        MoonSegmentedControl::new("filled-strip-harness")
            .selection(MoonSegmentSelection::Filled)
            .accent(MoonAccent::Blue)
            .item_gap(0.0)
            .items([
                MoonSegmentItem::new("B1", "0.1").width(60.0),
                MoonSegmentItem::new("B2", "0.5").width(60.0).selected(true),
                MoonSegmentItem::new("B3", "1.0").width(60.0),
            ])
    }
}

/// Catches the joined filled strip adding its separators or outline OUTSIDE the cells, which
/// would widen every cell and push a fitted toolbar strip past its pre-render budget.
#[gpui::test]
fn joined_filled_cells_abut_and_keep_their_widths(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init);
    let window = cx.add_window(|_, _| FilledStripHarness);
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();

    let cells: Vec<_> = [
        "moon-segment-cell:0",
        "moon-segment-cell:1",
        "moon-segment-cell:2",
    ]
    .into_iter()
    .map(|selector| {
        cx.debug_bounds(selector)
            .expect("every filled cell must register rendered bounds")
    })
    .collect();
    for cell in &cells {
        assert_eq!(cell.size.width, px(60.0));
    }
    assert_eq!(cells[1].origin.x, cells[0].origin.x + px(60.0));
    assert_eq!(cells[2].origin.x, cells[1].origin.x + px(60.0));
}

/// Catches a Compact tweak leaking into the default geometry, which would re-flow every existing
/// consumer of the control (analytics tabs, settings, chart popups) that never asked for density.
/// The oracle is the shipped design reference: 11px side padding and a 5px hotkey gap.
#[test]
fn regular_density_keeps_the_original_cell_chrome() {
    let tokens = MoonThemeTokens::default();
    let original = SegmentCellChrome {
        gap: tokens.ui(5.0),
        pad_x: tokens.ui(11.0),
    };

    assert_eq!(MoonSegmentDensity::default(), MoonSegmentDensity::Regular);
    assert_eq!(
        MoonSegmentedControl::new("default-density").density,
        MoonSegmentDensity::Regular
    );
    assert_eq!(
        MoonSegmentItem::new("", "x").density,
        MoonSegmentDensity::Regular
    );
    for has_hotkey in [true, false] {
        assert_eq!(
            segment_cell_chrome(&tokens, false, MoonSegmentDensity::Regular, has_hotkey),
            original
        );
    }
}

/// Catches Compact keeping the regular padding, or keeping the hotkey gap on a cell that shows no
/// hotkey, which leaves the toolbar preset strips as wide as before. The oracle is the mockup's
/// preset pill: 8px side padding, and an empty hotkey takes no room.
#[test]
fn compact_density_tightens_padding_and_drops_an_empty_hotkey_gap() {
    let tokens = MoonThemeTokens::default();

    assert_eq!(
        segment_cell_chrome(&tokens, false, MoonSegmentDensity::Compact, false),
        SegmentCellChrome {
            gap: 0.0,
            pad_x: tokens.ui(8.0),
        }
    );
    assert_eq!(
        segment_cell_chrome(&tokens, false, MoonSegmentDensity::Compact, true),
        SegmentCellChrome {
            gap: tokens.ui(5.0),
            pad_x: tokens.ui(8.0),
        }
    );
}

/// Root view with a compact, gapless, filled strip of two pre-fitted items.
struct CompactStripHarness {
    items: Vec<MoonSegmentItem>,
}

impl Render for CompactStripHarness {
    /// Render the fitted items in a compact filled strip without refitting them.
    ///
    /// Args:
    ///     _window: Test window.
    ///     _cx: Test view context.
    ///
    /// Returns:
    ///     The rendered segmented control.
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        MoonSegmentedControl::new("compact-strip-harness")
            .selection(MoonSegmentSelection::Filled)
            .density(MoonSegmentDensity::Compact)
            .accent(MoonAccent::Blue)
            .item_gap(0.0)
            .items(self.items.clone())
    }
}

/// Catches `MoonSegmentItem::fit_width` ignoring the item's density, which would leave a compact
/// strip exactly as wide as a regular one; and a compact filled strip drawing cells wider than
/// their fitted width, which would push the toolbar past its measured collapse budget. At a 1.0
/// scale an empty-hotkey cell must shed 2 x (11 - 8) px of padding plus the 5px gap.
#[gpui::test]
fn compact_cells_fit_narrower_and_render_at_their_fitted_width(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init);
    let (regular, compact) = cx.update(|cx| {
        MoonTheme::global_mut(cx).scale = MoonScale {
            ui: 1.0,
            font: 1.0,
            font_delta: 0.0,
            tier: Default::default(),
            zoom: 1.0,
        };
        let fit = |density, label: &str| {
            MoonSegmentItem::new("", label.to_string())
                .density(density)
                .fit_width(cx, 0.0, f32::MAX)
        };
        (
            [
                fit(MoonSegmentDensity::Regular, "50"),
                fit(MoonSegmentDensity::Regular, "250"),
            ],
            [
                fit(MoonSegmentDensity::Compact, "50"),
                fit(MoonSegmentDensity::Compact, "250").selected(true),
            ],
        )
    });
    for (regular, compact) in regular.iter().zip(&compact) {
        assert_eq!(
            regular.resolved_width() - compact.resolved_width(),
            2.0 * (11.0 - 8.0) + 5.0
        );
    }

    let widths: Vec<f32> = compact.iter().map(|item| item.resolved_width()).collect();
    let window = cx.add_window(move |_, _| CompactStripHarness {
        items: compact.to_vec(),
    });
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
    let first = cx
        .debug_bounds("moon-segment-cell:0")
        .expect("compact cell 0 must render");
    let second = cx
        .debug_bounds("moon-segment-cell:1")
        .expect("compact cell 1 must render");

    assert_eq!(first.size.width, px(widths[0]));
    assert_eq!(second.size.width, px(widths[1]));
    assert_eq!(second.origin.x, first.origin.x + px(widths[0]));
}
