use std::rc::Rc;

use gpui::prelude::*;
use gpui::{
    App, Bounds, Context, Entity, IntoElement, ParentElement, Render, SharedString, Window,
    WindowBounds, WindowOptions, div, point, px, rgb, rgba, size,
};
use gpui_platform::application;
use moon_ui::components::scroll::ScrollableElement;
use moon_ui::{
    DockArea, DockItem, IndexPath, MoonAccent, MoonBackgroundPolicy, MoonBadge, MoonBadgeSize,
    MoonBadgeVariant, MoonButton, MoonButtonIconSlot, MoonButtonSegment, MoonButtonSize,
    MoonButtonVariant, MoonCheckbox, MoonCheckboxSize, MoonColorPicker, MoonColorPickerState,
    MoonContextMenu, MoonDataCell, MoonDataRow, MoonDataTable, MoonDataTableColumn,
    MoonDataTableState, MoonDockPanel, MoonDropdown, MoonInput, MoonInputMaskPattern, MoonMenuItem,
    MoonMenuSize, MoonPalette, MoonPopover, MoonPopoverPlacement, MoonPopupMenu,
    MoonScrollbarVisibility, MoonSegmentItem, MoonSegmentedControl, MoonSelect, MoonSelectItem,
    MoonSelectState, MoonSlider, MoonSliderState, MoonStatusBar, MoonStatusIndicator,
    MoonStatusItem, MoonTabItem, MoonTabStrip, MoonTableCell, MoonTableColumn, MoonTableRow,
    MoonTableStyle, MoonText, MoonTextArea, MoonTheme, MoonThemeConfig, MoonTone, MoonTooltip,
    MoonTooltipPlacement, MoonTooltipSize, MoonTooltipView, MoonVirtualList,
    MoonVirtualListScrollHandle, MoonWindowFrame, MoonWindowFrameBrand, MoonWindowFrameControls,
    PanelView, Root, TabPanel, h_flex, rgba_from, v_flex,
};

const COMPONENT_COVERAGE: &[&str] = &[
    "MoonRoot",
    "MoonBackgroundPolicy",
    "MoonButton",
    "MoonButtonSegment",
    "MoonButtonIconSlot",
    "MoonBadge",
    "MoonCheckbox",
    "MoonColorPicker",
    "MoonContextMenu",
    "MoonDataTable",
    "MoonDockPanel",
    "DockArea",
    "TabPanel",
    "MoonDropdown",
    "MoonPopupMenu",
    "MoonMenuItem",
    "MoonInput",
    "MoonInputMaskPattern",
    "MoonPopover",
    "MoonSegmentedControl",
    "MoonSelect",
    "MoonSlider",
    "MoonStatusBar",
    "MoonTabStrip",
    "MoonTable primitives",
    "MoonText",
    "MoonTextArea",
    "MoonTooltip",
    "MoonTooltipView",
    "MoonVirtualList",
    "MoonWindowFrame",
    "MoonPalette",
];

const GALLERY_PAGES: &[&str] = &["Controls", "Inputs", "Data", "Overlays", "Layout"];

struct Gallery {
    active_page: usize,
    button_clicks: usize,
    alerts_enabled: bool,
    compact_checked: bool,
    segment_index: usize,
    tab_index: usize,
    dropdown_value: SharedString,
    popover_open: bool,
    context_menu_open: bool,
    event_log: Vec<SharedString>,
    select_state: Entity<MoonSelectState<SharedString>>,
    slider_state: Entity<MoonSliderState>,
    range_slider_state: Entity<MoonSliderState>,
    color_state: Entity<MoonColorPickerState>,
    data_table_state: Entity<MoonDataTableState>,
    virtual_scroll: MoonVirtualListScrollHandle,
    tooltip_view: Entity<MoonTooltipView>,
    dock: Entity<DockArea>,
}

impl Gallery {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let select_state = cx.new(|cx| {
            MoonSelectState::new(
                [
                    MoonSelectItem::new(SharedString::from("spot"), "Spot"),
                    MoonSelectItem::new(SharedString::from("futures"), "Futures"),
                    MoonSelectItem::new(SharedString::from("paper"), "Paper").disabled(true),
                ],
                Some(IndexPath::new(1)),
                window,
                cx,
            )
        });
        let slider_state = cx.new(|_| {
            MoonSliderState::new()
                .min(0.0)
                .max(100.0)
                .step(1.0)
                .default_value(63.0)
        });
        let range_slider_state = cx.new(|_| {
            MoonSliderState::new()
                .min(0.0)
                .max(100.0)
                .step(1.0)
                .default_value((18.0, 74.0))
        });
        let color_state =
            cx.new(|cx| MoonColorPickerState::new(window, cx).default_value(rgb(0xFFB347).into()));
        let data_table_state = cx.new(|_| MoonDataTableState::new());
        let tooltip_view =
            cx.new(|_| MoonTooltipView::new("MoonTooltipView entity").max_width(220.0));
        let virtual_scroll = MoonVirtualListScrollHandle::new();
        let dock = cx.new(|cx| DockArea::new("gallery-dock", Some(1), window, cx));
        let dock_items = gallery_dock_panels();
        let dock_weak = dock.downgrade();
        dock.update(cx, |dock, cx| {
            dock.set_center(
                DockItem::tabs(dock_items, &dock_weak, window, cx),
                window,
                cx,
            );
        });

