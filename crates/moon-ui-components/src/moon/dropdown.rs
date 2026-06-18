use gpui::prelude::FluentBuilder;
use gpui::*;
use crate::popover::Popover as CorePopover;

use super::{
    button::{MoonButton, MoonButtonSegment, MoonButtonSize, MoonButtonVariant},
    foundation::{MoonClickHandler, MoonSelectHandler},
    icons::{MOON_ICON_CHECK, moon_icon},
    text::MoonText,
    tokens::{MoonPalette, MoonRect, MoonTone, rgba_from},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoonMenuItemKind {
    Item,
    Label,
    Separator,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MoonMenuSize {
    Compact,
    Normal,
    Custom {
        row_height: f32,
        font_size: f32,
        line_height: f32,
        radius: f32,
        pad_x: f32,
        gap: f32,
    },
}

#[derive(Clone, Copy, Debug)]
struct MenuMetrics {
    row_height: f32,
    font_size: f32,
    line_height: f32,
    radius: f32,
    pad_x: f32,
    gap: f32,
}

#[derive(Clone)]
pub struct MoonMenuItem {
    key: SharedString,
    label: SharedString,
    kind: MoonMenuItemKind,
    right_label: Option<SharedString>,
    tone: MoonTone,
    selected: bool,
    checked: bool,
    disabled: bool,
    submenu: Vec<MoonMenuItem>,
    on_click: Option<MoonClickHandler>,
}

impl MoonMenuItem {
    pub fn new(label: impl Into<SharedString>) -> Self {
        let label = label.into();
        Self {
            key: label.clone(),
            label,
            kind: MoonMenuItemKind::Item,
            right_label: None,
            tone: MoonTone::Default,
            selected: false,
            checked: false,
            disabled: false,
            submenu: Vec::new(),
            on_click: None,
        }
    }

    pub fn with_key(key: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            kind: MoonMenuItemKind::Item,
            right_label: None,
            tone: MoonTone::Default,
            selected: false,
            checked: false,
            disabled: false,
            submenu: Vec::new(),
            on_click: None,
        }
    }

    pub fn label(label: impl Into<SharedString>) -> Self {
        let mut item = Self::new(label);
        item.kind = MoonMenuItemKind::Label;
        item.disabled = true;
        item
    }

    pub fn separator() -> Self {
        Self {
            key: SharedString::from("separator"),
            label: SharedString::from(""),
            kind: MoonMenuItemKind::Separator,
            right_label: None,
            tone: MoonTone::Muted,
            selected: false,
            checked: false,
            disabled: true,
            submenu: Vec::new(),
            on_click: None,
        }
    }

    pub fn key(&self) -> &SharedString {
        &self.key
    }

    pub fn right_label(mut self, right_label: impl Into<SharedString>) -> Self {
        self.right_label = Some(right_label.into());
        self
    }

    pub fn tone(mut self, tone: MoonTone) -> Self {
        self.tone = tone;
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn submenu(mut self, items: impl IntoIterator<Item = MoonMenuItem>) -> Self {
        self.submenu = items.into_iter().collect();
        self
    }

    pub fn has_submenu(&self) -> bool {
        !self.submenu.is_empty()
    }

    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(std::rc::Rc::new(handler));
        self
    }
}

#[derive(IntoElement)]
pub struct MoonPopupMenu {
    id: SharedString,
    headers: Vec<AnyElement>,
    items: Vec<MoonMenuItem>,
    size: MoonMenuSize,
    width: f32,
    max_height: Option<f32>,
    mono: bool,
}

impl MoonPopupMenu {
    pub fn new(id: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            headers: Vec::new(),
            items: Vec::new(),
            size: MoonMenuSize::Normal,
            width: 160.0,
            max_height: None,
            mono: true,
        }
    }

    pub fn item(mut self, item: MoonMenuItem) -> Self {
        self.items.push(item);
        self
    }

    pub fn header(mut self, header: impl IntoElement) -> Self {
        self.headers.push(header.into_any_element());
        self
    }

    pub fn items(mut self, items: impl IntoIterator<Item = MoonMenuItem>) -> Self {
        self.items.extend(items);
        self
    }

    pub fn size(mut self, size: MoonMenuSize) -> Self {
        self.size = size;
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    pub fn max_height(mut self, max_height: f32) -> Self {
        self.max_height = Some(max_height);
        self
    }

    pub fn mono(mut self, mono: bool) -> Self {
        self.mono = mono;
        self
    }

    pub fn render(self) -> impl IntoElement {
        self
    }

    pub fn render_with_palette(self, p: MoonPalette) -> AnyElement {
        let metrics = self.metrics();
        let id = self.id.clone();
        let mono = self.mono;
        let shadow = super::foundation::box_shadow(
            px(0.0),
            px(8.0),
            px(18.0),
            px(0.0),
            rgba_from(0x000000, 0.46),
        );

        let mut menu = div()
            .id(ElementId::from(self.id.clone()))
            .relative()
            .w(px(self.width))
            .p(px(4.0))
            .rounded(px(5.0))
            .border(px(1.0))
            .border_color(rgba_from(p.border, 1.0))
            .bg(rgba_from(p.shell_high, 0.98))
            .shadow(vec![shadow])
            .occlude()
            .flex()
            .flex_col()
            .gap(px(2.0));

        if let Some(max_height) = self.max_height {
            menu = menu.max_h(px(max_height)).overflow_y_scroll();
        }

        for header in self.headers {
            menu = menu.child(header);
        }

        for (ix, item) in self.items.into_iter().enumerate() {
            menu = menu.child(Self::render_item(
                &id, mono, ix, item, metrics, self.width, p,
            ));
        }

        menu.into_any_element()
    }

    fn render_item(
        menu_id: &SharedString,
        mono: bool,
        ix: usize,
        item: MoonMenuItem,
        metrics: MenuMetrics,
        menu_width: f32,
        p: MoonPalette,
    ) -> AnyElement {
        let row_id = SharedString::from(format!("{}:item:{}", menu_id, ix));

        match item.kind {
            MoonMenuItemKind::Separator => div()
                .id(ElementId::from(row_id))
                .h(px(1.0))
                .mx(px(2.0))
                .my(px(3.0))
                .bg(rgba_from(p.border, 0.82))
                .into_any_element(),
            MoonMenuItemKind::Label => div()
                .id(ElementId::from(row_id))
                .h(px(metrics.row_height))
                .px(px(metrics.pad_x))
                .flex()
                .items_center()
                .child(
                    MoonText::new(item.label)
                        .color(p.text_muted)
                        .alpha(0.88)
                        .font_size(metrics.font_size)
                        .line_height(metrics.line_height)
                        .weight(500.0)
                        .mono(mono)
                        .uppercase(false)
                        .render(),
                )
                .into_any_element(),
            MoonMenuItemKind::Item => {
                let disabled = item.disabled;
                let selected = item.selected;
                let checked = item.checked;
                let submenu = item.submenu;
                let has_submenu = !submenu.is_empty();
                let fg = if disabled {
                    p.text_muted
                } else {
                    item.tone.color(p)
                };
                let alpha = if disabled { 0.45 } else { 1.0 };

                let mut row = div()
                    .id(ElementId::from(row_id))
                    .relative()
                    .h(px(metrics.row_height))
                    .rounded(px(metrics.radius))
                    .px(px(metrics.pad_x))
                    .flex()
                    .items_center()
                    .gap(px(metrics.gap))
                    .cursor_default()
                    .when(selected, |this| this.bg(rgba_from(p.blue, 0.12)))
                    .when(!disabled, |this| {
                        this.hover(|this| this.bg(rgba_from(0xFFFFFF, 0.055)))
                            .active(|this| this.bg(rgba_from(0xFFFFFF, 0.032)))
                    })
                    .child(
                        div()
                            .w(px(12.0))
                            .h(px(metrics.line_height))
                            .flex()
                            .items_center()
                            .justify_center()
                            .when(checked, |this| {
                                this.child(moon_icon(MOON_ICON_CHECK, 11.0, p.blue, alpha))
                            }),
                    )
                    .child(
                        MoonText::new(item.label)
                            .color(fg)
                            .alpha(alpha)
                            .font_size(metrics.font_size)
                            .line_height(metrics.line_height)
                            .weight(if selected { 600.0 } else { 400.0 })
                            .mono(mono)
                            .uppercase(false)
                            .render(),
                    )
                    .child(div().flex_1());

                if let Some(right_label) = item.right_label {
                    row = row.child(
                        MoonText::new(right_label)
                            .color(p.text_muted)
                            .alpha(alpha * 0.88)
                            .font_size(metrics.font_size - 0.5)
                            .line_height(metrics.line_height)
                            .weight(400.0)
                            .mono(mono)
                            .uppercase(false)
                            .render(),
                    );
                } else if has_submenu {
                    row = row.child(
                        MoonText::new("›")
                            .color(p.text_muted)
                            .alpha(alpha * 0.88)
                            .font_size(metrics.font_size)
                            .line_height(metrics.line_height)
                            .weight(600.0)
                            .mono(mono)
                            .uppercase(false)
                            .render(),
                    );
                }

                if let Some(on_click) = item.on_click {
                    row = row
                        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                            cx.stop_propagation();
                        })
                        .on_click(move |event, window, cx| {
                            if disabled {
                                cx.stop_propagation();
                                return;
                            }
                            on_click(event, window, cx);
                        });
                }

                if selected && has_submenu {
                    row = row.child(
                        div()
                            .absolute()
                            .left(px(menu_width - 2.0))
                            .top(px(-4.0))
                            .child(
                                MoonPopupMenu::new(format!("{menu_id}:submenu:{ix}"))
                                    .items(submenu)
                                    .size(MoonMenuSize::Custom {
                                        row_height: metrics.row_height,
                                        font_size: metrics.font_size,
                                        line_height: metrics.line_height,
                                        radius: metrics.radius,
                                        pad_x: metrics.pad_x,
                                        gap: metrics.gap,
                                    })
                                    .width(menu_width)
                                    .render_with_palette(p),
                            ),
                    );
                }

                row.into_any_element()
            }
        }
    }

    fn metrics(&self) -> MenuMetrics {
        match self.size {
            MoonMenuSize::Compact => MenuMetrics {
                row_height: 20.0,
                font_size: 9.5,
                line_height: 12.0,
                radius: 3.0,
                pad_x: 6.0,
                gap: 5.0,
            },
            MoonMenuSize::Normal => MenuMetrics {
                row_height: 24.0,
                font_size: 10.5,
                line_height: 13.0,
                radius: 4.0,
                pad_x: 7.0,
                gap: 6.0,
            },
            MoonMenuSize::Custom {
                row_height,
                font_size,
                line_height,
                radius,
                pad_x,
                gap,
            } => MenuMetrics {
                row_height,
                font_size,
                line_height,
                radius,
                pad_x,
                gap,
            },
        }
    }
}

