use gpui::SharedString;

use crate::notification::Notification;

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

    pub fn into_inner(self) -> Notification {
        self.inner
    }
}

impl From<MoonNotification> for Notification {
    fn from(note: MoonNotification) -> Self {
        note.inner
    }
}