        Self {
            active_page: 0,
            button_clicks: 0,
            alerts_enabled: true,
            compact_checked: true,
            segment_index: 2,
            tab_index: 0,
            dropdown_value: SharedString::from("Auto"),
            popover_open: false,
            context_menu_open: false,
            event_log: vec![SharedString::from("Gallery ready")],
            select_state,
            slider_state,
            range_slider_state,
            color_state,
            data_table_state,
            virtual_scroll,
            tooltip_view,
            dock,
        }
    }

    fn push_event(&mut self, event: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.event_log.insert(0, event.into());
        self.event_log.truncate(10);
        cx.notify();
    }

    fn set_page(&mut self, page: usize, cx: &mut Context<Self>) {
        self.active_page = page.min(GALLERY_PAGES.len().saturating_sub(1));
        self.push_event(format!("Page: {}", GALLERY_PAGES[self.active_page]), cx);
    }

    fn render_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = MoonPalette::active(cx);
        let frame = MoonWindowFrame::main("gallery-window-frame", 1260.0)
            .brand(MoonWindowFrameBrand::Full)
            .controls(MoonWindowFrameControls::MinimizeMaximizeClose);

        h_flex()
            .relative()
            .h(px(36.0))
            .w_full()
            .px(px(12.0))
            .gap(px(12.0))
            .border_b_1()
            .border_color(rgba_from(p.border, 1.0))
            .bg(rgba_from(p.shell, 1.0))
            .child(frame.brand_cluster(cx))
            .child(
                MoonBadge::new("component gallery")
                    .tone(MoonTone::Info)
                    .variant(MoonBadgeVariant::Outline)
                    .render(),
            )
            .child(
                MoonText::new("All Moon visual components through the public moon_ui facade")
                    .uppercase(false)
                    .mono(true)
                    .color(p.text_soft)
                    .font_size(10.5)
                    .line_height(13.0)
                    .render(),
            )
            .child(div().flex_1())
            .child(frame.visual_controls(cx))
    }

    fn render_page_tabs(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = MoonPalette::active(cx);
        h_flex()
            .h(px(42.0))
            .w_full()
            .px(px(14.0))
            .gap(px(8.0))
            .border_b_1()
            .border_color(rgba_from(p.border, 1.0))
            .bg(rgba_from(p.shell_high, 1.0))
            .children(GALLERY_PAGES.iter().enumerate().map(|(ix, page)| {
                MoonButton::new(format!("gallery-page-{ix}"))
                    .label(*page)
                    .variant(if self.active_page == ix {
                        MoonButtonVariant::Blue
                    } else {
                        MoonButtonVariant::Panel
                    })
                    .selected(self.active_page == ix)
                    .on_click(cx.listener(move |this, _, _, cx| this.set_page(ix, cx)))
                    .render()
                    .into_any_element()
            }))
            .child(div().flex_1())
            .child(
                MoonBadge::new(format!("{} components covered", COMPONENT_COVERAGE.len()))
                    .tone(MoonTone::Info)
                    .variant(MoonBadgeVariant::Outline)
                    .render(),
            )
    }

    fn render_event_log(&self, cx: &App) -> impl IntoElement {
        let p = MoonPalette::active(cx);
        let mut body = v_flex()
            .w(px(290.0))
            .h_full()
            .p(px(12.0))
            .gap(px(8.0))
            .border_l_1()
            .border_color(rgba_from(p.border, 1.0))
            .bg(rgba_from(p.shell_high, 0.98))
            .child(
                MoonText::new("Event log")
                    .uppercase(false)
                    .mono(true)
                    .font_size(12.0)
                    .line_height(15.0)
                    .weight(700.0)
                    .color(p.amber)
                    .render(),
            );
        for event in &self.event_log {
            body = body.child(
                MoonText::new(event.clone())
                    .uppercase(false)
                    .mono(true)
                    .wrap()
                    .color(p.text_soft)
                    .render(),
            );
        }
        body
    }

    fn render_controls(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = MoonPalette::active(cx);
        let view = cx.entity();
        section("Controls", cx)
            .child(
                card("Buttons", cx)
                    .child(
                        h_flex()
                            .gap(px(8.0))
                            .flex_wrap()
                            .child(
                                MoonButton::new("btn-neutral")
                                    .label("Neutral")
                                    .variant(MoonButtonVariant::Neutral)
                                    .render(),
                            )
                            .child(
                                MoonButton::new("btn-blue")
                                    .label("Blue")
                                    .variant(MoonButtonVariant::Blue)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.button_clicks += 1;
                                        this.push_event(
                                            format!("Button clicked: {}", this.button_clicks),
                                            cx,
                                        );
                                    }))
                                    .render(),
                            )
                            .child(
                                MoonButton::new("btn-green")
                                    .label("Green")
                                    .variant(MoonButtonVariant::Green)
                                    .render(),
                            )
                            .child(
                                MoonButton::new("btn-danger")
                                    .label("Danger")
                                    .variant(MoonButtonVariant::Danger)
                                    .render(),
                            )
                            .child(
                                MoonButton::new("btn-outline")
                                    .label("Outline")
                                    .variant(MoonButtonVariant::OutlineAmber)
                                    .render(),
                            )
                            .child(
                                MoonButton::new("btn-ghost")
                                    .label("Ghost")
                                    .variant(MoonButtonVariant::Ghost)
                                    .render(),
                            )
                            .child(
                                MoonButton::new("btn-icon")
                                    .leading_icon(MoonButtonIconSlot::new(moon_ui::MOON_ICON_CHECK))
                                    .trailing_icon(MoonButtonIconSlot::new(
                                        moon_ui::MOON_ICON_CARET_DOWN,
                                    ))
                                    .segment(
                                        MoonButtonSegment::new("Segmented")
                                            .color(p.amber)
                                            .weight(700.0),
                                    )
                                    .segment(MoonButtonSegment::new("label").color(p.text_soft))
                                    .tooltip("MoonButton with icon slots and text segments")
                                    .render(),
                            )
                            .child(
                                MoonButton::new("btn-loading")
                                    .label("Loading")
                                    .loading_icon(moon_ui::MOON_ICON_CARET_DOWN)
                                    .loading(true)
                                    .render(),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap(px(8.0))
                            .child(
                                MoonButton::new("btn-micro")
                                    .label("Micro")
                                    .size(MoonButtonSize::Micro)
                                    .render(),
                            )
                            .child(
                                MoonButton::new("btn-action")
                                    .label("Action")
                                    .size(MoonButtonSize::Action)
                                    .render(),
                            )
                            .child(
                                MoonButton::new("btn-pill")
                                    .label("Pill selected")
                                    .size(MoonButtonSize::Pill)
                                    .variant(MoonButtonVariant::Panel)
                                    .selected(true)
                                    .trailing_icon(MoonButtonIconSlot::new(
                                        moon_ui::MOON_ICON_CHECK,
                                    ))
                                    .render(),
                            )
                            .child(
                                MoonButton::new("btn-disabled")
                                    .label("Disabled")
                                    .disabled(true)
                                    .render(),
                            ),
                    ),
            )
            .child(
                card("Badges / Checkbox / Segmented", cx)
                    .child(
                        h_flex()
                            .gap(px(8.0))
                            .flex_wrap()
                            .child(
                                MoonBadge::new("soft")
                                    .tone(MoonTone::Info)
                                    .variant(MoonBadgeVariant::Soft)
                                    .render(),
                            )
                            .child(
                                MoonBadge::new("solid")
                                    .tone(MoonTone::Positive)
                                    .variant(MoonBadgeVariant::Solid)
                                    .render(),
                            )
                            .child(
                                MoonBadge::new("outline")
                                    .tone(MoonTone::Warning)
                                    .variant(MoonBadgeVariant::Outline)
                                    .render(),
                            )
                            .child(MoonBadge::new("").dot().tone(MoonTone::Danger).render())
                            .child(MoonBadge::new("").count_max(128, 99).render())
                            .child(
                                MoonBadge::new("")
                                    .icon(moon_ui::MOON_ICON_CHECK)
                                    .size(MoonBadgeSize::Status)
                                    .render(),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap(px(14.0))
                            .child(
                                MoonCheckbox::new("check-normal")
                                    .label("checked")
                                    .checked(self.alerts_enabled)
                                    .on_change({
                                        let view = view.clone();
                                        move |checked, _, app| {
                                            let checked = *checked;
                                            view.update(app, |this, cx| {
                                                this.alerts_enabled = checked;
                                                this.push_event(
                                                    format!("Alerts checked: {checked}"),
                                                    cx,
                                                );
                                            });
                                        }
                                    }),
                            )
                            .child(
                                MoonCheckbox::new("check-compact")
                                    .label("compact")
                                    .size(MoonCheckboxSize::Compact)
                                    .checked(self.compact_checked)
                                    .on_change({
                                        let view = view.clone();
                                        move |checked, _, app| {
                                            let checked = *checked;
                                            view.update(app, |this, cx| {
                                                this.compact_checked = checked;
                                                this.push_event(
                                                    format!("Compact checked: {checked}"),
                                                    cx,
                                                );
                                            });
                                        }
                                    }),
                            )
                            .child(
                                MoonCheckbox::new("check-indeterminate")
                                    .label("indeterminate")
                                    .indeterminate(true),
                            )
                            .child(
                                MoonCheckbox::new("check-disabled")
                                    .label("disabled")
                                    .disabled(true),
                            ),
                    )
                    .child(
                        MoonSegmentedControl::new("segmented")
                            .accent(MoonAccent::Amber)
                            .items([
                                MoonSegmentItem::new("F1", "0.01")
                                    .width(82.0)
                                    .selected(self.segment_index == 0),
                                MoonSegmentItem::new("F2", "0.025")
                                    .width(82.0)
                                    .selected(self.segment_index == 1),
                                MoonSegmentItem::new("F3", "0.05")
                                    .width(82.0)
                                    .selected(self.segment_index == 2),
                                MoonSegmentItem::new("F4", "0.10")
                                    .width(82.0)
                                    .disabled(true),
                            ])
                            .on_click({
                                let view = view.clone();
                                move |ix, _, _, app| {
                                    view.update(app, |this, cx| {
                                        this.segment_index = ix;
                                        this.push_event(
                                            format!("Segment selected: F{}", ix + 1),
                                            cx,
                                        );
                                    });
                                }
                            })
                            .render(),
                    ),
            )
    }

    fn render_inputs(&self, cx: &App) -> impl IntoElement {
        let p = MoonPalette::active(cx);
        let mask = MoonInputMaskPattern::new("AAA-999");
        let price_mask = MoonInputMaskPattern::number_with_fraction(Some(' '), Some(2));

        section("Inputs", cx)
            .child(
                card("Text inputs", cx)
                    .child(
                        h_flex()
                            .gap(px(8.0))
                            .child(
                                MoonInput::new("input-default")
                                    .placeholder("StrategyName")
                                    .default_value("HooksDetect 0.3-1%")
                                    .small()
                                    .cleanable(true)
                                    .prefix(MoonBadge::new("S").tone(MoonTone::Info).render())
                                    .suffix(MoonBadge::new("ok").tone(MoonTone::Positive).render()),
                            )
                            .child(
                                MoonInput::new("input-password")
                                    .placeholder("API secret")
                                    .default_value("moon-secret-token")
                                    .mask_toggle()
                                    .small(),
                            )
                            .child(
                                MoonInput::new("input-disabled")
                                    .placeholder("disabled")
                                    .default_value("read only")
                                    .disabled(true)
                                    .small(),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap(px(10.0))
                            .child(
                                MoonText::new(format!(
                                    "Mask AAA-999: {} -> {}",
                                    "BOT123",
                                    mask.mask("BOT123")
                                ))
                                .uppercase(false)
                                .mono(true)
                                .color(p.text_soft)
                                .render(),
                            )
                            .child(
                                MoonText::new(format!(
                                    "Number mask: {} -> {}",
                                    "1234567.899",
                                    price_mask.mask("1234567.899")
                                ))
                                .uppercase(false)
                                .mono(true)
                                .color(p.text_soft)
                                .render(),
                            ),
                    ),
            )
            .child(
                card("Text area / Select / Slider / Color", cx)
                    .child(
                        h_flex()
                            .items_start()
                            .gap(px(12.0))
                            .child(
                                v_flex()
                                    .gap(px(8.0))
                                    .w(px(350.0))
                                    .child(
                                        MoonTextArea::new("text-area")
                                            .placeholder("formula / memo")
                                            .default_value(
                                                "CustomEMA(source, fast)\n  and volume > avg(volume, 20)",
                                            )
                                            .formula(),
                                    )
                                    .child(
                                        MoonTextArea::new("text-area-normal")
                                            .placeholder("normal memo")
                                            .default_value("Line one\nLine two")
                                            .rows(3),
                                    ),
                            )
                            .child(
                                v_flex()
                                    .gap(px(10.0))
                                    .w(px(300.0))
                                    .child(
                                        MoonSelect::new(&self.select_state)
                                            .id("gallery-select")
                                            .title_prefix("Market")
                                            .placeholder("Select market")
                                            .cleanable(true)
                                            .searchable(true)
                                            .menu_width(220.0)
                                            .menu_size(MoonMenuSize::Normal),
                                    )
                                    .child(
                                        MoonSlider::new(&self.slider_state)
                                            .id("gallery-slider")
                                            .height(22.0),
                                    )
                                    .child(
                                        MoonSlider::new(&self.range_slider_state)
                                            .id("gallery-range-slider")
                                            .height(22.0),
                                    )
                                    .child(
                                        MoonColorPicker::new(&self.color_state)
                                            .id("gallery-color-picker"),
                                    ),
                            ),
                    ),
            )
    }

    fn render_menus(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        section("Menus / Overlays", cx).child(
            card("Dropdown / PopupMenu / ContextMenu / Popover / Tooltip", cx)
                .relative()
                .min_h(px(330.0))
                .child(
                    h_flex()
                        .items_start()
                        .gap(px(14.0))
                        .child(
                            MoonDropdown::new("gallery-dropdown")
                                .label(format!("Scale {}", self.dropdown_value))
                                .trigger_width(142.0)
                                .default_open(false)
                                .menu_width(170.0)
                                .items([
                                    MoonMenuItem::with_key("Auto", "Auto")
                                        .selected(self.dropdown_value.as_ref() == "Auto"),
                                    MoonMenuItem::with_key("50", "50%")
                                        .selected(self.dropdown_value.as_ref() == "50"),
                                    MoonMenuItem::with_key("20", "20%")
                                        .checked(self.dropdown_value.as_ref() == "20"),
                                    MoonMenuItem::separator(),
                                    MoonMenuItem::new("Advanced").right_label(">").submenu([
                                        MoonMenuItem::new("Bid view"),
                                        MoonMenuItem::new("Ask view"),
                                    ]),
                                ])
                                .on_select({
                                    let view = view.clone();
                                    move |key, _, app| {
                                        let key = key.clone();
                                        view.update(app, |this, cx| {
                                            this.dropdown_value = key.clone();
                                            this.push_event(format!("Dropdown: {key}"), cx);
                                        });
                                    }
                                }),
                        )
                        .child(
                            MoonPopover::new("gallery-popover")
                                .open(self.popover_open)
                                .on_open_change({
                                    let view = view.clone();
                                    move |open, _, app| {
                                        view.update(app, |this, cx| {
                                            this.popover_open = open;
                                            this.push_event(format!("Popover open: {open}"), cx);
                                        });
                                    }
                                })
                                .placement(MoonPopoverPlacement::BottomStart)
                                .width(230.0)
                                .background_policy(MoonBackgroundPolicy::Transparent)
                                .trigger(
                                    MoonButton::new("popover-trigger")
                                        .label("Open popover")
                                        .variant(MoonButtonVariant::Panel)
                                        .render(),
                                )
                                .content(
                                    v_flex()
                                        .gap(px(8.0))
                                        .child(
                                            MoonText::new("Popover content")
                                                .uppercase(false)
                                                .mono(true)
                                                .render(),
                                        )
                                        .child(
                                            MoonButton::new("popover-action")
                                                .label("Action")
                                                .variant(MoonButtonVariant::Blue)
                                                .render(),
                                        ),
                                ),
                        )
                        .child(
                            v_flex()
                                .gap(px(8.0))
                                .w(px(260.0))
                                .child(
                                    MoonButton::new("context-menu-toggle")
                                        .label(if self.context_menu_open {
                                            "Close context menu"
                                        } else {
                                            "Open context menu"
                                        })
                                        .variant(MoonButtonVariant::Panel)
                                        .on_click({
                                            let view = view.clone();
                                            move |_, _, app| {
                                                view.update(app, |this, cx| {
                                                    this.context_menu_open =
                                                        !this.context_menu_open;
                                                    this.push_event(
                                                        format!(
                                                            "Context menu open: {}",
                                                            this.context_menu_open
                                                        ),
                                                        cx,
                                                    );
                                                });
                                            }
                                        })
                                        .render(),
                                )
                                .child(
                                    MoonTooltip::new("Direct tooltip")
                                        .detail("Long text wraps inside MoonTooltip.")
                                        .shortcut("Ctrl+K")
                                        .placement(MoonTooltipPlacement::Top)
                                        .size(MoonTooltipSize::Normal)
                                        .tone(MoonTone::Info)
                                        .max_width(240.0)
                                        .arrow(true),
                                )
                                .child(self.tooltip_view.clone()),
                        ),
                )
                .child(
                    MoonPopupMenu::new("gallery-popup-menu")
                        .width(190.0)
                        .max_height(130.0)
                        .items([
                            MoonMenuItem::new("Popup menu"),
                            MoonMenuItem::new("Checked").checked(true),
                            MoonMenuItem::new("Danger").tone(MoonTone::Danger),
                        ])
                        .render(),
                )
                .child(
                    MoonContextMenu::new("gallery-context-menu")
                        .position(point(px(760.0), px(182.0)))
                        .open(self.context_menu_open)
                        .width(190.0)
                        .items([
                            MoonMenuItem::new("Root context"),
                            MoonMenuItem::new("Move").right_label("M"),
                            MoonMenuItem::new("Delete").tone(MoonTone::Danger),
                        ]),
                ),
        )
    }

    fn render_tables(&self, cx: &App) -> impl IntoElement {
        let p = MoonPalette::active(cx);
        let table_style = MoonTableStyle::for_palette(p);
        let _table_primitives = MoonTableRow::new()
            .selected(true)
            .cell(MoonTableCell::text("MoonTableCell", MoonTone::Info, 600.0))
            .cell(MoonTableCell::text(
                "right aligned",
                MoonTone::Warning,
                500.0,
            ))
            .text_alpha(0.92);
        let _columns = [
            MoonTableColumn::new("Primitive", 140.0),
            MoonTableColumn::new("Align", 140.0).right(),
        ];

        section("Tables / Lists / Dock", cx)
            .child(
                card("MoonDataTable uses MoonTable primitives", cx)
                    .child(
                        MoonDataTable::new("gallery-data-table", 80, move |ix, _, app| {
                            let p = MoonPalette::active(app);
                            MoonDataRow::new([
                                MoonDataCell::text(format!("MOON/{ix:03}"))
                                    .tone(MoonTone::Default)
                                    .weight(600.0),
                                MoonDataCell::text(if ix % 2 == 0 { "LONG" } else { "SHORT" })
                                    .tone(if ix % 2 == 0 {
                                        MoonTone::Positive
                                    } else {
                                        MoonTone::Danger
                                    }),
                                MoonDataCell::text(format!("{:.4}", 0.125 + ix as f32 * 0.007))
                                    .tone(MoonTone::Info),
                                MoonDataCell::element(
                                    MoonBadge::new(if ix % 3 == 0 { "active" } else { "idle" })
                                        .tone(if ix % 3 == 0 {
                                            MoonTone::Positive
                                        } else {
                                            MoonTone::Muted
                                        })
                                        .render(),
                                ),
                                MoonDataCell::text(format!("${:.2}", 1200.0 + ix as f32 * 17.5))
                                    .text_color(if ix % 2 == 0 { p.green } else { p.orange }),
                            ])
                            .selected(ix == 2)
                        })
                        .state(&self.data_table_state)
                        .columns([
                            MoonDataTableColumn::new("market", "MARKET", 120.0)
                                .sortable(true)
                                .fixed_left(),
                            MoonDataTableColumn::new("side", "SIDE", 92.0).sortable(true),
                            MoonDataTableColumn::new("qty", "QTY", 92.0)
                                .right()
                                .sortable(true),
                            MoonDataTableColumn::new("status", "STATUS", 120.0),
                            MoonDataTableColumn::new("pnl", "PNL", 120.0)
                                .right()
                                .fill(),
                        ])
                        .style(table_style)
                        .row_header(true)
                        .cell_selectable(true)
                        .column_selectable(true)
                        .context_menu(|target, _, _| {
                            vec![
                                MoonMenuItem::new(format!("{target:?}")),
                                MoonMenuItem::new("Copy row"),
                                MoonMenuItem::new("Delete").tone(MoonTone::Danger),
                            ]
                        }),
                    )
                    .child(
                        h_flex()
                            .gap(px(8.0))
                            .child(
                                MoonText::new("MoonTableColumn / MoonTableRow / MoonTableCell are public primitives; the renderer is currently internal and is exercised through MoonDataTable.")
                                    .uppercase(false)
                                    .mono(true)
                                    .wrap()
                                    .color(p.text_soft)
                                    .render(),
                            )
                            .child(MoonBadge::new("MoonTable primitives constructed").render()),
                    ),
            )
            .child(
                h_flex()
                    .items_start()
                    .gap(px(12.0))
                    .child(
                        card("MoonVirtualList", cx)
                            .w(px(420.0))
                            .h(px(260.0))
                            .child(
                                MoonVirtualList::new(
                                    "gallery-virtual-list",
                                    500,
                                    30.0,
                                    |ix, _, app| {
                                        let p = MoonPalette::active(app);
                                        h_flex()
                                            .px(px(10.0))
                                            .gap(px(8.0))
                                            .child(MoonBadge::new(format!("{ix:03}")).render())
                                            .child(
                                                MoonText::new(format!("virtual row {ix}"))
                                                    .uppercase(false)
                                                    .mono(true)
                                                    .color(if ix % 2 == 0 { p.text } else { p.text_soft })
                                                    .render(),
                                            )
                                    },
                                )
                                .track_scroll(&self.virtual_scroll)
                                .scrollbar_visibility(MoonScrollbarVisibility::Always)
                                .background_policy(MoonBackgroundPolicy::Opaque)
                                .tail_fill_color(p.shell),
                            ),
                    )
                    .child(
                        card("DockArea / TabPanel / MoonDockPanel", cx)
                            .w(px(520.0))
                            .h(px(260.0))
                            .child(self.dock.clone()),
                    ),
            )
            .child(
                card("Standalone TabPanel", cx)
                    .h(px(190.0))
                    .child(
                        TabPanel::new("gallery-tab-panel", gallery_tab_panels())
                            .active_index(1)
                            .background_policy(MoonBackgroundPolicy::Opaque)
                            .content_background_policy(MoonBackgroundPolicy::Transparent)
                            .header_background_policy(MoonBackgroundPolicy::Opaque),
                    ),
            )
    }

    fn render_navigation(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = MoonPalette::active(cx);
        let view = cx.entity();
        section("Navigation / Status / Tokens", cx)
            .child(
                card("Tabs", cx).child(
                    MoonTabStrip::new("gallery-tabs")
                        .items([
                            MoonTabItem::new("Main").selected(self.tab_index == 0),
                            MoonTabItem::new("Orders")
                                .badge("12")
                                .selected(self.tab_index == 1),
                            MoonTabItem::new("Assets")
                                .closable(true)
                                .selected(self.tab_index == 2),
                            MoonTabItem::new("Disabled").disabled(true),
                        ])
                        .on_click(move |ix, _, _, app| {
                            view.update(app, |this, cx| {
                                this.tab_index = ix;
                                this.push_event(format!("Tab selected: {ix}"), cx);
                            });
                        })
                        .render(),
                ),
            )
            .child(
                card("Window frame variants", cx).child(
                    v_flex()
                        .gap(px(8.0))
                        .child(window_frame_row(
                            MoonWindowFrame::main("wf-main", 520.0),
                            "main window",
                            cx,
                        ))
                        .child(window_frame_row(
                            MoonWindowFrame::tool("wf-tool", 520.0),
                            "tool window",
                            cx,
                        ))
                        .child(window_frame_row(
                            MoonWindowFrame::popup("wf-popup", 520.0),
                            "popup window",
                            cx,
                        ))
                        .child(window_frame_row(
                            MoonWindowFrame::detached_chart("wf-chart", 520.0)
                                .brand(MoonWindowFrameBrand::Mark),
                            "detached chart",
                            cx,
                        ))
                        .child(window_frame_row(
                            MoonWindowFrame::debug("wf-debug", 520.0)
                                .brand(MoonWindowFrameBrand::Mark),
                            "debug window",
                            cx,
                        )),
                ),
            )
            .child(
                card("Palette / StatusBar / Scroll config", cx)
                    .child(
                        h_flex().gap(px(8.0)).flex_wrap().children(
                            [
                                ("shell", p.shell),
                                ("panel", p.panel),
                                ("border", p.border),
                                ("text", p.text),
                                ("green", p.green),
                                ("red", p.red),
                                ("amber", p.amber),
                                ("blue", p.blue),
                                ("accent", p.accent),
                            ]
                            .into_iter()
                            .map(|(name, color)| swatch(name, color).into_any_element())
                            .collect::<Vec<_>>(),
                        ),
                    )
                    .child(
                        MoonStatusBar::new("gallery-status")
                            .indicator(MoonStatusIndicator::new(p.green).glow(8.0, 0.24))
                            .items([
                                MoonStatusItem::new("connected").tone(MoonTone::Positive),
                                MoonStatusItem::separator(),
                                MoonStatusItem::new("vertical scroll").tone(MoonTone::Info),
                                MoonStatusItem::new("overlay scrollbar").tone(MoonTone::Warning),
                            ])
                            .right_items([
                                MoonStatusItem::new("MoonPalette").color(p.amber),
                                MoonStatusItem::new(format!(
                                    "{} components",
                                    COMPONENT_COVERAGE.len()
                                ))
                                .tone(MoonTone::Muted),
                            ])
                            .render(),
                    )
                    .child(
                        MoonText::new(
                            "This gallery keeps shell surfaces opaque, chart/layout hosts transparent, and scrollbars in Moon-styled overlay mode.",
                        )
                        .uppercase(false)
                        .mono(true)
                        .color(p.text_soft)
                        .wrap()
                        .render(),
                    ),
            )
    }
}

