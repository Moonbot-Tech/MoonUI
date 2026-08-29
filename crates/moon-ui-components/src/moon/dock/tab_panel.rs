//! Rendering implementation for the Moon dock's tab-panel adapter.

use gpui::prelude::FluentBuilder as _;
use gpui::{
    App, AppContext as _, ElementId, InteractiveElement as _, IntoElement, MouseButton,
    MouseDownEvent, ParentElement as _, RenderOnce, ScrollHandle, SharedString,
    StatefulInteractiveElement as _, Styled as _, WeakEntity, Window, div, px,
};

use super::{DockEvent, DockTabDrag, MoonTabPanelRuntimeState, TabPanel, tab_interaction_policy};
use crate::{
    event::InteractiveElementExt as _,
    moon::{
        MOON_ICON_CARET_DOWN, MoonDropdown, MoonMenuItem, MoonMenuSize,
        background::MoonBackgroundPolicy,
        button::{MoonButton, MoonButtonSize, MoonButtonVariant},
        h_flex,
        text::MoonText,
        theme::MoonTheme,
        tokens::{MoonPalette, rgba_from},
    },
};

impl RenderOnce for TabPanel {
    /// Render the tab group and synchronize edge-triggered panel activation notifications.
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = MoonPalette::active(cx);
        let tokens = MoonTheme::active_tokens(cx);
        let state = window.use_keyed_state(
            ElementId::from(SharedString::from(format!("{}:state", self.id))),
            cx,
            |_, _| MoonTabPanelRuntimeState {
                active_ix: self.active_ix,
                notified_active: None,
            },
        );
        if self.dock_area.is_some() && state.read(cx).active_ix != self.active_ix {
            state.update(cx, |state, _| {
                state.active_ix = self.active_ix;
                state.notified_active = None;
            });
        }
        if let Some(dock_area) = self.dock_area.as_ref().and_then(WeakEntity::upgrade) {
            dock_area.update(cx, |dock, _| {
                dock.tab_runtime_states
                    .insert(self.id.to_string(), state.downgrade());
            });
        }
        let active_ix = state
            .read(cx)
            .active_ix
            .min(self.items.len().saturating_sub(1));
        let active_panel = self.items.get(active_ix).cloned();
        let content_policy = self
            .content_background_policy
            .or_else(|| {
                active_panel
                    .as_ref()
                    .map(|panel| panel.background_policy(cx))
            })
            .unwrap_or(MoonBackgroundPolicy::Opaque);
        let parent_view = window.current_view();
        // Announce the front tab ON CHANGE, and tell the tabs behind it that they are hidden.
        //
        // Both halves are edge-triggered: `render` runs every frame, so notifying unconditionally
        // would turn a state change into a 60 Hz poll and lease every panel entity per frame. The
        // inherited dock notifies on transition too — the two docks in this crate must not disagree
        // about what `Panel::set_active` means. A panel that is hidden the whole time is told once,
        // which is what an unread indicator needs to distinguish "on screen" from "behind a tab".
        let announced = (
            active_panel.as_ref().map(|panel| panel.panel_name(cx)),
            self.items.len(),
        );
        if state.read(cx).notified_active.as_ref() != Some(&announced) {
            for (ix, panel) in self.items.iter().enumerate() {
                panel.set_active(ix == active_ix, window, cx);
            }
            state.update(cx, |state, _| {
                state.notified_active = Some(announced.clone());
            });
        }

        let mut root = div()
            .id(ElementId::from(self.id.clone()))
            .relative()
            .size_full()
            .min_w(px(0.0))
            .min_h(px(0.0))
            .flex()
            .flex_col()
            .overflow_hidden()
            .border(px(1.0))
            .border_color(rgba_from(p.border, 1.0));
        root = self.background_policy.apply(root, p.shell_high, 0.98);

