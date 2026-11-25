//! Regression coverage for timed and clicked dismissal of Moon notifications.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

use gpui::{
    AppContext as _, Context, IntoElement, Modifiers, MouseButton, MouseDownEvent, MouseUpEvent,
    PlatformInput, Render, TestAppContext, Window,
};

use super::MoonNotification;
use crate::{
    Root,
    moon::{MoonTheme, MoonWindowExt as _, ThemeMode},
};

/// Provide an application surface for the Root-owned notification layer.
struct NotificationHarness;

impl Render for NotificationHarness {
    /// Render the empty application surface.
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        gpui::div()
    }
}

/// Catches using the old five-second timer or silently disabling the timer for Moon actions.
#[gpui::test]
fn action_note_autohides_after_custom_delay(cx: &mut TestAppContext) {
    cx.update(crate::init);
    for mode in [ThemeMode::Dark, ThemeMode::Light] {
        cx.update(|cx| MoonTheme::set_mode(mode, cx));
        let (root, visual) = cx.add_window_view(|window, cx| {
            let view = cx.new(|_| NotificationHarness);
            Root::new(view, window, cx).bordered(false)
        });
        visual.update(|window, cx| {
            window.push_notification(
                MoonNotification::success("Rule added")
                    .action("Undo", |_, _| panic!("autohide must not run the action"))
                    .autohide_after(Duration::from_secs(6)),
                cx,
            );
        });
        visual.run_until_parked();
        visual
            .background_executor
            .advance_clock(Duration::from_millis(5_900));
        assert_eq!(root.read_with(visual, Root::notification_count), 1);

        visual
            .background_executor
            .advance_clock(Duration::from_millis(100));
        visual.run_until_parked();
        visual
            .background_executor
            .advance_clock(Duration::from_millis(200));
        assert_eq!(root.read_with(visual, Root::notification_count), 0);
    }
}

/// Catches an unbound action, a click that leaves the toast open, and a timer that closes it twice.
#[gpui::test]
fn action_click_runs_handler_and_dismisses(cx: &mut TestAppContext) {
    cx.update(crate::init);
    for mode in [ThemeMode::Dark, ThemeMode::Light] {
        cx.update(|cx| MoonTheme::set_mode(mode, cx));
        let events = Rc::new(RefCell::new(Vec::new()));
        let clicked = events.clone();
        let closed = events.clone();
        let (root, visual) = cx.add_window_view(|window, cx| {
            let view = cx.new(|_| NotificationHarness);
            Root::new(view, window, cx).bordered(false)
        });
        root.update_in(visual, |root, window, cx| {
            let note = MoonNotification::success("Rule added")
                .action("Undo", move |_, _| clicked.borrow_mut().push("undo"))
                .autohide_after(Duration::from_secs(6))
                .into_inner()
                .on_close(move |_, _| closed.borrow_mut().push("close"));
            root.push_notification(note, window, cx);
        });
        visual.run_until_parked();
        visual
            .background_executor
            .advance_clock(Duration::from_millis(300));
        visual.update(|window, _| window.refresh());
        visual.run_until_parked();
        let action = visual
            .debug_bounds("moon-notification-action")
            .expect("action button");

        visual.simulate_click(action.center(), Modifiers::none());
        assert_eq!(*events.borrow(), ["undo"]);
        visual.run_until_parked();
        visual
            .background_executor
            .advance_clock(Duration::from_millis(200));
        assert_eq!(root.read_with(visual, Root::notification_count), 0);
        assert_eq!(*events.borrow(), ["undo", "close"]);

        visual
            .background_executor
            .advance_clock(Duration::from_secs(6));
        visual
            .background_executor
            .advance_clock(Duration::from_millis(200));
        assert_eq!(*events.borrow(), ["undo", "close"]);
    }
}

/// Catches relying on the next render to disable the button, which lets a double click run it twice.
#[gpui::test]
fn action_runs_once_on_double_click_before_render(cx: &mut TestAppContext) {
    cx.update(crate::init);
    for mode in [ThemeMode::Dark, ThemeMode::Light] {
        cx.update(|cx| MoonTheme::set_mode(mode, cx));
        let runs = Rc::new(Cell::new(0));
        let clicked = runs.clone();
        let (_, visual) = cx.add_window_view(|window, cx| {
            let view = cx.new(|_| NotificationHarness);
            Root::new(view, window, cx).bordered(false)
        });
        visual.update(|window, cx| {
            window.push_notification(
                MoonNotification::success("Rule added")
                    .action("Undo", move |_, _| clicked.set(clicked.get() + 1)),
                cx,
            );
        });
        visual.run_until_parked();
        visual
            .background_executor
            .advance_clock(Duration::from_millis(300));
        visual.update(|window, _| window.refresh());
        visual.run_until_parked();
        let position = visual
            .debug_bounds("moon-notification-action")
            .expect("action button")
            .center();

        // Dispatch both clicks in the same update, leaving no frame between them.
        visual.update(|window, cx| {
            for click_count in [1, 2] {
                window.dispatch_event(
                    PlatformInput::MouseDown(MouseDownEvent {
                        position,
                        modifiers: Modifiers::none(),
                        button: MouseButton::Left,
                        click_count,
                        first_mouse: false,
                    }),
                    cx,
                );
                window.dispatch_event(
                    PlatformInput::MouseUp(MouseUpEvent {
                        position,
                        modifiers: Modifiers::none(),
                        button: MouseButton::Left,
                        click_count,
                    }),
                    cx,
                );
            }
        });
        assert_eq!(runs.get(), 1);
    }
}