impl Render for Gallery {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = MoonPalette::active(cx);
        let page = match self.active_page {
            0 => self.render_controls(cx).into_any_element(),
            1 => self.render_inputs(cx).into_any_element(),
            2 => self.render_tables(cx).into_any_element(),
            3 => self.render_menus(cx).into_any_element(),
            _ => self.render_navigation(cx).into_any_element(),
        };

        v_flex()
            .size_full()
            .bg(rgba_from(p.shell, 1.0))
            .text_color(rgb(p.text))
            .child(self.render_header(cx))
            .child(self.render_page_tabs(cx))
            .child(
                h_flex()
                    .items_start()
                    .flex_1()
                    .min_h_0()
                    .child(
                        v_flex()
                            .flex_1()
                            .h_full()
                            .overflow_y_scrollbar()
                            .p(px(14.0))
                            .gap(px(14.0))
                            .child(page),
                    )
                    .child(self.render_event_log(cx)),
            )
    }
}

fn gallery_dock_panels() -> Vec<Rc<dyn PanelView>> {
    vec![
        dock_panel("gallery-dock-orders", "Orders", MoonTone::Info),
        dock_panel("gallery-dock-log", "Log", MoonTone::Warning),
        dock_panel("gallery-dock-assets", "Assets", MoonTone::Positive),
    ]
}

