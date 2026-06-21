use gpui::*;

use crate::alert::Alert;

#[derive(IntoElement)]
pub struct MoonAlert {
    inner: Alert,
}

impl MoonAlert {
    pub fn new(id: impl Into<ElementId>, message: impl Into<SharedString>) -> Self {
        Self {
            inner: Alert::new(id, message.into()),
        }
    }

    pub fn info(id: impl Into<ElementId>, message: impl Into<SharedString>) -> Self {
        Self {
            inner: Alert::info(id, message.into()),
        }
    }

    pub fn success(id: impl Into<ElementId>, message: impl Into<SharedString>) -> Self {
        Self {
            inner: Alert::success(id, message.into()),
        }
    }

    pub fn warning(id: impl Into<ElementId>, message: impl Into<SharedString>) -> Self {
        Self {
            inner: Alert::warning(id, message.into()),
        }
    }

    pub fn error(id: impl Into<ElementId>, message: impl Into<SharedString>) -> Self {
        Self {
            inner: Alert::error(id, message.into()),
        }
    }

    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.inner = self.inner.title(title);
        self
    }

    pub fn banner(mut self) -> Self {
        self.inner = self.inner.banner();
        self
    }

    pub fn visible(mut self, visible: bool) -> Self {
        self.inner = self.inner.visible(visible);
        self
    }

    pub fn render(self) -> impl IntoElement {
        self
    }
}

impl RenderOnce for MoonAlert {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        self.inner.render(window, cx)
    }
}
