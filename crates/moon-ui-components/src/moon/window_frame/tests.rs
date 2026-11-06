//! Regression tests for reusable window-frame title behavior.

use gpui::{AppContext as _, Context, IntoElement, Render, TestAppContext, Window, div};
use std::{cell::Cell, rc::Rc};

/// A real window whose lifetime is controlled through the reusable frame button.
struct CloseHost;

impl Render for CloseHost {
    /// No layout is needed to test the button's shared invocation path.
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

/// Replacing request_close with remove_window would destroy the window despite its veto.
#[gpui::test]
fn frame_close_preserves_veto_and_marks_an_accepted_user_close(cx: &mut TestAppContext) {
    let allow = Rc::new(Cell::new(false));
    let requests = Rc::new(Cell::new(0));
    let window = cx.add_window({
        let allow = allow.clone();
        let requests = requests.clone();
        move |window, cx| {
            window.on_window_should_close(cx, move |_, _| {
                requests.set(requests.get() + 1);
                allow.get()
            });
            CloseHost
        }
    });
    window
        .update(cx, |_, window, cx| {
            super::FrameButton::Close.invoke(window, cx)
        })
        .unwrap();
    assert_eq!(requests.get(), 1);
    assert!(cx.update_window(window.into(), |_, _, _| ()).is_ok());
    allow.set(true);
    window
        .update(cx, |_, window, cx| {
            super::FrameButton::Close.invoke(window, cx)
        })
        .unwrap();
    assert_eq!(requests.get(), 2);
    assert!(cx.update_window(window.into(), |_, _, _| ()).is_err());
}

/// Removing `.truncate()` from `window_frame.rs:MoonWindowFrame::title_cluster` lets a narrow
/// window title wrap onto a second line and overrun the fixed-height title bar.
#[test]
fn title_cluster_shrinks_and_truncates_without_wrapping() {
    let source = include_str!("../window_frame.rs");
    let method = source
        .split_once("pub fn title_cluster(")
        .and_then(|(_, tail)| tail.split_once("pub fn drag_handle("))
        .map(|(body, _)| body)
        .expect("title_cluster must remain before drag_handle");

    assert!(
        method.contains(".min_w_0()") && method.contains(".truncate()"),
        "title_cluster must shrink and apply GPUI's single-line ellipsis primitive"
    );
}

/// Dropping the platform gate around `window_frame.rs:drag_region_div`'s mouse handler breaks the
/// Windows titlebar outright: the hit test reports the region as `HTCAPTION`, so the press arrives
/// as a non-client message that only reaches `DefWindowProc` — the system window move, the
/// double-click maximize, and the arming of the Close/Min/Max buttons — while it stays unconsumed.
/// The handler calls `cx.stop_propagation()`, so its mere presence on Windows consumes it.
#[test]
fn drag_region_leaves_the_press_to_the_os_on_windows() {
    let source = include_str!("../window_frame.rs");
    let helper = source
        .split_once("fn drag_region_div(")
        .map(|(_, tail)| tail)
        .expect("drag_region_div must remain in window_frame.rs");
    let gate = helper
        .find("cfg!(not(target_os = \"windows\"))")
        .expect("drag_region_div must gate its mouse handler off Windows");
    let handler = helper
        .find("on_mouse_down")
        .expect("drag_region_div must still handle the press on the other platforms");

    assert!(
        gate < handler,
        "the Windows gate must wrap the handler, not sit inside it"
    );
}
