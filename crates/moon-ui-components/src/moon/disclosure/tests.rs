//! Regression coverage for the caret's poses, its passivity, and its scaling.

use super::{
    MoonDisclosure, MoonDisclosureDirection, moon_disclosure_click_next,
    moon_disclosure_rotation_turns,
};
use crate::moon::{MoonScale, MoonTheme};
use gpui::{Styled as _, px};

/// `disclosure.rs:moon_disclosure_rotation_turns` must turn the down-pointing asset toward the
/// RIGHT for a collapsed `RightDown` caret.
///
/// This fork rotates clockwise, so right is three quarters, not one. The plausible edit is
/// "a quarter turn is obviously enough" — `0.25` points the collapsed caret LEFT, which reads as
/// "this collapses" on a row that in fact expands, inverting the affordance. It compiles, lays out
/// identically and paints a perfectly crisp arrow, so nothing else notices.
#[test]
fn a_collapsed_right_down_caret_points_right() {
    assert_eq!(
        moon_disclosure_rotation_turns(MoonDisclosureDirection::RightDown, false),
        0.75
    );
    assert_eq!(
        moon_disclosure_rotation_turns(MoonDisclosureDirection::RightDown, true),
        0.0
    );
}

/// `disclosure.rs:moon_disclosure_rotation_turns` must leave a collapsed `DownUp` caret unrotated.
///
/// This is the pixel-parity guarantee `MoonSelectorPill` rests on: its caret is decorative and
/// permanently collapsed, so any non-zero value here silently re-aims the caret on every selector
/// pill in the app, with no state change anywhere to make the regression noticeable.
#[test]
fn a_collapsed_down_up_caret_is_drawn_unrotated() {
    assert_eq!(
        moon_disclosure_rotation_turns(MoonDisclosureDirection::DownUp, false),
        0.0
    );
    assert_eq!(
        moon_disclosure_rotation_turns(MoonDisclosureDirection::DownUp, true),
        0.5
    );
}

/// `disclosure.rs:moon_disclosure_click_next` must refuse a click on a passive or disabled caret.
///
/// Dropping the `interactive` guard lets a handler attached to a passive caret fire off the click
/// that is bubbling up to its parent row: the row toggles, the caret toggles back, and the net
/// effect is that clicking the row does nothing while clicking just beside it works. Dropping the
/// `disabled` guard lets a disabled section fold.
///
/// This test's NAME is registered as the `disclosure.click_behavior` contract in
/// `xtask/src/component_audit.rs` — renaming it here without renaming it there reddens CI.
#[test]
fn disclosure_click_is_inert_without_an_id_or_when_disabled() {
    assert_eq!(moon_disclosure_click_next(false, true, false), Some(true));
    assert_eq!(moon_disclosure_click_next(true, true, false), Some(false));
    assert_eq!(moon_disclosure_click_next(false, false, false), None);
    assert_eq!(moon_disclosure_click_next(true, false, false), None);
    assert_eq!(moon_disclosure_click_next(false, true, true), None);
}

/// `disclosure.rs:MoonDisclosure::caret_box` must leave the passive arm without a cursor style.
///
/// In this fork `should_insert_hitbox` inserts a hitbox for a cursor style, a hover style OR a
/// listener — an `ElementId` is not required. So the obvious cleanup, "both arms are the same box,
/// just always set `cursor_pointer`", hands the passive caret a hitbox that eats the click meant
/// for the header row it sits in. Every `MoonCollapsible` then stops toggling when the user clicks
/// the most natural target in the widget, and the caret still looks exactly right.
#[gpui::test]
fn a_passive_caret_takes_no_cursor(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init);

    let (passive, interactive) = cx.update(|cx| {
        (
            MoonDisclosure::glyph(false)
                .hover_color(0x00ff00)
                .caret_box(cx)
                .style()
                .mouse_cursor,
            MoonDisclosure::button("caret", false)
                .caret_box(cx)
                .style()
                .mouse_cursor,
        )
    });

    assert!(
        passive.is_none(),
        "a passive caret with a cursor takes a hitbox and swallows its row's click"
    );
    assert!(
        interactive.is_some(),
        "the interactive arm is the one that must look clickable"
    );
}

/// `disclosure.rs:MoonDisclosure::caret_box` must keep the box square and the SAME size in both
/// poses.
///
/// Rotation is free of layout cost here, so today a pose costs nothing. The plausible future edit
/// implements a pose by swapping in a differently-shaped asset, or by nudging with directional
/// padding instead of rotating — either shifts the neighbouring label by a pixel on every single
/// toggle. That jitter is invisible to every other test and to the type system.
#[gpui::test]
fn a_caret_box_is_square_and_identical_across_poses(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init);

    let (collapsed, expanded) = cx.update(|cx| {
        (
            MoonDisclosure::glyph(false)
                .size(11.0)
                .box_size(12.0)
                .caret_box(cx)
                .style()
                .size
                .clone(),
            MoonDisclosure::glyph(true)
                .size(11.0)
                .box_size(12.0)
                .caret_box(cx)
                .style()
                .size
                .clone(),
        )
    });

    assert_eq!(collapsed.width, collapsed.height, "the box must be square");
    assert_eq!(collapsed.width, expanded.width);
    assert_eq!(collapsed.height, expanded.height);
}

/// `disclosure.rs:MoonDisclosure::caret_box` must scale its box by the UI token exactly once.
///
/// Both adopting call sites now hand a design-reference number across a module boundary, and the
/// in-file `tokens.ui(..)` calls that made the scaling obvious are gone with them. A missed scale
/// and a double scale are the repo's documented number-one bug class, and at the default scale of
/// 1.0 they are indistinguishable — hence the deliberately odd 2.5 here.
#[gpui::test]
fn caret_geometry_scales_with_the_ui_token(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init);
    cx.update(|cx| {
        MoonTheme::global_mut(cx).scale = MoonScale {
            ui: 2.5,
            font: 1.0,
            font_delta: 0.0,
        };
    });

    let size = cx.update(|cx| {
        MoonDisclosure::glyph(false)
            .size(11.0)
            .box_size(12.0)
            .caret_box(cx)
            .style()
            .size
            .clone()
    });

    assert_eq!(size.width, Some(px(30.0).into()), "12.0 * 2.5, scaled once");
}
