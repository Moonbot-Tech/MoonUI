use std::cell::RefCell;
use std::rc::Rc;

use gpui::prelude::FluentBuilder;
use gpui::*;

use crate::ElementExt;

use super::badge::{MoonBadge, MoonBadgeSize, MoonBadgeVariant};
use super::button::{MoonButtonSize, MoonButtonVariant};
use super::dropdown::{MoonDropdown, MoonMenuItem, MoonMenuSize};
use super::foundation::{accent_underline, h_flex};
use super::icons::MOON_ICON_CARET_DOWN;
use super::text::MoonText;
use super::theme::{MoonTheme, MoonThemeTokens};
use super::tokens::{MoonPalette, MoonRect, rgba_from};

#[derive(Clone, Debug)]
pub struct MoonTabItem {
    label: SharedString,
    badge: Option<SharedString>,
    width: Option<f32>,
    selected: bool,
    disabled: bool,
    closable: bool,
}

impl MoonTabItem {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            badge: None,
            width: None,
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
        self.width = Some(width);
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
    overflow_menu: bool,
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
            overflow_menu: false,
            on_click: None,
            on_close: None,
        }
    }

    /// Constrain the strip to an optional max size.
    ///
    /// `MoonRect.x` / `MoonRect.y` are ignored; they are not a position. The root stays in-flow
    /// (`relative`) whether or not bounds are set.
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

    /// Show a Moon chevron dropdown of every tab after the scroll region. Default `false`
    /// (same as TabBar::menu). Chart, main-stack, and dock pass `true`.
    pub fn overflow_menu(mut self, overflow_menu: bool) -> Self {
        self.overflow_menu = overflow_menu;
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

    pub fn render_with_palette(
        self,
        window: &mut Window,
        cx: &mut App,
        p: MoonPalette,
    ) -> impl IntoElement {
        self.render_with_theme(window, cx, p, MoonThemeTokens::default())
    }

    /// Renders the strip with explicit palette and scale tokens.
    ///
    /// Tabs are in-flow flex children. When `MoonTabItem::width` is omitted the tab sizes to its
    /// label, badge, close control, and token paddings. A supplied width remains a fixed override
    /// and the label yields (`flex_1` + `min_w_0`) so badge and close stay inside that box.
    ///
    /// Args:
    ///     window: Window used to persist the scroll handle and tab bounds.
    ///     cx: App context used to read keyed state.
    ///     p: Palette used for the strip and item states.
    ///     tokens: Scale tokens used for tab geometry.
    ///
    /// Returns:
    ///     The themed tab-strip element.
    pub fn render_with_theme(
        self,
        window: &mut Window,
        cx: &mut App,
        p: MoonPalette,
        tokens: MoonThemeTokens,
    ) -> impl IntoElement {
        let strip_id = self.id.clone();
        let tab_h = tokens.fit_height(28.0, 13.0, 7.5);
        let selected_ix = self.items.iter().position(|item| item.selected);
        let item_metas: Vec<(SharedString, bool, bool)> = self
            .items
            .iter()
            .map(|item| (item.label.clone(), item.selected, item.disabled))
            .collect();
        let on_click_menu = self.on_click.clone();

        let scroll_handle = window
            .use_keyed_state(format!("{strip_id}-scroll"), cx, |_, _| {
                ScrollHandle::default()
            })
            .read(cx)
            .clone();
        let prev_selected =
            window.use_keyed_state(format!("{strip_id}-selected"), cx, |_, _| None::<usize>);
        let bounds_rc = window
            .use_keyed_state(format!("{strip_id}-tab-bounds"), cx, |_, _| {
                Rc::new(RefCell::new(Vec::<Bounds<Pixels>>::new()))
            })
            .read(cx)
            .clone();
        bounds_rc
            .borrow_mut()
            .resize(self.items.len(), Bounds::default());

        if prev_selected.read(cx).as_ref() != selected_ix.as_ref() {
            if let Some(ix) = selected_ix {
                let recorded = bounds_rc.borrow().get(ix).copied().unwrap_or_default();
                if recorded.size.width > px(0.0) {
                    scroll_selected_tab_into_view(&scroll_handle, recorded);
                } else {
                    scroll_handle.scroll_to_item(ix);
                }
            }
            prev_selected.update(cx, |value, _| *value = selected_ix);
        }

        let mut root = div()
            .id(self.id)
            .relative()
            .flex()
            .flex_row()
            .items_center()
            .min_w_0()
            .w_full()
            .h(px(tab_h))
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
            root = root.max_w(px(bounds.w)).max_h(px(bounds.h));
        }

        let mut tabs_inner = h_flex()
            .id("tabs-inner")
            .relative()
            .pl(px(self.padding_left))
            .gap(px(self.gap))
            .overflow_x_scroll()
            .track_scroll(&scroll_handle);

        for (ix, item) in self.items.into_iter().enumerate() {
            let tab = render_moon_tab(
                ix,
                item,
                p,
                tokens.clone(),
                self.on_click.clone(),
                self.on_close.clone(),
            );
            let bounds_rc = bounds_rc.clone();
            tabs_inner = tabs_inner.child(
                div()
                    .flex_none()
                    .on_prepaint(move |bounds, _, _| {
                        if let Some(slot) = bounds_rc.borrow_mut().get_mut(ix) {
                            *slot = bounds;
                        }
                    })
                    .child(tab),
            );
        }

        root = root.child(
            h_flex()
                .id("tabs")
                .flex_1()
                .min_w_0()
                .overflow_x_hidden()
                .child(tabs_inner),
        );

        if self.overflow_menu {
            root = root.child(div().flex_none().child(overflow_menu_dropdown(
                strip_id,
                item_metas,
                on_click_menu,
            )));
        }

        root
    }
}