impl RenderOnce for MoonPopupMenu {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        self.render_with_palette(MoonPalette::active(cx))
    }
}

fn wire_dropdown_items(
    items: Vec<MoonMenuItem>,
    close_on_select: bool,
    on_select: Option<MoonSelectHandler>,
    state: Entity<MoonDropdownState>,
    controlled_open: Option<bool>,
    on_open_change: Option<std::rc::Rc<dyn Fn(bool, &mut Window, &mut App)>>,
    parent_view: EntityId,
) -> Vec<MoonMenuItem> {
    items
        .into_iter()
        .map(|mut item| {
            if matches!(item.kind, MoonMenuItemKind::Item) {
                let key = item.key.clone();
                let existing_handler = item.on_click.clone();
                let on_select = on_select.clone();
                let state = state.clone();
                let on_open_change = on_open_change.clone();
                item.on_click = Some(std::rc::Rc::new(move |event, window, cx| {
                    if let Some(existing_handler) = existing_handler.as_ref() {
                        existing_handler(event, window, cx);
                    }
                    if let Some(on_select) = on_select.as_ref() {
                        on_select(&key, window, cx);
                    }
                    if close_on_select {
                        if let Some(on_open_change) = on_open_change.as_ref() {
                            on_open_change(false, window, cx);
                        }
                        if controlled_open.is_none() {
                            state.update(cx, |state, _| {
                                state.open = false;
                            });
                            cx.notify(parent_view);
                        }
                    }
                }));
            }
            item
        })
        .collect()
}

