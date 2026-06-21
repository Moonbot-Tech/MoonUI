use gpui::*;

use crate::{
    Sizable, Size,
    description_list::{DescriptionItem, DescriptionList, DescriptionText},
};

#[derive(IntoElement)]
pub struct MoonDescriptionList {
    inner: DescriptionList,
}

impl MoonDescriptionList {
    pub fn new() -> Self {
        Self {
            inner: DescriptionList::new(),
        }
    }

    pub fn vertical() -> Self {
        Self {
            inner: DescriptionList::vertical(),
        }
    }

    pub fn horizontal() -> Self {
        Self {
            inner: DescriptionList::horizontal(),
        }
    }

    pub fn item(
        mut self,
        label: impl Into<DescriptionText>,
        value: impl Into<DescriptionText>,
        span: usize,
    ) -> Self {
        self.inner = self.inner.item(label, value, span);
        self
    }

    pub fn child(mut self, child: impl Into<DescriptionItem>) -> Self {
        self.inner = self.inner.child(child);
        self
    }

    pub fn separator(mut self) -> Self {
        self.inner = self.inner.separator();
        self
    }

    pub fn columns(mut self, columns: usize) -> Self {
        self.inner = self.inner.columns(columns);
        self
    }

    pub fn bordered(mut self, bordered: bool) -> Self {
        self.inner = self.inner.bordered(bordered);
        self
    }

    pub fn small(mut self) -> Self {
        self.inner = self.inner.with_size(Size::Small);
        self
    }

    pub fn render(self) -> impl IntoElement {
        self
    }
}

impl Default for MoonDescriptionList {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderOnce for MoonDescriptionList {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        self.inner.render(window, cx)
    }
}
