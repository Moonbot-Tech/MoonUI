use gpui::prelude::FluentBuilder;
use gpui::*;

use super::{
    icons::{MOON_ICON_CARET_DOWN, moon_icon},
    text::MoonText,
    theme::MoonTheme,
    tokens::{MoonRect, rgba_from},
};

#[derive(Default)]
struct MoonCollapsibleState {
    open: bool,
}

#[derive(IntoElement)]
pub struct MoonCollapsible {
    id: SharedString,
    bounds: Option<MoonRect>,
    title: Option<SharedString>,
    header: Vec<AnyElement>,
    content: Vec<AnyElement>,
    default_open: bool,
    controlled_open: Option<bool>,
    disabled: bool,
    header_height: f32,
    gap: f32,
}

impl MoonCollapsible {
    pub fn new(id: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            bounds: None,
            title: None,
            header: Vec::new(),
            content: Vec::new(),
            default_open: false,
            controlled_open: None,
            disabled: false,
            header_height: 26.0,
            gap: 6.0,
        }
    }

    pub fn bounds(mut self, bounds: MoonRect) -> Self {
        self.bounds = Some(bounds);
        self
    }

    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn header(mut self, header: impl IntoElement) -> Self {
        self.header.push(header.into_any_element());
        self
    }

    pub fn content(mut self, content: impl IntoElement) -> Self {
        self.content.push(content.into_any_element());
        self
    }

    pub fn default_open(mut self, open: bool) -> Self {
        self.default_open = open;
        self
    }

    pub fn open(mut self, open: bool) -> Self {
        self.controlled_open = Some(open);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn header_height(mut self, height: f32) -> Self {
        self.header_height = height;
        self
    }

    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }
}

impl ParentElement for MoonCollapsible {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.header.extend(elements);
    }
}

impl RenderOnce for MoonCollapsible {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = MoonTheme::active_tokens(cx);
        let p = tokens.palette;
        let state = window.use_keyed_state(
            ElementId::from(SharedString::from(format!("{}:state", self.id))),
            cx,
            |_, _| MoonCollapsibleState {
                open: self.default_open,
            },
        );

        let controlled_open = self.controlled_open;
        let open = controlled_open.unwrap_or_else(|| state.read(cx).open);
        let parent_view = window.current_view();
        let disabled = self.disabled;

        let mut root = div()
            .id(ElementId::from(self.id.clone()))
            .relative()
            .flex()
            .flex_col()
            .gap(px(tokens.ui(self.gap)));

        if let Some(bounds) = self.bounds {
            root = root
                .absolute()
                .left(px(bounds.x))
                .top(px(bounds.y))
                .w(px(bounds.w))
                .h(px(bounds.h));
        }

        let mut header = div()
            .id(ElementId::from(SharedString::from(format!(
                "{}:header",
                self.id
            ))))
            .h(px(tokens.ui(self.header_height)))
            .rounded(px(tokens.ui(4.0)))
            .border(px(tokens.ui(1.0)))
            .border_color(rgba_from(p.border, if open { 1.0 } else { 0.74 }))
            .bg(rgba_from(p.shell_high, if open { 0.98 } else { 0.72 }))
            .flex()
            .items_center()
            .gap(px(tokens.ui(8.0)))
            .px(px(tokens.ui(8.0)))
            .cursor_default()
            .when(!disabled, |this| {
                this.hover(|this| this.bg(rgba_from(p.overlay, 0.055)))
                    .active(|this| this.bg(rgba_from(p.overlay, 0.035)))
            })
            .on_mouse_down(MouseButton::Left, {
                let state = state.clone();
                move |_, window, cx| {
                    cx.stop_propagation();
                    if disabled {
                        return;
                    }
                    window.prevent_default();
                    if controlled_open.is_none() {
                        state.update(cx, |state, _| state.open = !state.open);
                        cx.notify(parent_view);
                    }
                }
            })
            .child(
                div()
                    .w(px(tokens.ui(12.0)))
                    .h(px(tokens.ui(12.0)))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(moon_icon(
                        MOON_ICON_CARET_DOWN,
                        tokens.ui(11.0),
                        p.text_muted,
                        if disabled { 0.45 } else { 1.0 },
                    )),
            );

        if self.header.is_empty() {
            header = header.child(
                MoonText::new(self.title.unwrap_or_else(|| SharedString::from("Section")))
                    .color(if disabled { p.text_muted } else { p.text_soft })
                    .alpha(if disabled { 0.45 } else { 1.0 })
                    .font_size(10.5)
                    .line_height(13.0)
                    .weight(600.0)
                    .mono(true)
                    .uppercase(false)
                    .render(),
            );
        } else {
            header = header.children(self.header);
        }

        root = root.child(header);

        if open {
            root = root.child(
                div()
                    .relative()
                    .flex()
                    .flex_col()
                    .gap(px(tokens.ui(self.gap)))
                    .children(self.content),
            );
        }

        root
    }
}