#[derive(Default)]
struct MoonDropdownState {
    open: bool,
}

#[derive(IntoElement)]
pub struct MoonDropdown {
    id: SharedString,
    bounds: Option<MoonRect>,
    label: SharedString,
    segments: Vec<MoonButtonSegment>,
    items: Vec<MoonMenuItem>,
    trigger_variant: MoonButtonVariant,
    trigger_size: MoonButtonSize,
    trigger_width: Option<f32>,
    selected: bool,
    disabled: bool,
    default_open: bool,
    controlled_open: Option<bool>,
    menu_width: f32,
    menu_offset_x: f32,
    menu_offset_y: f32,
    menu_size: MoonMenuSize,
    menu_max_height: Option<f32>,
    close_on_select: bool,
    on_select: Option<MoonSelectHandler>,
    on_open_change: Option<std::rc::Rc<dyn Fn(bool, &mut Window, &mut App)>>,
}

impl MoonDropdown {
    pub fn new(id: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            bounds: None,
            label: SharedString::from(""),
            segments: Vec::new(),
            items: Vec::new(),
            trigger_variant: MoonButtonVariant::Neutral,
            trigger_size: MoonButtonSize::Toolbar,
            trigger_width: None,
            selected: false,
            disabled: false,
            default_open: false,
            controlled_open: None,
            menu_width: 160.0,
            menu_offset_x: 0.0,
            menu_offset_y: 4.0,
            menu_size: MoonMenuSize::Normal,
            menu_max_height: None,
            close_on_select: true,
            on_select: None,
            on_open_change: None,
        }
    }

    pub fn bounds(mut self, bounds: MoonRect) -> Self {
        self.bounds = Some(bounds);
        self
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = label.into();
        self
    }

    pub fn segment(mut self, segment: MoonButtonSegment) -> Self {
        self.segments.push(segment);
        self
    }

    pub fn item(mut self, item: MoonMenuItem) -> Self {
        self.items.push(item);
        self
    }

    pub fn items(mut self, items: impl IntoIterator<Item = MoonMenuItem>) -> Self {
        self.items.extend(items);
        self
    }

    pub fn trigger_variant(mut self, variant: MoonButtonVariant) -> Self {
        self.trigger_variant = variant;
        self
    }

    pub fn trigger_size(mut self, size: MoonButtonSize) -> Self {
        self.trigger_size = size;
        self
    }

    pub fn trigger_width(mut self, width: f32) -> Self {
        self.trigger_width = Some(width);
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

    pub fn default_open(mut self, default_open: bool) -> Self {
        self.default_open = default_open;
        self
    }

    pub fn open(mut self, open: bool) -> Self {
        self.controlled_open = Some(open);
        self
    }

    pub fn menu_width(mut self, width: f32) -> Self {
        self.menu_width = width;
        self
    }

    pub fn menu_offset(mut self, x: f32, y: f32) -> Self {
        self.menu_offset_x = x;
        self.menu_offset_y = y;
        self
    }

    pub fn menu_size(mut self, size: MoonMenuSize) -> Self {
        self.menu_size = size;
        self
    }

    pub fn menu_max_height(mut self, max_height: f32) -> Self {
        self.menu_max_height = Some(max_height);
        self
    }

    pub fn close_on_select(mut self, close_on_select: bool) -> Self {
        self.close_on_select = close_on_select;
        self
    }

    pub fn on_select(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select = Some(std::rc::Rc::new(handler));
        self
    }

    pub fn on_open_change(
        mut self,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_open_change = Some(std::rc::Rc::new(handler));
        self
    }

    fn trigger_height(&self) -> f32 {
        match self.trigger_size {
            MoonButtonSize::Micro => 18.0,
            MoonButtonSize::Toolbar => 28.0,
            MoonButtonSize::Action => 26.0,
            MoonButtonSize::Pill => 30.0,
            MoonButtonSize::Custom { height, .. } => height,
        }
    }

    fn render_trigger(&self) -> impl IntoElement {
        let trigger_id = SharedString::from(format!("{}:trigger", self.id));
        let mut trigger = MoonButton::new(trigger_id)
            .variant(self.trigger_variant)
            .size(self.trigger_size)
            .selected(self.selected)
            .disabled(self.disabled);

        if let Some(bounds) = self.bounds {
            trigger = trigger.bounds(MoonRect::new(0.0, 0.0, bounds.w, bounds.h));
        } else if let Some(width) = self.trigger_width {
            trigger = trigger.width(width);
        }

        if self.segments.is_empty() {
            trigger = trigger.label(self.label.clone());
        } else {
            for segment in self.segments.clone() {
                trigger = trigger.segment(segment);
            }
        }

        trigger.render()
    }
}

