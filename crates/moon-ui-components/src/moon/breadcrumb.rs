use std::rc::Rc;

use gpui::*;

use super::{text::MoonText, theme::MoonTheme, tokens::rgba_from};

#[derive(Clone)]
pub struct MoonBreadcrumbItem {
    label: SharedString,
    disabled: bool,
    on_click: Option<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>>,
}

impl MoonBreadcrumbItem {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            disabled: false,
            on_click: None,
        }
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }
}

impl From<&'static str> for MoonBreadcrumbItem {
    fn from(value: &'static str) -> Self {
        Self::new(value)
    }
}

impl From<String> for MoonBreadcrumbItem {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

#[derive(IntoElement)]
pub struct MoonBreadcrumb {
    items: Vec<MoonBreadcrumbItem>,
}

impl MoonBreadcrumb {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn child(mut self, item: impl Into<MoonBreadcrumbItem>) -> Self {
        self.items.push(item.into());
        self
    }

    pub fn children(
        mut self,
        items: impl IntoIterator<Item = impl Into<MoonBreadcrumbItem>>,
    ) -> Self {
        self.items.extend(items.into_iter().map(Into::into));
        self
    }

    pub fn render(self) -> impl IntoElement {
        self
    }
}

impl Default for MoonBreadcrumb {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderOnce for MoonBreadcrumb {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = MoonTheme::active_tokens(cx);
        let p = tokens.palette;
        let count = self.items.len();
        let mut root = div()
            .flex()
            .items_center()
            .gap(px(tokens.ui(6.0)))
            .font_family(tokens.font_family(true));

        for (ix, item) in self.items.into_iter().enumerate() {
            let is_last = ix + 1 == count;
            let mut label = div()
                .id(("moon-breadcrumb", ix))
                .rounded(px(tokens.ui(3.0)))
                .px(px(tokens.ui(3.0)))
                .py(px(tokens.ui(1.0)))
                .cursor_default()
                .child(
                    MoonText::new(item.label)
                        .color(if is_last { p.text } else { p.text_soft })
                        .alpha(if item.disabled { 0.42 } else { 1.0 })
                        .font_size(10.5)
                        .line_height(13.0)
                        .weight(if is_last { 600.0 } else { 400.0 })
                        .mono(true)
                        .uppercase(false)
                        .render(),
                );
            if !is_last && !item.disabled {
                if let Some(on_click) = item.on_click {
                    label = label
                        .cursor_pointer()
                        .hover(|this| {
                            this.bg(rgba_from(p.overlay, 0.04))
                                .text_color(rgba_from(p.text, 1.0))
                        })
                        .on_click(move |event, window, cx| on_click(event, window, cx));
                }
            }
            root = root.child(label);
            if !is_last {
                root = root.child(
                    MoonText::new("/")
                        .color(p.text_muted)
                        .font_size(10.0)
                        .line_height(13.0)
                        .mono(true)
                        .uppercase(false)
                        .render(),
                );
            }
        }

        root
    }
}
