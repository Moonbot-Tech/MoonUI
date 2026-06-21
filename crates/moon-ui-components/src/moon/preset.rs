use gpui::prelude::FluentBuilder;
use gpui::*;

use super::{
    foundation::box_shadow,
    text::MoonText,
    theme::MoonTheme,
    tokens::{MoonRect, rgba_from},
};

#[derive(Clone, Debug)]
pub struct MoonPresetItem {
    number: SharedString,
    hotkey: SharedString,
    label: SharedString,
    selected: bool,
    disabled: bool,
}

impl MoonPresetItem {
    pub fn new(
        number: impl Into<SharedString>,
        hotkey: impl Into<SharedString>,
        label: impl Into<SharedString>,
    ) -> Self {
        Self {
            number: number.into(),
            hotkey: hotkey.into(),
            label: label.into(),
            selected: false,
            disabled: false,
        }
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
pub struct MoonPresetStrip {
    id: ElementId,
    bounds: Option<MoonRect>,
    items: Vec<MoonPresetItem>,
    slot_width: f32,
}

impl MoonPresetStrip {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            bounds: None,
            items: Vec::new(),
            slot_width: 80.0,
        }
    }

    pub fn bounds(mut self, bounds: MoonRect) -> Self {
        self.bounds = Some(bounds);
        self
    }

    pub fn item(mut self, item: MoonPresetItem) -> Self {
        self.items.push(item);
        self
    }

    pub fn items(mut self, items: impl IntoIterator<Item = MoonPresetItem>) -> Self {
        self.items.extend(items);
        self
    }

    pub fn slot_width(mut self, slot_width: f32) -> Self {
        self.slot_width = slot_width;
        self
    }

    pub fn render(self) -> impl IntoElement {
        self
    }
}

impl RenderOnce for MoonPresetStrip {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = MoonTheme::active_tokens(cx);
        let p = tokens.palette;
        let mut root = div()
            .id(self.id)
            .relative()
            .h(px(tokens.ui(36.0)))
            .overflow_hidden();

        if let Some(bounds) = self.bounds {
            root = root
                .absolute()
                .left(px(bounds.x))
                .top(px(bounds.y))
                .w(px(bounds.w))
                .h(px(bounds.h));
        }

        for (ix, item) in self.items.into_iter().enumerate() {
            let x = ix as f32 * self.slot_width;
            let active = item.selected;
            let disabled = item.disabled;
            let fg = if active { p.amber } else { p.text_muted };
            let fg_alpha = if disabled { 0.40 } else { 1.0 };
            let hotkey_alpha = if active || !disabled { 1.0 } else { 0.40 };

            let selected_bg = linear_gradient(
                180.0,
                linear_color_stop(rgba_from(p.amber, 0.12), 0.0),
                linear_color_stop(rgba_from(p.amber, 0.024), 1.0),
            );
            let underline_left = linear_gradient(
                90.0,
                linear_color_stop(rgba_from(p.amber, 0.0), 0.0),
                linear_color_stop(rgba_from(p.amber, 0.70), 1.0),
            );
            let underline_right = linear_gradient(
                90.0,
                linear_color_stop(rgba_from(p.amber, 0.70), 0.0),
                linear_color_stop(rgba_from(p.amber, 0.0), 1.0),
            );

            let mut slot = div()
                .id(("preset-slot", ix))
                .absolute()
                .left(px(tokens.ui(x)))
                .top(px(0.0))
                .w(px(tokens.ui(self.slot_width)))
                .h(px(tokens.ui(36.0)))
                .rounded(px(tokens.ui(3.0)))
                .cursor_default()
                .when(active, |this| this.bg(selected_bg))
                .when(!active && !disabled, |this| {
                    this.hover(|this| this.bg(rgba_from(p.overlay, 0.018)))
                        .active(|this| this.bg(rgba_from(p.overlay, 0.012)))
                })
                .child(
                    div()
                        .absolute()
                        .left(px(tokens.ui(8.0)))
                        .top(px(tokens.ui(4.0)))
                        .w(px(tokens.ui(64.0)))
                        .h(px(tokens.ui(14.0)))
                        .flex()
                        .items_baseline()
                        .justify_between()
                        .child(
                            MoonText::new(item.number)
                                .color(fg)
                                .alpha(fg_alpha)
                                .font_size(10.5)
                                .line_height(14.0)
                                .weight(600.0)
                                .mono(true)
                                .uppercase(false)
                                .render(),
                        )
                        .child(
                            MoonText::new(item.hotkey)
                                .color(p.text_muted)
                                .alpha(hotkey_alpha)
                                .font_size(8.0)
                                .line_height(10.0)
                                .weight(400.0)
                                .mono(true)
                                .uppercase(false)
                                .render(),
                        ),
                )
                .child(
                    div()
                        .absolute()
                        .left(px(tokens.ui(8.0)))
                        .top(px(tokens.ui(19.0)))
                        .w(px(tokens.ui(64.0)))
                        .h(px(tokens.ui(13.0)))
                        .overflow_hidden()
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .child(
                            MoonText::new(item.label)
                                .color(fg)
                                .alpha(fg_alpha)
                                .font_size(9.5)
                                .line_height(13.0)
                                .weight(400.0)
                                .mono(true)
                                .uppercase(false)
                                .render(),
                        ),
                );

            if active {
                let shadow = box_shadow(
                    px(0.0),
                    px(0.0),
                    px(tokens.ui(6.0)),
                    px(0.0),
                    rgba_from(p.amber, 0.42),
                );
                slot = slot.child(
                    div()
                        .absolute()
                        .left(px(tokens.ui(4.0)))
                        .top(px(tokens.ui(34.0)))
                        .w(px(tokens.ui(36.0)))
                        .h(px(tokens.ui(1.0)))
                        .bg(underline_left)
                        .shadow(vec![shadow.clone()]),
                );
                slot = slot.child(
                    div()
                        .absolute()
                        .left(px(tokens.ui(40.0)))
                        .top(px(tokens.ui(34.0)))
                        .w(px(tokens.ui(36.0)))
                        .h(px(tokens.ui(1.0)))
                        .bg(underline_right)
                        .shadow(vec![shadow]),
                );
            }

            root = root.child(slot);
        }

        root
    }
}
