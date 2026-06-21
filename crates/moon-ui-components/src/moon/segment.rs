use gpui::prelude::FluentBuilder;
use gpui::*;

use super::{
    foundation::MoonIndexedClickHandler,
    text::MoonText,
    theme::{MoonTheme, MoonThemeTokens},
    tokens::{MoonPalette, MoonRect, rgba_from},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoonAccent {
    Amber,
    Blue,
    Green,
    Red,
}

impl MoonAccent {
    fn color(self, palette: MoonPalette) -> u32 {
        match self {
            MoonAccent::Amber => palette.amber,
            MoonAccent::Blue => palette.blue,
            MoonAccent::Green => palette.green,
            MoonAccent::Red => palette.red,
        }
    }
}

#[derive(Clone, Debug)]
pub struct MoonSegmentItem {
    hotkey: SharedString,
    label: SharedString,
    width: f32,
    selected: bool,
    disabled: bool,
}

impl MoonSegmentItem {
    pub fn new(hotkey: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            hotkey: hotkey.into(),
            label: label.into(),
            width: 64.0,
            selected: false,
            disabled: false,
        }
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

#[derive(IntoElement)]
pub struct MoonSegmentedControl {
    id: ElementId,
    bounds: Option<MoonRect>,
    items: Vec<MoonSegmentItem>,
    accent: MoonAccent,
    item_gap: f32,
    on_click: Option<MoonIndexedClickHandler>,
}

impl MoonSegmentedControl {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            bounds: None,
            items: Vec::new(),
            accent: MoonAccent::Amber,
            item_gap: 0.0,
            on_click: None,
        }
    }

    pub fn bounds(mut self, bounds: MoonRect) -> Self {
        self.bounds = Some(bounds);
        self
    }

    pub fn accent(mut self, accent: MoonAccent) -> Self {
        self.accent = accent;
        self
    }

    pub fn item(mut self, item: MoonSegmentItem) -> Self {
        self.items.push(item);
        self
    }

    pub fn items(mut self, items: impl IntoIterator<Item = MoonSegmentItem>) -> Self {
        self.items.extend(items);
        self
    }

    pub fn item_gap(mut self, item_gap: f32) -> Self {
        self.item_gap = item_gap;
        self
    }

    pub fn on_click(
        mut self,
        handler: impl Fn(usize, &ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(std::rc::Rc::new(handler));
        self
    }

    pub fn render(self) -> impl IntoElement {
        self
    }

    pub fn render_with_palette(self, p: MoonPalette) -> impl IntoElement {
        self.render_with_theme(p, MoonThemeTokens::default())
    }

    pub fn render_with_theme(self, p: MoonPalette, tokens: MoonThemeTokens) -> impl IntoElement {
        let accent = self.accent.color(p);
        let on_click = self.on_click.clone();

        let mut root = div()
            .id(self.id)
            .relative()
            .flex()
            .items_center()
            .h(px(tokens.fit_height(26.0, 14.0, 6.0)))
            .gap(px(tokens.ui(self.item_gap)))
            .whitespace_nowrap();

        if let Some(bounds) = self.bounds {
            root = root
                .absolute()
                .left(px(bounds.x))
                .top(px(bounds.y))
                .w(px(bounds.w))
                .h(px(bounds.h));
        }

        for (ix, item) in self.items.into_iter().enumerate() {
            let selected = item.selected;
            let disabled = item.disabled;
            let key_color = if selected { accent } else { p.text_muted };
            let key_alpha = if selected { 0.60 } else { 0.667 };
            let label_color = if selected { accent } else { p.text_muted };
            let item_click = on_click.clone();

            let selected_bg = linear_gradient(
                180.0,
                linear_color_stop(rgba_from(accent, 0.10), 0.0),
                linear_color_stop(rgba_from(accent, 0.016), 1.0),
            );
            let underline_width = (item.width - 16.0).max(1.0);
            let underline_left = linear_gradient(
                90.0,
                linear_color_stop(rgba_from(accent, 0.0), 0.0),
                linear_color_stop(rgba_from(accent, 0.92), 1.0),
            );
            let underline_right = linear_gradient(
                90.0,
                linear_color_stop(rgba_from(accent, 0.92), 0.0),
                linear_color_stop(rgba_from(accent, 0.0), 1.0),
            );

            let mut cell = div()
                .id(("segment-item", ix))
                .relative()
                .flex()
                .items_center()
                .justify_center()
                .gap(px(tokens.ui(5.0)))
                .w(px(item.width))
                .h_full()
                .px(px(tokens.ui(11.0)))
                .rounded(px(tokens.ui(if selected { 4.0 } else { 0.0 })))
                .cursor_default()
                .when(selected, |this| this.bg(selected_bg))
                .when(!selected && !disabled, |this| {
                    this.hover(move |this| this.bg(rgba_from(p.overlay, 0.025)))
                        .active(move |this| this.bg(rgba_from(p.overlay, 0.016)))
                })
                .child(
                    MoonText::new(item.hotkey)
                        .color(key_color)
                        .alpha(if disabled { 0.40 } else { key_alpha })
                        .font_size(8.5)
                        .line_height(12.0)
                        .weight(400.0)
                        .mono(true)
                        .uppercase(false)
                        .render(),
                )
                .child(
                    MoonText::new(item.label)
                        .color(label_color)
                        .alpha(if disabled { 0.40 } else { 1.0 })
                        .font_size(11.0)
                        .line_height(14.0)
                        .weight(if selected { 500.0 } else { 400.0 })
                        .mono(true)
                        .uppercase(false)
                        .render(),
                );

            if selected {
                let shadow = super::foundation::box_shadow(
                    px(0.0),
                    px(0.0),
                    px(tokens.ui(8.0)),
                    px(0.0),
                    rgba_from(accent, 0.72),
                );
                cell = cell
                    .child(
                        div()
                            .absolute()
                            .left(px(tokens.ui(8.0)))
                            .top(px(tokens.ui(24.0)))
                            .w(px(underline_width * 0.5))
                            .h(px(1.0))
                            .bg(underline_left)
                            .shadow(vec![shadow.clone()]),
                    )
                    .child(
                        div()
                            .absolute()
                            .left(px(tokens.ui(8.0) + underline_width * 0.5))
                            .top(px(tokens.ui(24.0)))
                            .w(px(underline_width * 0.5))
                            .h(px(1.0))
                            .bg(underline_right)
                            .shadow(vec![shadow]),
                    );
            }

            if let Some(on_click) = item_click {
                cell = cell.on_click(move |event, window, cx| {
                    if disabled {
                        cx.stop_propagation();
                        return;
                    }
                    on_click(ix, event, window, cx);
                });
            }

            root = root.child(cell);
        }

        root
    }
}

impl RenderOnce for MoonSegmentedControl {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = MoonTheme::active_tokens(cx);
        self.render_with_theme(MoonPalette::active(cx), tokens)
    }
}
