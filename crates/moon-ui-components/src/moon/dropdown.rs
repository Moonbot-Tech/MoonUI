use crate::popover::Popover as CorePopover;
use gpui::prelude::FluentBuilder;
use gpui::*;

use super::{
    button::{MoonButton, MoonButtonSegment, MoonButtonSize, MoonButtonVariant},
    foundation::{MoonClickHandler, MoonSelectHandler, selected_background},
    icons::{MOON_ICON_CHECK, moon_icon},
    text::MoonText,
    theme::{MoonTheme, MoonThemeTokens},
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

impl MenuMetrics {
    fn scaled(self, tokens: &MoonThemeTokens) -> Self {
        let line_height = tokens.line_height(self.line_height);
        Self {
            row_height: tokens.ui(self.row_height).max(line_height + tokens.ui(4.0)),
            font_size: self.font_size,
            line_height: self.line_height,
            radius: tokens.ui(self.radius),
            pad_x: tokens.ui(self.pad_x),
            gap: tokens.ui(self.gap),
        }
    }
}

fn moon_menu_item_accepts_click(kind: MoonMenuItemKind, disabled: bool) -> bool {
    matches!(kind, MoonMenuItemKind::Item) && !disabled
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MoonDropdownSelectPlan {
    close_menu: bool,
    update_internal_open: bool,
}

fn moon_dropdown_select_plan(
    close_on_select: bool,
    controlled_open: Option<bool>,
) -> MoonDropdownSelectPlan {
    MoonDropdownSelectPlan {
        close_menu: close_on_select,
        update_internal_open: close_on_select && controlled_open.is_none(),
    }
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
        self.render_with_theme(p, MoonThemeTokens::default())
    }

    fn render_with_theme(self, p: MoonPalette, tokens: MoonThemeTokens) -> AnyElement {
        let metrics = self.metrics().scaled(&tokens);
        self.render_with_metrics(p, metrics, tokens)
    }

    /// Renders the menu with precomputed layout metrics and the supplied theme tokens.
    fn render_with_metrics(
        self,
        p: MoonPalette,
        metrics: MenuMetrics,
        tokens: MoonThemeTokens,
    ) -> AnyElement {
        let id = self.id.clone();
        let mono = self.mono;
        let shadow = super::foundation::box_shadow(
            px(0.0),
            px(8.0),
            px(18.0),
            px(0.0),
            rgba_from(p.shadow, 0.46),
        );

        let mut menu = div()
            .id(ElementId::from(self.id.clone()))
            // Addressable from `VisualTestContext::debug_bounds` so a test can assert where the
            // menu lands after the deferred/anchored pass. Field, setter and paint-time record are
            // all `cfg`-gated to test builds, so this costs a release build nothing.
            .debug_selector(|| id.to_string())
            .relative()
            .w(px(self.width))
            .p(px(tokens.ui(4.0)))
            .rounded(px(tokens.ui(5.0)))
            .border(px(1.0))
            .border_color(rgba_from(p.border, 1.0))
            .bg(rgba_from(p.shell_high, 0.98))
            .shadow(vec![shadow])
            .occlude()
            .flex()
            .flex_col()
            .gap(px(tokens.ui(2.0)));

        if let Some(max_height) = self.max_height {
            menu = menu.max_h(px(max_height)).overflow_y_scroll();
        }

        for header in self.headers {
            menu = menu.child(header);
        }

        for (ix, item) in self.items.into_iter().enumerate() {
            menu = menu.child(Self::render_item(
                &id,
                mono,
                ix,
                item,
                metrics,
                self.width,
                p,
                tokens.clone(),
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
        tokens: MoonThemeTokens,
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
                } else if selected {
                    p.selected_fg()
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
                    .when(selected, |this| this.bg(selected_background(p)))
                    .when(!disabled, |this| {
                        this.hover(move |this| this.bg(rgba_from(p.overlay, 0.055)))
                            .active(move |this| this.bg(rgba_from(p.overlay, 0.032)))
                    })
                    .child(
                        div()
                            .w(px(12.0))
                            .h(px(tokens.line_height(metrics.line_height)))
                            .flex()
                            .items_center()
                            .justify_center()
                            .when(checked, |this| {
                                this.child(moon_icon(
                                    MOON_ICON_CHECK,
                                    tokens.ui(11.0),
                                    p.accent,
                                    alpha,
                                ))
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

                if moon_menu_item_accepts_click(MoonMenuItemKind::Item, disabled) {
                    if let Some(on_click) = item.on_click {
                        row = row
                            .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                                cx.stop_propagation();
                            })
                            .on_click(move |event, window, cx| {
                                on_click(event, window, cx);
                            });
                    }
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
                                    .width(menu_width)
                                    .render_with_metrics(p, metrics, tokens.clone()),
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
        let tokens = MoonTheme::active_tokens(cx);
        self.render_with_theme(MoonPalette::active(cx), tokens)
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
            if moon_menu_item_accepts_click(item.kind, item.disabled) {
                let key = item.key.clone();
                let existing_handler = item.on_click.clone();
                let on_select = on_select.clone();
                let state = state.clone();
                let on_open_change = on_open_change.clone();
                item.on_click = Some(std::rc::Rc::new(move |event, window, cx| {
                    let plan = moon_dropdown_select_plan(close_on_select, controlled_open);
                    if let Some(existing_handler) = existing_handler.as_ref() {
                        existing_handler(event, window, cx);
                    }
                    if let Some(on_select) = on_select.as_ref() {
                        on_select(&key, window, cx);
                    }
                    if plan.close_menu {
                        if let Some(on_open_change) = on_open_change.as_ref() {
                            on_open_change(false, window, cx);
                        }
                        if plan.update_internal_open {
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
    /// Renders the trigger and, while open, its deferred anchored menu.
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state_id = ElementId::from(SharedString::from(format!("{}:moon-state", self.id)));
        let state = window.use_keyed_state(state_id, cx, |_, _| MoonDropdownState {
            open: self.default_open,
        });

        let controlled_open = self.controlled_open;
        let open = controlled_open.unwrap_or_else(|| state.read(cx).open);
        let on_open_change = self.on_open_change.clone();
        let parent_view = window.current_view();
        // How much the open menu must clear before its own gap. It depends on how the trigger is
        // laid out, so it is decided here rather than at the `.mt(..)` that consumes it.
        //
        // In flow (no caller-supplied bounds) the answer is ZERO: the anchor `CorePopover` hands
        // the menu already sits on the trigger's BOTTOM edge, because `ElementExt::on_prepaint`
        // measures an absolutely positioned canvas appended after the trigger, and in a block
        // container such a child takes its static position — below the preceding in-flow sibling.
        // Adding the height here too would push the menu down by a second trigger height.
        //
        // With caller-supplied bounds `MoonButton::bounds` renders the trigger ABSOLUTELY, which
        // leaves the popover host without a single in-flow child: its auto height collapses to
        // zero, the `size_full` canvas collapses with it, and the capture lands on the trigger's
        // TOP edge. Only there does the supplied height have to be added back.
        let trigger_height = self.bounds.map_or(0.0, |bounds| bounds.h);
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
                let tokens = MoonTheme::active_tokens(cx);
                let mut menu = MoonPopupMenu::new(menu_id.clone())
                    .items(popup_items.clone())
                    .size(menu_size)
                    .width(menu_width)
                    .mono(true);
                if let Some(max_height) = menu_max_height {
                    menu = menu.max_height(max_height);
                }

                div()
                    .mt(px(trigger_height + tokens.ui(menu_offset_y)))
                    .ml(px(tokens.ui(menu_offset_x)))
                    .child(menu.render_with_theme(p, tokens))
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

#[cfg(test)]
mod tests {
    use super::{
        MoonButtonSize, MoonDropdown, MoonDropdownSelectPlan, MoonMenuItem, MoonMenuItemKind,
        MoonRect, moon_dropdown_select_plan, moon_menu_item_accepts_click,
    };
    use std::{cell::RefCell, rc::Rc};

    const DROPDOWN_ID: &str = "geometry-probe";
    /// `MoonDropdown::render` names its menu `{id}:menu`, and `MoonPopupMenu` registers that id as
    /// its debug selector.
    const MENU_SELECTOR: &str = "geometry-probe:menu";

    /// Root view holding one OPEN `MoonDropdown`, recording the laid-out bounds of the
    /// wrapper's direct children — i.e. the dropdown root's box, which is the trigger's box.
    /// The menu is not among them: it renders into a `deferred(anchored(..))` layer, so it is
    /// reachable only through `VisualTestContext::debug_bounds`.
    ///
    /// `trigger_rect` selects which of the two placement paths is exercised: `None` leaves the
    /// dropdown in the parent's flow, `Some(..)` drives the caller-supplied-bounds path, where
    /// the dropdown root is laid out absolutely.
    ///
    /// A `MoonDropdown` cannot be drawn as a bare element: it calls `use_keyed_state`, which
    /// needs a real rendering view on the stack.
    ///
    /// Same measuring idiom as `ButtonHarness` in `moon/button.rs`; the two differ only in how
    /// they drive frames, since the menu renders into a deferred layer.
    struct DropdownHarness {
        trigger_bounds: Rc<RefCell<Vec<gpui::Bounds<gpui::Pixels>>>>,
        trigger_rect: Option<MoonRect>,
    }

    impl gpui::Render for DropdownHarness {
        /// Renders the dropdown and records the trigger bounds after layout.
        fn render(
            &mut self,
            _window: &mut gpui::Window,
            _cx: &mut gpui::Context<Self>,
        ) -> impl gpui::IntoElement {
            use gpui::{ParentElement as _, Styled as _};
            let sink = self.trigger_bounds.clone();
            let mut dropdown = MoonDropdown::new(DROPDOWN_ID)
                .label("trigger")
                // An explicit non-default size, so the measured box is a fixed known one.
                .trigger_size(MoonButtonSize::Action)
                .default_open(true)
                .item(MoonMenuItem::new("only item"));
            if let Some(rect) = self.trigger_rect {
                dropdown = dropdown.bounds(rect);
            }
            // Start-aligned on both axes so the dropdown shrink-wraps its trigger instead of
            // being stretched by the root — the measurement must be the trigger's own box.
            gpui::div()
                .flex()
                .flex_row()
                .items_start()
                .justify_start()
                .on_children_prepainted(move |bounds, _, _| *sink.borrow_mut() = bounds)
                .child(dropdown)
        }
    }

    /// Opens a dropdown in a fresh test window and returns `(trigger box, menu box)`.
    fn open_and_measure(
        cx: &mut gpui::TestAppContext,
        trigger_rect: Option<MoonRect>,
    ) -> (gpui::Bounds<gpui::Pixels>, gpui::Bounds<gpui::Pixels>) {
        cx.update(crate::init);
        let bounds = Rc::new(RefCell::new(Vec::new()));
        let sink = bounds.clone();
        let window = cx.add_window(move |_, _| DropdownHarness {
            trigger_bounds: sink,
            trigger_rect,
        });
        let mut cx = gpui::VisualTestContext::from_window(window.into(), cx);

        // `Popover` renders no content until it has captured its trigger's bounds, and asks for a
        // fresh frame at that moment — so the menu exists only from the frame after the first
        // layout. Drive frames until it appears; the 8 is a bounded retry, not an expected count.
        let menu = (0..8)
            .find_map(|_| {
                cx.update(|window, _| window.refresh());
                cx.run_until_parked();
                cx.debug_bounds(MENU_SELECTOR)
            })
            .expect(
                "open dropdown must render its menu; if `MoonDropdown::render` no longer names it \
                 `{id}:menu`, MENU_SELECTOR is what went stale",
            );

        let recorded = bounds.borrow();
        assert_eq!(
            recorded.len(),
            1,
            "expected exactly the dropdown as a child"
        );
        (recorded[0], menu)
    }

    /// Asserts the menu hangs in the narrow band just under its trigger.
    ///
    /// The gap is the popover content's own `top_1` inset plus `menu_offset_y` — about a third of
    /// the trigger's height at the default scale. Bounding it by HALF the trigger height keeps the
    /// assertion free of any hard-coded rem size or token scale while rejecting an extra
    /// trigger-height offset.
    fn assert_menu_hugs_trigger(
        trigger: gpui::Bounds<gpui::Pixels>,
        menu: gpui::Bounds<gpui::Pixels>,
    ) {
        let gap = menu.origin.y - trigger.bottom();
        assert!(
            gap >= gpui::px(0.0) && gap < trigger.size.height * 0.5,
            "menu must hang just below the trigger: gap {gap:?}, trigger height {:?}",
            trigger.size.height
        );
        assert_eq!(
            menu.origin.x, trigger.origin.x,
            "menu must stay left-aligned with its trigger"
        );
    }

    /// Verifies that an open menu hangs just under its trigger.
    ///
    /// The in-flow anchor already resolves to the trigger's bottom edge, so the menu offset must
    /// contain only the intended gap and not an additional trigger height.
    ///
    /// This is also the tripwire for `ElementExt::on_prepaint`: if its capture is ever corrected
    /// to report the host's true origin, the anchor moves to the trigger's TOP and this test fails
    /// — pointing at the compensating `.mt(...)` in `MoonDropdown::render`.
    #[gpui::test]
    fn open_menu_hangs_just_below_its_trigger(cx: &mut gpui::TestAppContext) {
        let (trigger, menu) = open_and_measure(cx, None);
        assert_menu_hugs_trigger(trigger, menu);
    }

    /// Verifies the caller-supplied-bounds path described at the `trigger_height` binding in
    /// `MoonDropdown::render`.
    ///
    /// This path requires conditional height compensation to keep the menu below its trigger.
    /// Both placement paths must land the menu in the same band.
    ///
    /// No production call site passes `MoonDropdown::bounds` today, in this repo or in
    /// MoonTerminal, so this pins public API that is currently speculative — which is precisely
    /// why it needs a test rather than an argument.
    #[gpui::test]
    fn supplied_bounds_menu_also_hangs_just_below_its_trigger(cx: &mut gpui::TestAppContext) {
        let (trigger, menu) = open_and_measure(cx, Some(MoonRect::new(40.0, 24.0, 120.0, 26.0)));
        assert_menu_hugs_trigger(trigger, menu);
    }

    #[test]
    fn menu_item_clickability_respects_kind_and_disabled_state() {
        assert!(moon_menu_item_accepts_click(MoonMenuItemKind::Item, false));
        assert!(!moon_menu_item_accepts_click(MoonMenuItemKind::Item, true));
        assert!(!moon_menu_item_accepts_click(
            MoonMenuItemKind::Label,
            false
        ));
        assert!(!moon_menu_item_accepts_click(
            MoonMenuItemKind::Separator,
            false
        ));
    }

    #[test]
    fn dropdown_select_plan_respects_close_and_controlled_state() {
        assert_eq!(
            moon_dropdown_select_plan(true, None),
            MoonDropdownSelectPlan {
                close_menu: true,
                update_internal_open: true,
            }
        );
        assert_eq!(
            moon_dropdown_select_plan(true, Some(true)),
            MoonDropdownSelectPlan {
                close_menu: true,
                update_internal_open: false,
            }
        );
        assert_eq!(
            moon_dropdown_select_plan(false, None),
            MoonDropdownSelectPlan {
                close_menu: false,
                update_internal_open: false,
            }
        );
    }
}
