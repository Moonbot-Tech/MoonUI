use gpui::*;

use crate::{Disableable, Sizable, Size, pagination::Pagination};

#[derive(IntoElement)]
pub struct MoonPagination {
    inner: Pagination,
}

impl MoonPagination {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            inner: Pagination::new(id),
        }
    }

    pub fn current_page(mut self, page: usize) -> Self {
        self.inner = self.inner.current_page(page);
        self
    }

    pub fn total_pages(mut self, pages: usize) -> Self {
        self.inner = self.inner.total_pages(pages);
        self
    }

    pub fn visible_pages(mut self, max: usize) -> Self {
        self.inner = self.inner.visible_pages(max);
        self
    }

    pub fn compact(mut self) -> Self {
        self.inner = self.inner.compact();
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.inner = self.inner.disabled(disabled);
        self
    }

    pub fn small(mut self) -> Self {
        self.inner = self.inner.with_size(Size::Small);
        self
    }

    pub fn on_click(mut self, handler: impl Fn(&usize, &mut Window, &mut App) + 'static) -> Self {
        self.inner = self.inner.on_click(handler);
        self
    }

    pub fn render(self) -> impl IntoElement {
        self
    }
}

impl RenderOnce for MoonPagination {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        self.inner.render(window, cx)
    }
}