fn gallery_tab_panels() -> Vec<Rc<dyn PanelView>> {
    vec![
        dock_panel("gallery-tab-alpha", "Alpha", MoonTone::Accent),
        dock_panel("gallery-tab-beta", "Beta", MoonTone::Info),
    ]
}

fn dock_panel(name: &'static str, title: &'static str, tone: MoonTone) -> Rc<dyn PanelView> {
    Rc::new(
        MoonDockPanel::new(name, title, move |_, app| {
            let p = MoonPalette::active(app);
            v_flex()
                .size_full()
                .p(px(10.0))
                .gap(px(8.0))
                .child(
                    MoonText::new(format!("{title} panel"))
                        .uppercase(false)
                        .mono(true)
                        .color(tone.color(p))
                        .font_size(12.0)
                        .line_height(15.0)
                        .weight(600.0)
                        .render(),
                )
                .child(
                    MoonText::new(
                        "MoonDockPanel content with panel controls and background policy.",
                    )
                    .uppercase(false)
                    .mono(true)
                    .wrap()
                    .color(p.text_soft)
                    .render(),
                )
                .into_any_element()
        })
        .detachable(true)
        .closable(true)
        .zoomable(true)
        .background_policy(MoonBackgroundPolicy::Opaque),
    )
}

fn section(title: &'static str, cx: &App) -> gpui::Div {
    let p = MoonPalette::active(cx);
    v_flex().gap(px(10.0)).child(
        MoonText::new(title)
            .uppercase(false)
            .mono(true)
            .font_size(14.0)
            .line_height(18.0)
            .weight(700.0)
            .color(p.text)
            .render(),
    )
}