impl RenderOnce for MoonTabStrip {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        // ScrollHandle persist and layout both run here so they have a Window.
        // Creating a new handle every frame is forbidden; render_with_theme uses
        // window.use_keyed_state(..., ScrollHandle::default()).
        let tokens = MoonTheme::active_tokens(cx);
        self.render_with_theme(window, cx, MoonPalette::active(cx), tokens)
    }
}

fn render_moon_tab(
    ix: usize,
    item: MoonTabItem,
    p: MoonPalette,
    tokens: MoonThemeTokens,
    on_click: Option<MoonTabHandler>,
    on_close: Option<MoonTabHandler>,
) -> impl IntoElement {
    let active = item.selected;
    let disabled = item.disabled;
    let closable = item.closable;
    let fixed_width = item.width;
    let fg = if active { p.text } else { p.text_muted };
    let fg_alpha = if disabled { 0.45 } else { 1.0 };

    let label = MoonText::new(item.label)
        .color(fg)
        .alpha(fg_alpha)
        .font_size(10.0)
        .line_height(13.0)
        .weight(if active { 600.0 } else { 400.0 })
        .mono(true)
        .uppercase(false)
        .render();

    let mut tab = div()
        .id(("moon-tab", ix))
        .relative()
        .h(px(tokens.fit_height(28.0, 13.0, 7.5)))
        .flex()
        .flex_none()
        .items_center()
        .pl(px(tokens.ui(8.0)))
        .pr(px(tokens.ui(if closable { 5.0 } else { 8.0 })))
        .gap(px(tokens.ui(8.0)))
        .when(disabled, |this| this.cursor(CursorStyle::Arrow))
        .when(!disabled, |this| this.cursor_pointer())
        .when(!active && !disabled, |this| {
            this.hover(move |this| this.bg(rgba_from(p.overlay, 0.018)))
                .active(move |this| this.bg(rgba_from(p.overlay, 0.012)))
        });

    if let Some(width) = fixed_width {
        tab = tab.w(px(width));
        // The label is the only child allowed to yield: `flex_1 + min_w_0` lets a long
        // label clip inside the fixed-width tab instead of pushing the badge and the
        // close button out of it, and `justify_center` centres it in whatever space is
        // left. Centring here rather than on the tab row itself is deliberate — on the
        // row it would centre the whole label+badge+close cluster and drag the close
        // button inward. No top margin: the row is already `items_center`, so an extra
        // one only pushes the text off the vertical axis.
        tab = tab.child(
            h_flex()
                .flex_1()
                .min_w_0()
                .justify_center()
                .overflow_hidden()
                .child(label),
        );
    } else {
        tab = tab.child(label);
    }

    if let Some(on_click) = on_click.clone()
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
                .bg_color(if active { p.accent } else { p.overlay })
                .bg_alpha(if active { 0.80 } else { 0.06 })
                .text_color(if active { p.shell } else { p.text_soft })
                .weight(600.0)
                // No top margin: the badge shares the row's `items_center` axis with the
                // label and the close button, and one would reintroduce the offset the
                // label just lost.
                .disabled(disabled)
                .render_with_theme(p, tokens.clone()),
        );
    }

    if closable {
        let on_close = on_close.clone();
        tab = tab.child(
            div()
                .id(("moon-tab-close", ix))
                .w(px(tokens.ui(16.0)))
                .h(px(tokens.ui(16.0)))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(tokens.ui(3.0)))
                .text_size(px(tokens.font(10.0)))
                .line_height(px(tokens.line_height(10.0)))
                .text_color(rgba_from(p.text_muted, 0.90))
                .hover(move |this| this.bg(rgba_from(p.overlay, 0.045)))
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
        tab = tab.child(moon_active_tab_underline_scaled(p, tokens));
    }

    tab
}