        if self.show_header {
            let mut header = div()
                .id(ElementId::from(SharedString::from(format!(
                    "{}:header",
                    self.id
                ))))
                .h(px(tokens.fit_height(29.0, 13.0, 8.0)))
                .flex()
                .flex_none()
                .items_center()
                .gap(px(tokens.ui(4.0)))
                .px(px(tokens.ui(6.0)))
                .border_b(px(1.0))
                .border_color(rgba_from(p.border, 1.0));
            header = self.header_background_policy.apply(header, p.panel, 1.0);

            let tabs_scroll_handle = window
                .use_keyed_state(
                    ElementId::from(SharedString::from(format!("{}:tabs-scroll", self.id))),
                    cx,
                    |_, _| ScrollHandle::default(),
                )
                .read(cx)
                .clone();
            let prev_tab_ix = window.use_keyed_state(
                ElementId::from(SharedString::from(format!("{}:tabs-selected", self.id))),
                cx,
                |_, _| None::<usize>,
            );
            if prev_tab_ix.read(cx).as_ref() != Some(&active_ix) {
                tabs_scroll_handle.scroll_to_item(active_ix);
                prev_tab_ix.update(cx, |value, _| *value = Some(active_ix));
            }
            let mut tabs_inner = h_flex()
                .id(ElementId::from(SharedString::from(format!(
                    "{}:tabs-inner",
                    self.id
                ))))
                .relative()
                .gap(px(tokens.ui(4.0)))
                .overflow_x_scroll()
                .track_scroll(&tabs_scroll_handle);
            let mut overflow_items: Vec<MoonMenuItem> = Vec::new();

            for (ix, panel) in self.items.iter().enumerate() {
                let selected = ix == active_ix;
                let state = state.clone();
                let dock_area = self.dock_area.clone();
                let dock_root = self.dock_root;
                let dock_path = self.dock_path.clone();
                let panel_name = panel.panel_name(cx);
                let tab_label = panel.tab_name(cx).unwrap_or_else(|| panel.panel_name(cx));
                {
                    let state = state.clone();
                    let dock_area = self.dock_area.clone();
                    let dock_root = self.dock_root;
                    let dock_path = self.dock_path.clone();
                    overflow_items.push(
                        MoonMenuItem::with_key(
                            format!("{}:overflow:{ix}", self.id),
                            tab_label.clone(),
                        )
                        .checked(selected)
                        .selected(selected)
                        .on_click(move |_, _, cx| {
                            state.update(cx, |state, _| state.active_ix = ix);
                            if let (Some(dock_area), Some(root)) = (dock_area.as_ref(), dock_root) {
                                _ = dock_area.update(cx, |dock, cx| {
                                    dock.activate_tab_from_user(root, &dock_path, ix, cx);
                                });
                            }
                            cx.notify(parent_view);
                        }),
                    );
                }
                let pinned = self
                    .pinned_leading_panels
                    .iter()
                    .any(|name| name.as_ref() == panel_name.as_ref());
                let last_pinned = pinned
                    && self.items.get(ix + 1).map_or(true, |next| {
                        !self
                            .pinned_leading_panels
                            .iter()
                            .any(|name| name.as_ref() == next.panel_name(cx).as_ref())
                    });
                let tab_debug_selector = format!("{}:tab-host:{ix}", self.id);
                // The panel's own element right of the label (an unread badge, a status dot). The
                // inherited dock already renders `title_suffix` in its tab bar; the Moon dock did
                // not, so a Moon-hosted panel had no way to put anything on its tab.
                //
                // Asked of the BACKGROUND tabs only. A suffix announces what the user is not
                // looking at, and the dock is where "not looking at it" is already known — leaving
                // it to each panel would make every one of them mirror this flag to answer a
                // question its caller had in hand.
                let tab_suffix = (!selected)
                    .then(|| panel.title_suffix(window, cx))
                    .flatten();
                // Match the top MoonTabStrip: a 28-unit mono tab with an amber bottom underline
                // instead of a panel-colored active background. Drag, drop, and double-click all
                // share this host so docking behavior stays independent of its presentation.
                let mut tab_host = div()
                    .id(ElementId::from(SharedString::from(format!(
                        "{}:tab-host:{ix}",
                        self.id
                    ))))
                    .debug_selector(move || tab_debug_selector)
                    .relative()
                    .h(px(tokens.fit_height(28.0, 13.0, 7.5)))
                    .flex()
                    .flex_none()
                    .items_center()
                    .px(px(tokens.ui(8.0)))
                    .cursor_pointer()
                    .when(!selected, |this| {
                        this.hover(move |h| h.bg(rgba_from(p.overlay, 0.018)))
                            .active(move |a| a.bg(rgba_from(p.overlay, 0.012)))
                    })
                    .child(
                        div().mt(px(tokens.ui(2.0))).child(
                            MoonText::new(tab_label)
                                .color(if selected { p.text } else { p.text_muted })
                                .font_size(10.0)
                                .line_height(13.0)
                                .weight(if selected { 600.0 } else { 400.0 })
                                .mono(true)
                                .render(),
                        ),
                    )
                    .children(tab_suffix.map(|suffix| {
                        div()
                            .mt(px(tokens.ui(2.0)))
                            .ml(px(tokens.ui(5.0)))
                            .flex()
                            .flex_none()
                            .items_center()
                            .child(suffix)
                    }))
                    .on_click(move |_, _, cx| {
                        state.update(cx, |state, _| state.active_ix = ix);
                        if let (Some(dock_area), Some(root)) = (dock_area.as_ref(), dock_root) {
                            _ = dock_area.update(cx, |dock, cx| {
                                dock.activate_tab_from_user(root, &dock_path, ix, cx);
                            });
                        }
                        cx.notify(parent_view);
                    })
                    // Right-click hands the tab to the host, which owns whatever menu a tab
                    // offers. Propagation stops here so the click cannot also reach a
                    // window-level handler behind the header.
                    .on_mouse_down(MouseButton::Right, {
                        let dock_area = self.dock_area.clone();
                        let panel_name = panel_name.clone();
                        move |event: &MouseDownEvent, _window, cx| {
                            if let Some(dock_area) =
                                dock_area.as_ref().and_then(|area| area.upgrade())
                            {
                                dock_area.update(cx, |_dock, cx| {
                                    cx.emit(DockEvent::TabContextMenu {
                                        panel_name: panel_name.clone(),
                                        position: event.position,
                                    });
                                });
                            }
                            // Consumed either way: the tab owns right-click, so a teardown-time
                            // click with the dock already gone must not fall through to whatever
                            // sits behind the header.
                            cx.stop_propagation();
                        }
                    });
                if pinned {
                    tab_host = tab_host
                        .bg(rgba_from(p.accent, 0.08))
                        .when(last_pinned, |tab| {
                            tab.border_r(px(tokens.ui(1.0)))
                                .border_color(rgba_from(p.accent, 0.72))
                        });
                }
                if selected {
                    // Reuse the exact palette-backed underline from the top tab strip.
                    tab_host = tab_host.child(crate::moon::tab::moon_active_tab_underline_scaled(
                        p,
                        tokens.clone(),
                    ));
                }
                let interactions =
                    tab_interaction_policy(self.layout_editable, pinned, self.detach_allowed);
                if interactions.accepts_drop {
                    if let (Some(dock_area), Some(root)) = (self.dock_area.clone(), self.dock_root)
                    {
                        if let Some(dock_entity) = dock_area.upgrade() {
                            let dock_id = dock_entity.entity_id();
                            let drop_dock_area = dock_area.clone();
                            let drop_path = self.dock_path.clone();
                            tab_host = tab_host
                                .drag_over::<DockTabDrag>(|style, _, _, cx| {
                                    let p = MoonPalette::active(cx);
                                    style
                                        .border_l(px(2.0))
                                        .border_color(rgba_from(p.accent, 0.9))
                                })
                                .on_drop(move |drag: &DockTabDrag, _window, cx| {
                                    if drag.dock_id != dock_id {
                                        return;
                                    }
                                    _ = drop_dock_area.update(cx, |dock, cx| {
                                        _ = if drag.root == root && drag.path == drop_path {
                                            dock.move_tab_before_from_user(
                                                root,
                                                &drop_path,
                                                drag.panel_name.as_ref(),
                                                ix,
                                                cx,
                                            )
                                        } else {
                                            dock.move_panel_to_tabs_from_user(
                                                drag.panel_name.as_ref(),
                                                root,
                                                &drop_path,
                                                ix,
                                                cx,
                                            )
                                        };
                                    });
                                });
                            if interactions.draggable {
                                let drag = DockTabDrag {
                                    dock_id,
                                    root,
                                    path: self.dock_path.clone(),
                                    panel_name: panel_name.clone(),
                                    splittable: panel.show_dock_header(cx),
                                };
                                tab_host = tab_host.on_drag(drag, |drag, _, _, cx| {
                                    cx.stop_propagation();
                                    cx.new(|_| drag.clone())
                                });
                            }
                            if interactions.detachable {
                                tab_host = tab_host.on_double_click({
                                    let dbl_area = dock_area.clone();
                                    let dbl_name = panel_name.clone();
                                    move |_, _, cx| {
                                        // A tab double-click requests the same detach action as
                                        // the header button; the host owns the detached window.
                                        if let Some(area) = dbl_area.upgrade() {
                                            area.update(cx, |dock, cx| {
                                                dock.request_detach_from_user(dbl_name.clone(), cx);
                                            });
                                        }
                                    }
                                });
                            }
                        }
                    }
                }
                tabs_inner = tabs_inner.child(tab_host);
            }

            header = header.child(
                h_flex()
                    .id(ElementId::from(SharedString::from(format!(
                        "{}:tabs",
                        self.id
                    ))))
                    .flex_1()
                    .min_w_0()
                    .overflow_x_hidden()
                    .child(tabs_inner),
            );
            header = header.child(
                div().flex_none().child(
                    MoonDropdown::new(format!("{}:overflow", self.id))
                        .trigger_icon(MOON_ICON_CARET_DOWN)
                        .trigger_caret(false)
                        .trigger_variant(MoonButtonVariant::Ghost)
                        .trigger_size(MoonButtonSize::Micro)
                        .menu_size(MoonMenuSize::Compact)
                        .items(overflow_items),
                ),
            );
            // Leftover space lives inside the tab cluster (`flex_1 min_w_0`); a second `flex_1`
            // sibling here would steal half the header. Toolbar buttons stay `flex_none` after
            // the chevron.
            if let Some(panel) = active_panel.as_ref() {
                let mut tools = h_flex().flex_none().items_center().gap(px(tokens.ui(4.0)));
                if let Some(buttons) = panel.toolbar_buttons(window, cx) {
                    for button in buttons {
                        tools = tools.child(button);
                    }
                }
                if self.show_panel_controls {
                    let panel_name = panel.panel_name(cx);
                    if self.layout_editable && self.detach_allowed && panel.detachable(cx) {
                        let dock_area = self.dock_area.clone();
                        tools = tools.child(
                            MoonButton::new(format!("{}:detach", self.id))
                                .label("⧉")
                                .size(MoonButtonSize::Micro)
                                .variant(MoonButtonVariant::Ghost)
                                .on_click({
                                    let panel_name = panel_name.clone();
                                    move |_, _, cx| {
                                        if let Some(dock_area) =
                                            dock_area.as_ref().and_then(|area| area.upgrade())
                                        {
                                            dock_area.update(cx, |dock, cx| {
                                                dock.request_detach_from_user(
                                                    panel_name.clone(),
                                                    cx,
                                                );
                                            });
                                        }
                                    }
                                })
                                .render(),
                        );
                    }
                    if panel.zoomable(cx) {
                        let dock_area = self.dock_area.clone();
                        let zoom_label = if self
                            .dock_area
                            .as_ref()
                            .and_then(|dock_area| dock_area.upgrade())
                            .and_then(|dock_area| dock_area.read(cx).zoomed_panel.as_ref().cloned())
                            .as_ref()
                            == Some(&panel_name)
                        {
                            "□"
                        } else {
                            "▣"
                        };
                        tools = tools.child(
                            MoonButton::new(format!("{}:zoom", self.id))
                                .label(zoom_label)
                                .size(MoonButtonSize::Micro)
                                .variant(MoonButtonVariant::Ghost)
                                .on_click({
                                    let panel_name = panel_name.clone();
                                    move |_, window, cx| {
                                        if let Some(dock_area) =
                                            dock_area.as_ref().and_then(|area| area.upgrade())
                                        {
                                            dock_area.update(cx, |dock, cx| {
                                                dock.toggle_zoom_panel(
                                                    panel_name.clone(),
                                                    window,
                                                    cx,
                                                );
                                            });
                                        }
                                    }
                                })
                                .render(),
                        );
                    }
                    if self.layout_editable && self.close_allowed && panel.closable(cx) {
                        let dock_area = self.dock_area.clone();
                        tools = tools.child(
                            MoonButton::new(format!("{}:close", self.id))
                                .label("×")
                                .size(MoonButtonSize::Micro)
                                .variant(MoonButtonVariant::Ghost)
                                .on_click({
                                    let panel_name = panel_name.clone();
                                    move |_, _window, cx| {
                                        if let Some(dock_area) =
                                            dock_area.as_ref().and_then(|area| area.upgrade())
                                        {
                                            // The host decides whether close returns the panel to
                                            // its home strip or destroys it; the dock only asks.
                                            dock_area.update(cx, |dock, cx| {
                                                dock.request_close_from_user(
                                                    panel_name.clone(),
                                                    cx,
                                                );
                                            });
                                        }
                                    }
                                })
                                .render(),
                        );
                    }
                }
                header = header.child(tools);
            }

            root = root.child(header);
        }

        let mut content = div()
            .id(ElementId::from(SharedString::from(format!(
                "{}:content",
                self.id
            ))))
            .relative()
            .flex_1()
            .w_full()
            .min_h(px(0.0))
            .overflow_hidden();
        content = content_policy.apply(content, p.shell, 1.0);

        if let Some(panel) = active_panel {
            let mut panel_host = div().absolute().top_0().right_0().bottom_0().left_0();
            panel_host = content_policy.apply(panel_host, p.shell, 1.0);
            content = content.child(panel_host.child(panel.render_panel(window, cx)));
        }

        root.child(content)
    }
}