fn card(title: &'static str, cx: &App) -> gpui::Div {
    let p = MoonPalette::active(cx);
    v_flex()
        .gap(px(10.0))
        .p(px(12.0))
        .rounded(px(6.0))
        .border_1()
        .border_color(rgba_from(p.border, 1.0))
        .bg(rgba_from(p.shell_high, 0.92))
        .child(
            MoonText::new(title)
                .uppercase(false)
                .mono(true)
                .font_size(11.0)
                .line_height(14.0)
                .weight(700.0)
                .color(p.amber)
                .render(),
        )
}

fn swatch(name: &'static str, color: u32) -> impl IntoElement {
    h_flex()
        .gap(px(6.0))
        .child(
            div()
                .size(px(15.0))
                .rounded(px(3.0))
                .border_1()
                .border_color(rgba_from(0x000000, 0.35))
                .bg(rgb(color)),
        )
        .child(
            MoonText::new(format!("{name} #{color:06X}"))
                .uppercase(false)
                .mono(true)
                .font_size(10.0)
                .line_height(12.0)
                .render(),
        )
}

fn window_frame_row(frame: MoonWindowFrame, title: &'static str, cx: &App) -> impl IntoElement {
    let p = MoonPalette::active(cx);
    h_flex()
        .h(px(30.0))
        .px(px(8.0))
        .rounded(px(5.0))
        .border_1()
        .border_color(rgba_from(p.border, 1.0))
        .bg(rgba_from(p.panel, 0.86))
        .child(frame.title_cluster(title, cx))
        .child(div().flex_1())
        .child(frame.visual_controls(cx))
}

