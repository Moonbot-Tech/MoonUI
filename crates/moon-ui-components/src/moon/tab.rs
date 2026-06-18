use std::rc::Rc;

use gpui::prelude::FluentBuilder;
use gpui::*;

use super::badge::{MoonBadge, MoonBadgeSize, MoonBadgeVariant};
use super::text::MoonText;
use super::tokens::{MoonPalette, MoonRect, rgba_from};

#[derive(Clone, Debug)]
pub struct MoonTabItem {
    label: SharedString,
    badge: Option<SharedString>,
    width: f32,
    selected: bool,
    disabled: bool,
    closable: bool,
}

impl MoonTabItem {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            badge: None,
            width: 70.0,
            selected: false,
            disabled: false,
            closable: false,
        }
    }

    pub fn badge(mut self, badge: impl Into<SharedString>) -> Self {
        self.badge = Some(badge.into());
        self
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

    pub fn closable(mut self, closable: bool) -> Self {
        self.closable = closable;
        self
    }
}

type MoonTabHandler = Rc<dyn Fn(usize, &ClickEvent, &mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct MoonTabStrip {
    id: ElementId,
    bounds: Option<MoonRect>,
    items: Vec<MoonTabItem>,
    padding_left: f32,
    gap: f32,
    on_click: Option<MoonTabHandler>,
    on_close: Option<MoonTabHandler>,
}

impl MoonTabStrip {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            bounds: None,
            items: Vec::new(),
            padding_left: 20.0,
            gap: 8.0,
            on_click: None,
            on_close: None,
        }
    }

    pub fn bounds(mut self, bounds: MoonRect) -> Self {
        self.bounds = Some(bounds);
        self
    }

    pub fn padding_left(mut self, padding_left: f32) -> Self {
        self.padding_left = padding_left;
        self
    }

    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    pub fn item(mut self, item: MoonTabItem) -> Self {
        self.items.push(item);
        self
    }

    pub fn items(mut self, items: impl IntoIterator<Item = MoonTabItem>) -> Self {
        self.items.extend(items);
        self
    }

    pub fn on_click(
        mut self,
        handler: impl Fn(usize, &ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    pub fn on_close(
        mut self,
        handler: impl Fn(usize, &ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_close = Some(Rc::new(handler));
        self
    }

    pub fn render(self) -> impl IntoElement {
        self
    }

    pub fn render_with_palette(self, p: MoonPalette) -> impl IntoElement {
        let mut root = div()
            .id(self.id)
            .relative()
            .overflow_hidden()
            .bg(rgba_from(p.shell_high, 1.0))
            .child(
                div()
                    .absolute()
                    .left(px(0.0))
                    .bottom(px(0.0))
                    .w_full()
                    .h(px(1.0))
                    .bg(rgba_from(p.border, 0.78)),
            );

        if let Some(bounds) = self.bounds {
            root = root
                .absolute()
                .left(px(bounds.x))
                .top(px(bounds.y))
                .w(px(bounds.w))
                .h(px(bounds.h));
        }

        let mut x = self.padding_left;
        for (ix, item) in self.items.into_iter().enumerate() {
            let active = item.selected;
            let disabled = item.disabled;
            let closable = item.closable;
            let fg = if active { p.text } else { p.text_muted };
            let fg_alpha = if disabled { 0.45 } else { 1.0 };

            let mut tab = div()
                .id(("moon-tab", ix))
                .absolute()
                .left(px(x))
                .top(px(0.0))
                .w(px(item.width))
                .h(px(28.0))
                .flex()
                .items_center()
                .pl(px(8.0))
                .pr(px(if closable { 5.0 } else { 8.0 }))
                .gap(px(8.0))
                .when(disabled, |this| this.cursor(CursorStyle::Arrow))
                .when(!disabled, |this| this.cursor_pointer())
                .when(!active && !disabled, |this| {
                    this.hover(|this| this.bg(rgba_from(0xFFFFFF, 0.018)))
                        .active(|this| this.bg(rgba_from(0xFFFFFF, 0.012)))
                })
                .child(
                    MoonText::new(item.label)
                        .color(fg)
                        .alpha(fg_alpha)
                        .font_size(10.0)
                        .line_height(13.0)
                        .weight(if active { 600.0 } else { 400.0 })
                        .mono(true)
                        .uppercase(false)
                        .render()
                        .mt(px(2.0)),
                );

            if let Some(on_click) = self.on_click.clone()
                && !disabled
            {
                tab = tab.on_click(move |event, window, cx| {
                    on_click(ix, event, window, cx);
                });
            }

            if let Some(badge) = item.badge {
                tab = tab.child(
                    MoonBadge::new(badge)
                        .size(MoonBadgeSize::Tiny)
                        .variant(if active {
                            MoonBadgeVariant::Solid
                        } else {
                            MoonBadgeVariant::Soft
                        })
                        .bg_color(if active { p.amber } else { 0xFFFFFF })
                        .bg_alpha(if active { 0.80 } else { 0.06 })
                        .text_color(if active { p.shell } else { p.text_soft })
                        .weight(600.0)
                        .margin_top(2.0)
                        .disabled(disabled)
                        .render_with_palette(p),
                );
            }

            if closable {
                let on_close = self.on_close.clone();
                tab = tab.child(
                    div()
                        .id(("moon-tab-close", ix))
                        .w(px(16.0))
                        .h(px(16.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(3.0))
                        .text_size(px(10.0))
                        .line_height(px(10.0))
                        .text_color(rgba_from(p.text_muted, 0.90))
                        .hover(|this| this.bg(rgba_from(0xFFFFFF, 0.045)))
                        .child("x")
                        .on_click(move |event, window, cx| {
                            if let Some(on_close) = &on_close {
                                on_close(ix, event, window, cx);
                            }
                            cx.stop_propagation();
                        }),
                );
            }

            if active {
                tab = tab.child(moon_active_tab_underline(p));
            }

            root = root.child(tab);
            x += item.width + self.gap;
        }

        root
    }
}

impl RenderOnce for MoonTabStrip {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        self.render_with_palette(MoonPalette::active(cx))
    }
}

/// Янтарный underline активной вкладки (точный вид MoonTabStrip), адаптивный по ширине:
/// fade-in слева, сплошной центр (растягивается), fade-out справа, с мягкой тенью.
/// Абсолютно позиционируется по низу родителя (родитель должен быть `relative`).
/// Единый источник вида для верхних (MoonTabStrip) и нижних (dock TabPanel) вкладок.
pub fn moon_active_tab_underline(p: MoonPalette) -> Div {
    let underline_left = linear_gradient(
        90.0,
        linear_color_stop(rgba_from(p.amber, 0.0), 0.0),
        linear_color_stop(rgba_from(p.amber, 1.0), 1.0),
    );
    let underline_right = linear_gradient(
        90.0,
        linear_color_stop(rgba_from(p.amber, 1.0), 0.0),
        linear_color_stop(rgba_from(p.amber, 0.0), 1.0),
    );
    let shadow = super::foundation::box_shadow(
        px(0.0),
        px(0.0),
        px(8.0),
        px(0.0),
        rgba_from(p.amber, 0.70),
    );
    div()
        .absolute()
        .left(px(5.0))
        .right(px(5.0))
        .bottom(px(0.0))
        .h(px(1.0))
        .flex()
        .child(
            div()
                .w(px(25.0))
                .h_full()
                .bg(underline_left)
                .shadow(vec![shadow.clone()]),
        )
        .child(
            div()
                .flex_1()
                .h_full()
                .bg(rgba_from(p.amber, 1.0))
                .shadow(vec![shadow.clone()]),
        )
        .child(
            div()
                .w(px(25.0))
                .h_full()
                .bg(underline_right)
                .shadow(vec![shadow]),
        )
}