fn overflow_menu_dropdown(
    strip_id: ElementId,
    items: Vec<(SharedString, bool, bool)>,
    on_click: Option<MoonTabHandler>,
) -> MoonDropdown {
    let mut menu = MoonDropdown::new(format!("{strip_id}-overflow"))
        .trigger_icon(MOON_ICON_CARET_DOWN)
        .trigger_caret(false)
        .trigger_variant(MoonButtonVariant::Ghost)
        .trigger_size(MoonButtonSize::Micro)
        .menu_size(MoonMenuSize::Compact);
    for (ix, (label, selected, disabled)) in items.into_iter().enumerate() {
        let mut item = MoonMenuItem::with_key(format!("{strip_id}-overflow-{ix}"), label)
            .checked(selected)
            .selected(selected)
            .disabled(disabled);
        if !disabled {
            if let Some(on_click) = on_click.clone() {
                item = item.on_click(move |event, window, cx| {
                    on_click(ix, event, window, cx);
                });
            }
        }
        menu = menu.item(item);
    }
    menu
}

fn scroll_selected_tab_into_view(handle: &ScrollHandle, tab: Bounds<Pixels>) {
    let container = handle.bounds();
    if container.size.width <= px(0.0) || tab.size.width <= px(0.0) {
        return;
    }
    let mut offset = handle.offset();
    if tab.left() + offset.x < container.left() {
        offset.x = container.left() - tab.left();
        handle.set_offset(offset);
    } else if tab.right() + offset.x > container.right() {
        offset.x = container.right() - tab.right();
        handle.set_offset(offset);
    }
}

/// Акцентный underline активной вкладки (точный вид MoonTabStrip), адаптивный по ширине:
/// fade-in слева, сплошной центр (растягивается), fade-out справа, с мягкой тенью.
/// Абсолютно позиционируется по низу родителя (родитель должен быть `relative`).
/// Единый источник вида для верхних (MoonTabStrip) и нижних (dock TabPanel) вкладок.
pub fn moon_active_tab_underline(p: MoonPalette) -> Div {
    moon_active_tab_underline_scaled(p, MoonThemeTokens::default())
}

pub fn moon_active_tab_underline_scaled(p: MoonPalette, tokens: MoonThemeTokens) -> Div {
    accent_underline(p, &tokens, 5.0, 5.0, 0.0)
}