impl RenderOnce for MoonDropdown {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state_id = ElementId::from(SharedString::from(format!("{}:moon-state", self.id)));
        let state = window.use_keyed_state(state_id, cx, |_, _| MoonDropdownState {
            open: self.default_open,
        });

        let controlled_open = self.controlled_open;
        let open = controlled_open.unwrap_or_else(|| state.read(cx).open);
        let on_open_change = self.on_open_change.clone();
        let parent_view = window.current_view();
        let trigger_height = self
            .bounds
            .map(|bounds| bounds.h)
            .unwrap_or_else(|| self.trigger_height());
        let trigger = self.render_trigger().into_any_element();

        let mut root = div()
            .id(ElementId::from(SharedString::from(format!(
                "{}:root",
                self.id
            ))))
            .relative();

        if let Some(bounds) = self.bounds {
            root = root
                .absolute()
                .left(px(bounds.x))
                .top(px(bounds.y))
                .w(px(bounds.w))
                .h(px(bounds.h));
        }

        if self.disabled {
            return root.child(trigger).into_any_element();
        }

        let menu_id = SharedString::from(format!("{}:menu", self.id));
        let items = self.items;
        let menu_size = self.menu_size;
        let menu_width = self.menu_width;
        let menu_max_height = self.menu_max_height;
        let menu_offset_x = self.menu_offset_x;
        let menu_offset_y = self.menu_offset_y;
        let close_on_select = self.close_on_select;
        let on_select = self.on_select.clone();
        let popup_items = wire_dropdown_items(
            items,
            close_on_select,
            on_select,
            state.clone(),
            controlled_open,
            self.on_open_change.clone(),
            parent_view,
        );

        let mut popover = CorePopover::new(ElementId::from(self.id.clone()))
            .appearance(false)
            .anchor(Anchor::TopLeft)
            .deferred_priority(30_000)
            .open(open)
            .trigger_any(trigger)
            .content(move |_, _window, cx| {
                let p = MoonPalette::active(cx);
                let mut menu = MoonPopupMenu::new(menu_id.clone())
                    .items(popup_items.clone())
                    .size(menu_size)
                    .width(menu_width)
                    .mono(true);
                if let Some(max_height) = menu_max_height {
                    menu = menu.max_height(max_height);
                }

                div()
                    .mt(px(trigger_height + menu_offset_y))
                    .ml(px(menu_offset_x))
                    .child(menu.render_with_palette(p))
            });

        {
            let state = state.clone();
            let on_open_change = on_open_change.clone();
            popover = popover.on_open_change(move |open, window, cx| {
                if let Some(on_open_change) = on_open_change.as_ref() {
                    on_open_change(*open, window, cx);
                }
                if controlled_open.is_none() {
                    state.update(cx, |state, _| {
                        state.open = *open;
                    });
                    cx.notify(parent_view);
                }
            });
        }

        root.child(popover).into_any_element()
    }
}
