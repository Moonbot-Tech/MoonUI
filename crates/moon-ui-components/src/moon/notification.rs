use std::{rc::Rc, time::Duration};

use gpui::{App, InteractiveElement as _, SharedString, Window};

use crate::{
    button::{Button, ButtonVariants as _},
    notification::Notification,
};

/// A Moon-styled notification with optional timed dismissal and an action button.
pub struct MoonNotification {
    inner: Notification,
}

impl MoonNotification {
    pub fn new() -> Self {
        Self {
            inner: Notification::new(),
        }
    }

    pub fn info(message: impl Into<SharedString>) -> Self {
        Self {
            inner: Notification::info(message),
        }
    }

    pub fn success(message: impl Into<SharedString>) -> Self {
        Self {
            inner: Notification::success(message),
        }
    }

    pub fn warning(message: impl Into<SharedString>) -> Self {
        Self {
            inner: Notification::warning(message),
        }
    }

    pub fn error(message: impl Into<SharedString>) -> Self {
        Self {
            inner: Notification::error(message),
        }
    }

    pub fn message(mut self, message: impl Into<SharedString>) -> Self {
        self.inner = self.inner.message(message);
        self
    }

    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.inner = self.inner.title(title);
        self
    }

    pub fn autohide(mut self, autohide: bool) -> Self {
        self.inner = self.inner.autohide(autohide);
        self
    }

    /// Set the time before automatic dismissal starts.
    pub fn autohide_after(mut self, duration: Duration) -> Self {
        self.inner = self.inner.autohide_after(duration);
        self
    }

    /// Add a small ghost button that invokes the handler and then dismisses the notification.
    ///
    /// The action enables autohide. Call [`Self::autohide`] afterward to override it.
    pub fn action(
        mut self,
        label: impl Into<SharedString>,
        on_click: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        let label = label.into();
        let on_click = Rc::new(on_click);
        self.inner = self
            .inner
            .action(move |_, _, cx| {
                let notification = cx.weak_entity();
                let on_click = on_click.clone();
                Button::new("moon-notification-action")
                    .debug_selector(|| "moon-notification-action".into())
                    .label(label.clone())
                    .ghost()
                    .on_click(move |_, window, cx| {
                        cx.stop_propagation();
                        on_click(window, cx);
                        _ = notification.update(cx, |note, cx| note.dismiss(window, cx));
                    })
            })
            .autohide(true);
        self
    }

    pub fn into_inner(self) -> Notification {
        self.inner
    }
}

impl From<MoonNotification> for Notification {
    fn from(note: MoonNotification) -> Self {
        note.inner
    }
}

#[cfg(test)]
mod tests;
