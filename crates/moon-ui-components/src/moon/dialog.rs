use gpui::*;

use crate::dialog::{Dialog, DialogContent};

#[derive(IntoElement)]
pub struct MoonDialog {
    inner: Dialog,
}

impl MoonDialog {
    pub(crate) fn from_inner(inner: Dialog) -> Self {
        Self { inner }
    }

    pub(crate) fn into_inner(self) -> Dialog {
        self.inner
    }

    pub fn w(mut self, width: impl Into<Pixels>) -> Self {
        self.inner = self.inner.w(width);
        self
    }

    pub fn width(mut self, width: impl Into<Pixels>) -> Self {
        self.inner = self.inner.width(width);
        self
    }

    pub fn close_button(mut self, close_button: bool) -> Self {
        self.inner = self.inner.close_button(close_button);
        self
    }

    pub fn overlay(mut self, overlay: bool) -> Self {
        self.inner = self.inner.overlay(overlay);
        self
    }

    pub fn overlay_closable(mut self, overlay_closable: bool) -> Self {
        self.inner = self.inner.overlay_closable(overlay_closable);
        self
    }

    pub fn title(mut self, title: impl IntoElement) -> Self {
        self.inner = self.inner.title(title);
        self
    }

    pub fn header(mut self, header: impl IntoElement) -> Self {
        self.inner = self.inner.header(header);
        self
    }

    pub fn footer(mut self, footer: impl IntoElement) -> Self {
        self.inner = self.inner.footer(footer);
        self
    }

    pub fn content<F>(mut self, builder: F) -> Self
    where
        F: Fn(MoonDialogContent, &mut Window, &mut App) -> MoonDialogContent + 'static,
    {
        self.inner = self.inner.content(move |content, window, cx| {
            builder(MoonDialogContent::from_inner(content), window, cx).into_inner()
        });
        self
    }

    pub fn on_cancel(
        mut self,
        on_cancel: impl Fn(&ClickEvent, &mut Window, &mut App) -> bool + 'static,
    ) -> Self {
        self.inner = self.inner.on_cancel(on_cancel);
        self
    }

    pub fn on_close(
        mut self,
        on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.inner = self.inner.on_close(on_close);
        self
    }

    pub fn render(self) -> impl IntoElement {
        self
    }
}

impl Styled for MoonDialog {
    fn style(&mut self) -> &mut StyleRefinement {
        self.inner.style()
    }
}

impl RenderOnce for MoonDialog {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        self.inner.render(window, cx)
    }
}

pub struct MoonDialogContent {
    inner: DialogContent,
}

impl MoonDialogContent {
    fn from_inner(inner: DialogContent) -> Self {
        Self { inner }
    }

    fn into_inner(self) -> DialogContent {
        self.inner
    }
}

impl ParentElement for MoonDialogContent {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.inner.extend(elements);
    }
}
