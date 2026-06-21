use std::sync::Arc;

use gpui::*;

use crate::{
    Sizable, Size,
    accordion::{Accordion, AccordionItem},
};

#[derive(IntoElement)]
pub struct MoonAccordion {
    inner: Accordion,
}

impl MoonAccordion {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            inner: Accordion::new(id),
        }
    }

    pub fn multiple(mut self, multiple: bool) -> Self {
        self.inner = self.inner.multiple(multiple);
        self
    }

    pub fn bordered(mut self, bordered: bool) -> Self {
        self.inner = self.inner.bordered(bordered);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.inner = self.inner.disabled(disabled);
        self
    }

    pub fn item<F>(mut self, child: F) -> Self
    where
        F: FnOnce(MoonAccordionItem) -> MoonAccordionItem,
    {
        let item = child(MoonAccordionItem::new());
        self.inner = self.inner.item(|_| item.into_inner());
        self
    }

    pub fn on_toggle_click(
        mut self,
        on_toggle_click: impl Fn(&[usize], &mut Window, &mut App) + Send + Sync + 'static,
    ) -> Self {
        let on_toggle_click = Arc::new(on_toggle_click);
        self.inner = self
            .inner
            .on_toggle_click(move |open_ixs, window, cx| on_toggle_click(open_ixs, window, cx));
        self
    }

    pub fn render(self) -> impl IntoElement {
        self
    }
}

impl Sizable for MoonAccordion {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.inner = self.inner.with_size(size);
        self
    }
}

impl RenderOnce for MoonAccordion {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        self.inner.render(window, cx)
    }
}

pub struct MoonAccordionItem {
    inner: AccordionItem,
}

impl MoonAccordionItem {
    fn new() -> Self {
        Self {
            inner: AccordionItem::new(),
        }
    }

    fn into_inner(self) -> AccordionItem {
        self.inner
    }

    pub fn title(mut self, title: impl IntoElement) -> Self {
        self.inner = self.inner.title(title);
        self
    }

    pub fn open(mut self, open: bool) -> Self {
        self.inner = self.inner.open(open);
        self
    }

    pub fn bordered(mut self, bordered: bool) -> Self {
        self.inner = self.inner.bordered(bordered);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.inner = self.inner.disabled(disabled);
        self
    }
}

impl ParentElement for MoonAccordionItem {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.inner.extend(elements);
    }
}

impl Sizable for MoonAccordionItem {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.inner = self.inner.with_size(size);
        self
    }
}