fn run_gallery() {
    application().run(|cx: &mut App| {
        moon_ui::foundation::init(cx);
        MoonTheme::install_config(MoonThemeConfig::moon_terminal(), cx);

        let p = MoonPalette::active(cx);
        let bounds = Bounds::centered(None, size(px(1280.0), px(900.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_clear_color: Some(rgba((p.shell << 8) | 0xFF)),
                app_id: Some("pro.moonbot.moon-ui-gallery".to_string()),
                ..Default::default()
            },
            |window, cx| {
                let view = cx.new(|cx| Gallery::new(window, cx));
                cx.new(|cx| {
                    Root::new(view, window, cx)
                        .background_policy(MoonBackgroundPolicy::Opaque)
                        .background(MoonPalette::active(cx).shell)
                })
            },
        )
        .expect("open MoonUI gallery window");
        cx.activate(true);
    });
}

fn main() {
    run_gallery();
}

#[cfg(test)]
mod tests {
    use super::COMPONENT_COVERAGE;

    #[test]
    fn gallery_has_a_visual_coverage_manifest() {
        assert!(COMPONENT_COVERAGE.len() >= 30);
        assert!(COMPONENT_COVERAGE.contains(&"MoonButton"));
        assert!(COMPONENT_COVERAGE.contains(&"MoonDataTable"));
        assert!(COMPONENT_COVERAGE.contains(&"DockArea"));
        assert!(COMPONENT_COVERAGE.contains(&"MoonWindowFrame"));
    }
}
