//! Interactive MoonUI gallery state, pages, and detached-panel rendering.

use super::handoff::window_frame_row;
use super::*;

/// Owns the interactive gallery window state.
pub(super) struct Gallery {
    active_page: usize,
    theme_mode: ThemeMode,
    snapshot: Option<SnapshotRun>,
    button_clicks: usize,
    alerts_enabled: bool,
    /// Opt-in for the indeterminate progress demo; see the comment at its render site.
    progress_loading_demo: bool,
    compact_checked: bool,
    new_toggle_checked: bool,
    new_radio_index: usize,
    new_stepper_value: f32,
    new_switch_checked: bool,
    new_rating_value: usize,
    new_pagination_page: usize,
    new_sidebar_collapsed: bool,
    disclosure_expanded: bool,
    settings_enabled: Rc<Cell<bool>>,
    settings_symbol: Rc<RefCell<SharedString>>,
    settings_mode: Rc<RefCell<SharedString>>,
    settings_risk: Rc<Cell<f64>>,
    segment_index: usize,
    tab_index: usize,
    dropdown_value: SharedString,
    popover_open: bool,
    context_menu_open: bool,
    event_log: Vec<SharedString>,
    pending_detach: Vec<SharedString>,
    select_state: Entity<MoonSelectState<SharedString>>,
    combobox_state: Entity<MoonComboboxState<MoonSearchableVec<&'static str>>>,
    date_picker_state: Entity<MoonDatePickerState>,
    date_time_picker_state: Entity<MoonDateTimePickerState>,
    time_picker_state: Entity<MoonTimePickerState>,
    calendar_state: Entity<MoonCalendarState>,
    list_state: Entity<MoonListState<GalleryListDelegate>>,
    tree_state: Entity<MoonTreeState>,
    controlled_tree_state: Entity<MoonTreeState>,
    slider_state: Entity<MoonSliderState>,
    range_slider_state: Entity<MoonSliderState>,
    color_state: Entity<MoonColorPickerState>,
    data_table_state: Entity<MoonDataTableState>,
    virtual_scroll: MoonVirtualListScrollHandle,
    tooltip_view: Entity<MoonTooltipView>,
    dock: Entity<DockArea>,
}

#[cfg_attr(not(feature = "snapshot"), allow(dead_code))]
struct SnapshotRun {
    dir: PathBuf,
    page_ix: usize,
    capture_scheduled: bool,
    settle_frames: usize,
    next_capture_at: Instant,
    cleaned_dir: bool,
}

impl Gallery {
    /// Builds the interactive gallery and its component state.
    pub(super) fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        active_page: usize,
        snapshot_dir: Option<PathBuf>,
        theme_mode: ThemeMode,
    ) -> Self {
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
        let combobox_state = cx.new(|cx| {
            MoonComboboxState::new(
                MoonSearchableVec::new(vec!["BTCUSDT", "ETHUSDT", "SOLUSDT", "BNBUSDT"]),
                vec![MoonComponentIndexPath::new(0)],
                window,
                cx,
            )
            .searchable(true)
        });
        let date_picker_state = cx.new(|cx| MoonDatePickerState::new(window, cx));
        let calendar_state = cx.new(|cx| MoonCalendarState::new(window, cx));
        let list_state = cx.new(|cx| {
            MoonListState::new(GalleryListDelegate::new(), window, cx)
                .searchable(true)
                .selectable(true)
        });
        let tree_state = cx.new(|cx| {
            MoonTreeState::new(cx).items([
                MoonTreeItem::new("ui", "Moon UI")
                    .expanded(true)
                    .child(MoonTreeItem::new("ui.controls", "Controls"))
                    .child(MoonTreeItem::new("ui.overlays", "Overlays"))
                    .child(MoonTreeItem::new("ui.data", "Data")),
                MoonTreeItem::new("runtime", "Runtime")
                    .expanded(true)
                    .child(MoonTreeItem::new("runtime.gpui", "GPUI fork"))
                    .child(MoonTreeItem::new("runtime.theme", "Theme bridge")),
            ])
        });
        let date_time_picker_state = cx.new(|cx| MoonDateTimePickerState::new(window, cx));
        // Every minute: the standalone control is the one people set an exact time in.
        let time_picker_state = cx.new(|cx| MoonTimePickerState::new(cx));
        let controlled_tree_state = cx.new(|cx| {
            MoonTreeState::new(cx).items([
                MoonTreeItem::new("core.1", "server 1")
                    .folder(true)
                    .child(
                        MoonTreeItem::new("core.1.folder.hooks", "Moon Hook")
                            .folder(true)
                            .child(MoonTreeItem::new("strategy.1", "HooksDetect 0.3-1%"))
                            .child(MoonTreeItem::new("strategy.2", "Delta Reversal")),
                    )
                    .child(MoonTreeItem::new("core.1.folder.empty", "Empty folder").folder(true)),
                MoonTreeItem::new("core.2", "server 2")
                    .folder(true)
                    .child(MoonTreeItem::new("strategy.3", "Scalp Guard")),
            ])
        });
        controlled_tree_state.update(cx, |state, cx| {
            state.set_selection_mode(MoonTreeSelectionMode::Multi, cx);
            state.set_expanded(
                [
                    SharedString::from("core.1"),
                    SharedString::from("core.1.folder.hooks"),
                    SharedString::from("core.2"),
                ],
                cx,
            );
            state.set_selected_ids(
                [
                    SharedString::from("strategy.1"),
                    SharedString::from("strategy.2"),
                ],
                cx,
            );
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
        cx.subscribe(&dock, |this, dock, event: &DockEvent, cx| match event {
            DockEvent::LayoutChanged => {
                let _ = dock;
                this.push_event("Dock layout changed", cx);
            }
            DockEvent::PanelActivated { panel_name } => {
                this.push_event(format!("Dock panel activated: {panel_name}"), cx);
            }
            DockEvent::DetachRequested { panel_name } => {
                this.pending_detach.push(panel_name.clone());
                this.push_event(format!("Dock detach requested: {panel_name}"), cx);
            }
            DockEvent::PanelCloseRequested { panel_name } => {
                this.push_event(format!("Dock close requested: {panel_name}"), cx);
            }
            // The dock owns no tab menu: it reports the right-click and the host decides. The
            // gallery has no per-panel switches to offer, so it only shows that the event lands —
            // right-click a dock tab and the event log records the panel and the click position.
            DockEvent::TabContextMenu {
                panel_name,
                position,
            } => {
                this.push_event(
                    format!(
                        "Dock tab menu requested: {panel_name} at {:.0},{:.0}",
                        f32::from(position.x),
                        f32::from(position.y)
                    ),
                    cx,
                );
            }
        })
        .detach();

        Self {
            active_page: active_page.min(GALLERY_PAGES.len().saturating_sub(1)),
            theme_mode,
            snapshot: snapshot_dir.map(|dir| SnapshotRun {
                dir,
                page_ix: active_page.min(GALLERY_PAGES.len().saturating_sub(1)),
                capture_scheduled: false,
                settle_frames: 8,
                next_capture_at: Instant::now() + Duration::from_millis(500),
                cleaned_dir: false,
            }),
            button_clicks: 0,
            alerts_enabled: true,
            progress_loading_demo: false,
            compact_checked: true,
            new_toggle_checked: true,
            new_radio_index: 1,
            new_stepper_value: 3.0,
            new_switch_checked: true,
            new_rating_value: 3,
            new_pagination_page: 4,
            new_sidebar_collapsed: false,
            disclosure_expanded: false,
            settings_enabled: Rc::new(Cell::new(true)),
            settings_symbol: Rc::new(RefCell::new(SharedString::from("BTCUSDT"))),
            settings_mode: Rc::new(RefCell::new(SharedString::from("paper"))),
            settings_risk: Rc::new(Cell::new(2.5)),
            segment_index: 2,
            tab_index: 0,
            dropdown_value: SharedString::from("Auto"),
            popover_open: false,
            context_menu_open: false,
            event_log: vec![SharedString::from("Gallery ready")],
            pending_detach: Vec::new(),
            select_state,
            combobox_state,
            date_picker_state,
            date_time_picker_state,
            time_picker_state,
            calendar_state,
            list_state,
            tree_state,
            controlled_tree_state,
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

    fn set_theme_mode(&mut self, mode: ThemeMode, cx: &mut Context<Self>) {
        if self.theme_mode == mode {
            return;
        }
        self.theme_mode = mode;
        MoonTheme::set_mode(mode, std::borrow::BorrowMut::borrow_mut(cx));
        self.push_event(format!("Theme: {}", theme_mode_name(mode)), cx);
    }

    fn schedule_snapshot_capture(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(snapshot) = self.snapshot.as_mut() else {
            return;
        };
        if snapshot.capture_scheduled {
            return;
        }
        snapshot.capture_scheduled = true;
        cx.on_next_frame(window, |this, window, cx| {
            this.capture_snapshot_page(window, cx);
        });
    }

    #[cfg(feature = "snapshot")]
    fn capture_snapshot_page(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(snapshot) = self.snapshot.as_mut() else {
            return;
        };
        if snapshot.settle_frames == 8 {
            window.blur();
        }
        if snapshot.settle_frames > 0 {
            snapshot.settle_frames -= 1;
            snapshot.capture_scheduled = false;
            cx.notify();
            return;
        }
        let now = Instant::now();
        if now < snapshot.next_capture_at {
            snapshot.capture_scheduled = false;
            cx.notify();
            return;
        }
        if !snapshot.cleaned_dir {
            if let Err(err) = clear_snapshot_dir(&snapshot.dir) {
                eprintln!(
                    "failed to clear snapshot dir {}: {err}",
                    snapshot.dir.display()
                );
                cx.quit();
                return;
            }
            snapshot.cleaned_dir = true;
        }
        let page = GALLERY_PAGES
            .get(snapshot.page_ix)
            .copied()
            .unwrap_or("unknown");
        if let Err(err) = std::fs::create_dir_all(&snapshot.dir) {
            eprintln!(
                "failed to create snapshot dir {}: {err}",
                snapshot.dir.display()
            );
            cx.quit();
            return;
        }
        let path = snapshot.dir.join(format!("{page}.png"));
        let image = match snapshot_window_image(window) {
            Ok(image) => image,
            Err(err) => {
                eprintln!("snapshot {page} failed: {err}");
                cx.quit();
                return;
            }
        };
        if let Err(err) = image.save(&path) {
            eprintln!("snapshot {page} failed to save {}: {err}", path.display());
            cx.quit();
            return;
        }
        eprintln!("snapshot {page} -> {}", path.display());

        snapshot.page_ix += 1;
        if snapshot.page_ix >= GALLERY_PAGES.len() {
            cx.quit();
            return;
        }
        self.active_page = snapshot.page_ix;
        snapshot.capture_scheduled = false;
        snapshot.settle_frames = 8;
        snapshot.next_capture_at = Instant::now() + Duration::from_millis(700);
        cx.notify();
    }

    #[cfg(not(feature = "snapshot"))]
    fn capture_snapshot_page(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        eprintln!("moon-ui-gallery --snapshot-dir requires `--features snapshot`");
        cx.quit();
    }

    fn render_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = MoonPalette::active(cx);
        let next_mode = match self.theme_mode {
            ThemeMode::Light => ThemeMode::Dark,
            ThemeMode::Dark | ThemeMode::System => ThemeMode::Light,
        };
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
            .child(
                MoonButton::new("gallery-theme-toggle")
                    .label(theme_mode_name(self.theme_mode))
                    .variant(MoonButtonVariant::Panel)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.set_theme_mode(next_mode, cx);
                    }))
                    .render(),
            )
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

    /// Render the controls gallery with interactive button, input, and selector examples.
    ///
    /// Args:
    ///     cx: Gallery context used for theme lookup and interaction listeners.
    ///
    /// Returns:
    ///     The complete controls section.
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
                                    .padding_x(7.0)
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
                card("MoonAccordion", cx).child(
                    MoonAccordion::new("moon-accordion")
                        .multiple(true)
                        .item(|item| {
                            item.title("MoonAccordion item")
                                .open(true)
                                .child(
                                    MoonText::new("Accordion behavior is mirrored from Longbridge behind a Moon-facing API.")
                                        .uppercase(false)
                                        .mono(true)
                                        .wrap()
                                        .color(p.text_soft)
                                        .render(),
                                )
                        })
                        .item(|item| {
                            item.title("Second item").child(
                                MoonText::new("Application code should import MoonAccordion, not moon_ui::components::accordion::Accordion.")
                                    .uppercase(false)
                                    .mono(true)
                                    .wrap()
                                    .color(p.text_soft)
                                    .render(),
                            )
                        })
                        .render(),
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
                            .gap(px(8.0))
                            .flex_wrap()
                            .child(
                                MoonTag::positive()
                                    .rounded_full()
                                    .label("MoonTag positive")
                                    .render(),
                            )
                            .child(
                                MoonTag::warning()
                                    .outline()
                                    .label("MoonTag warning")
                                    .render(),
                            )
                            .child(
                                MoonProgress::new("moon-progress-positive")
                                    .value(68.0)
                                    .tone(MoonTone::Positive)
                                    .render(),
                            )
                            .child(
                                div().w(px(160.0)).child(
                                    MoonProgress::new("moon-progress-loading")
                                        // Opt-in: an endlessly repeating animation forces a full
                                        // window re-render on every vsync for as long as it is on
                                        // screen. Measured on this page in a debug build, leaving
                                        // it always-on cost ~1.2 cores at idle and starved the
                                        // input handling of the landing page.
                                        .loading(self.progress_loading_demo)
                                        .value(if self.progress_loading_demo { 0.0 } else { 42.0 })
                                        .tone(MoonTone::Info)
                                        .height(5.0)
                                        .render(),
                                ),
                            )
                            .child(
                                MoonCheckbox::new("check-progress-loading")
                                    .label("animate")
                                    .size(MoonCheckboxSize::Compact)
                                    .checked(self.progress_loading_demo)
                                    .on_change({
                                        let view = view.clone();
                                        move |checked, _, app| {
                                            let checked = *checked;
                                            view.update(app, |this, cx| {
                                                this.progress_loading_demo = checked;
                                                this.push_event(
                                                    format!("Progress loading animation: {checked}"),
                                                    cx,
                                                );
                                            });
                                        }
                                    }),
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
                                    .fit_width(cx, 52.0, 110.0)
                                    .tooltip("Select preset F1")
                                    .selected(self.segment_index == 0),
                                MoonSegmentItem::new("F2", "0.025")
                                    .fit_width(cx, 52.0, 110.0)
                                    .tooltip("Select preset F2")
                                    .selected(self.segment_index == 1),
                                MoonSegmentItem::new("F3", "0.05")
                                    .fit_width(cx, 52.0, 110.0)
                                    .tooltip("Select preset F3")
                                    .selected(self.segment_index == 2),
                                MoonSegmentItem::new("F4", "0.10")
                                    .fit_width(cx, 52.0, 110.0)
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
                            .on_scroll({
                                let view = view.clone();
                                move |ix, _, _, app| {
                                    view.update(app, |this, cx| {
                                        this.push_event(
                                            format!("Segment scrolled: F{}", ix + 1),
                                            cx,
                                        );
                                    });
                                }
                            })
                            .render(),
                    )
                    // All four accents side by side: `gallery.visual_coverage` is the only
                    // mechanism that catches a segment-colour regression, and a single strip
                    // covers just one enum variant.
                    .child(
                        h_flex().gap_3().children(
                            [
                                ("segmented-amber", MoonAccent::Amber, "Amber"),
                                ("segmented-blue", MoonAccent::Blue, "Blue"),
                                ("segmented-green", MoonAccent::Green, "Green"),
                                ("segmented-red", MoonAccent::Red, "Red"),
                            ]
                            .map(|(id, accent, label)| {
                                MoonSegmentedControl::new(id)
                                    .accent(accent)
                                    .items([
                                        MoonSegmentItem::new("S1", label)
                                            .width(72.0)
                                            .selected(true),
                                        MoonSegmentItem::new("S2", "off").width(52.0),
                                    ])
                                    .render()
                            }),
                        ),
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
        section("Menus / Overlays", cx)
            .child(
                card("MoonAlert", cx)
                    .child(
                        MoonAlert::info(
                            "moon-alert-info",
                            "MoonAlert mirrors Longbridge alert behavior behind a Moon-facing API.",
                        )
                        .title("Info alert")
                        .render(),
                    )
                    .child(
                        MoonAlert::warning(
                            "moon-alert-warning",
                            "Raw Alert stays visible on NewControls until the escape path is removed.",
                        )
                        .title("Warning alert")
                        .render(),
                    ),
            )
            .child(
                card("MoonDialog / MoonNotification", cx).child(
                    h_flex()
                        .gap(px(8.0))
                        .flex_wrap()
                        .child(
                            MoonButton::new("moon-dialog-open")
                                .label("Open MoonDialog")
                                .variant(MoonButtonVariant::Panel)
                                .on_click(|_, window, app| {
                                    window.open_unique_moon_dialog(
                                        "gallery-moon-dialog",
                                        app,
                                        |dialog, _window, _cx| {
                                            dialog
                                                .w(px(300.0))
                                                .title(div().child("MoonDialog"))
                                                .content(|content, _window, _cx| {
                                                    content.child(div().child(
                                                        "Dialog opened through MoonWindowExt.",
                                                    ))
                                                })
                                        },
                                    );
                                })
                                .render(),
                        )
                        .child(
                            MoonButton::new("moon-notification-push")
                                .label("Push notification")
                                .variant(MoonButtonVariant::Panel)
                                .on_click(|_, window, app| {
                                    window.push_notification(
                                        MoonNotification::info("Root-owned MoonNotification")
                                            .title("MoonNotification")
                                            .autohide(false),
                                        app,
                                    );
                                })
                                .render(),
                        ),
                ),
            )
            .child(
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
                                .trigger_leading_icon(MoonButtonIconSlot::new(
                                    moon_ui::MOON_ICON_CHECK,
                                ))
                                .trigger_caret(true)
                                .fit_trigger_width(100.0, 180.0)
                                .default_open(false)
                                .fit_menu_width(220.0, 560.0)
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
                                    MoonMenuItem::new(
                                        "Only current market orders with a long translated label",
                                    ),
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
                                .content_width_ui(230.0)
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
                                            move |_, window, app| {
                                                let opening = !view.read(app).context_menu_open;
                                                view.update(app, |this, cx| {
                                                    this.context_menu_open = opening;
                                                    this.push_event(
                                                        format!(
                                                            "Context menu open: {}",
                                                            this.context_menu_open
                                                        ),
                                                        cx,
                                                    );
                                                });
                                                if opening {
                                                    let weak = view.downgrade();
                                                    window.open_fitted_moon_context_menu_with_dismiss(
                                                        app,
                                                        "gallery-context-menu",
                                                        window.mouse_position(),
                                                        vec![
                                                            MoonMenuItem::new("Root context"),
                                                            MoonMenuItem::new(
                                                                "Add to selected cores blacklist (56)",
                                                            ),
                                                            MoonMenuItem::new(
                                                                "Add to strategy with a deliberately long name",
                                                            ),
                                                            MoonMenuItem::new("Delete")
                                                                .tone(MoonTone::Danger),
                                                        ],
                                                        220.0..=560.0,
                                                        move |window, app| {
                                                            window.close_context_menu(app);
                                                            _ = weak.update(app, |this, cx| {
                                                                this.context_menu_open = false;
                                                                this.push_event(
                                                                    "Context menu dismissed",
                                                                    cx,
                                                                );
                                                            });
                                                        },
                                                    );
                                                } else {
                                                    window.close_context_menu(app);
                                                }
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
                        .max_height_ui(130.0)
                        .items([
                            MoonMenuItem::new("Popup menu"),
                            MoonMenuItem::new("Checked").checked(true),
                            MoonMenuItem::new("Danger").tone(MoonTone::Danger),
                        ])
                        .render(),
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

    /// Render navigation, window-chrome, palette, and status-bar compositions.
    ///
    /// Args:
    ///     cx: Gallery context used to read the active theme and wire interactions.
    ///
    /// Returns:
    ///     The complete Navigation gallery page.
    fn render_navigation(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = MoonPalette::active(cx);
        let tab_view = cx.entity();
        let status_view = cx.entity();
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
                            tab_view.update(app, |this, cx| {
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
                                MoonStatusItem::group_separator(),
                                MoonStatusItem::new("overlay scrollbar").tone(MoonTone::Warning),
                            ])
                            .right_items([
                                MoonStatusItem::new("MoonPalette")
                                    .id("gallery-status-action")
                                    .color(p.amber)
                                    .tooltip("Click to record a status action")
                                    .on_click(move |_, _, app| {
                                        status_view.update(app, |this, cx| {
                                            this.push_event("Status action clicked", cx);
                                        });
                                    }),
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

    fn render_new_controls(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = MoonPalette::active(cx);
        let view = cx.entity();
        let settings_enabled = self.settings_enabled.clone();
        let settings_symbol = self.settings_symbol.clone();
        let settings_mode = self.settings_mode.clone();
        let settings_risk = self.settings_risk.clone();

        section("NewControls / Ready Moon adaptations", cx)
            .child(
                card("What this page means", cx)
                    .child(
                        MoonText::new(
                            "This page shows adapted Moon-facing controls that are already usable by applications. A Longbridge component is not allowed here just because it has a wrapper; it must look and behave like Moon UI first.",
                        )
                        .uppercase(false)
                        .mono(true)
                        .wrap()
                        .color(p.text_soft)
                        .render(),
                    )
                    .child(
                        h_flex()
                            .gap(px(8.0))
                            .flex_wrap()
                            .child(MoonBadge::new("MoonReady").tone(MoonTone::Positive).render())
                            .child(MoonBadge::new("Longbridge behavior").tone(MoonTone::Info).render())
                            .child(MoonBadge::new("Visual checked").tone(MoonTone::Accent).render()),
                    ),
            )
            .child(
                h_flex()
                    .items_start()
                    .gap(px(12.0))
                    .child(
                        card("Feedback", cx)
                            .w(px(500.0))
                            .child(
                                MoonAlert::success(
                                    "new-controls-ready-alert",
                                    "MoonAlert keeps Longbridge behavior behind a Moon-facing API.",
                                )
                                .title("MoonAlert")
                                .render(),
                            )
                            .child(
                                h_flex()
                                    .gap(px(8.0))
                                    .child(MoonTag::positive().label("MoonTag").render())
                                    .child(MoonTag::warning().label("warning").render())
                                    .child(MoonTag::danger().outline().label("outline").render()),
                            )
                            .child(
                                div().w(px(240.0)).child(
                                    MoonProgress::new("new-controls-progress")
                                        .value(68.0)
                                        .tone(MoonTone::Positive)
                                        .height(5.0)
                                        .render(),
                                ),
                            ),
                    )
                    .child(
                        card("Root-owned overlays", cx)
                            .w(px(500.0))
                            .child(
                                h_flex()
                                    .gap(px(8.0))
                                    .flex_wrap()
                                    .child(
                                        MoonButton::new("new-controls-dialog")
                                            .label("Open MoonDialog")
                                            .variant(MoonButtonVariant::Panel)
                                            .on_click(|_, window, app| {
                                                window.open_unique_moon_dialog(
                                                    "new-controls-dialog",
                                                    app,
                                                    |dialog, _window, _cx| {
                                                        dialog
                                                            .w(px(300.0))
                                                            .title(div().child("MoonDialog"))
                                                            .content(|content, _window, _cx| {
                                                                content.child(div().child(
                                                                    "Opened through MoonWindowExt.",
                                                                ))
                                                            })
                                                    },
                                                );
                                            })
                                            .render(),
                                    )
                                    .child(
                                        MoonButton::new("new-controls-notification")
                                            .label("Push MoonNotification")
                                            .variant(MoonButtonVariant::Panel)
                                            .on_click(|_, window, app| {
                                                window.push_notification(
                                                    MoonNotification::info(
                                                        "MoonNotification from NewControls",
                                                    )
                                                    .title("MoonNotification")
                                                    .autohide(false),
                                                    app,
                                                );
                                            })
                                            .render(),
                                    )
                                    .child(
                                        MoonButton::new("new-controls-native-menu")
                                            .label("Open native menu")
                                            .variant(MoonButtonVariant::Panel)
                                            .on_click(|_, window, app| {
                                                MoonNativeMenu::new()
                                                    .label("MoonNativeMenu")
                                                    .menu("No-op action", Box::new(NoAction))
                                                    .menu_with_check(
                                                        "Checked item",
                                                        true,
                                                        Box::new(NoAction),
                                                    )
                                                    .separator()
                                                    .submenu(
                                                        "Submenu",
                                                        MoonNativeMenu::new().menu(
                                                            "Nested item",
                                                            Box::new(NoAction),
                                                        ),
                                                    )
                                                    .show(point(px(180.0), px(180.0)), window, app);
                                            })
                                            .render(),
                                    )
                            )
                            .child(
                                MoonAccordion::new("new-controls-accordion")
                                    .item(|item| {
                                        item.title("MoonAccordion").open(true).child(
                                            "Longbridge expansion behavior, Moon-facing API.",
                                        )
                                    })
                                    .render(),
                            ),
                    ),
            )
            .child(
                card("Choice controls", cx)
                    .child(
                        h_flex()
                            .gap(px(18.0))
                            .flex_wrap()
                            .child(
                                MoonToggle::new("new-controls-toggle")
                                    .checked(self.new_toggle_checked)
                                    .label("overlay hints")
                                    .on_change({
                                        let view = view.clone();
                                        move |checked, _, app| {
                                            let checked = *checked;
                                            view.update(app, |this, cx| {
                                                this.new_toggle_checked = checked;
                                                this.push_event(format!("MoonToggle: {checked}"), cx);
                                            });
                                        }
                                    }),
                            )
                            .child(
                                MoonToggle::new("new-controls-toggle-compact")
                                    .checked(false)
                                    .label("compact")
                                    .size(MoonToggleSize::Compact),
                            )
                            .child(MoonSpinner::new().tone(MoonTone::Info))
                            .child(MoonKbd::new("Ctrl+K"))
                            .child(MoonKbd::new("Esc").outline(true)),
                    )
                    .child(
                        h_flex()
                            .gap(px(18.0))
                            .items_center()
                            .flex_wrap()
                            .child(
                                MoonSwitch::new("new-controls-switch")
                                    .checked(self.new_switch_checked)
                                    .label("MoonSwitch")
                                    .tooltip("Longbridge switch behavior through Moon facade")
                                    .on_click({
                                        let view = view.clone();
                                        move |checked, _, app| {
                                            let checked = *checked;
                                            view.update(app, |this, cx| {
                                                this.new_switch_checked = checked;
                                                this.push_event(format!("MoonSwitch: {checked}"), cx);
                                            });
                                        }
                                    }),
                            )
                            .child(
                                MoonRating::new("new-controls-rating")
                                    .value(self.new_rating_value)
                                    .max(5)
                                    .on_click({
                                        let view = view.clone();
                                        move |value, _, app| {
                                            let value = *value;
                                            view.update(app, |this, cx| {
                                                this.new_rating_value = value;
                                                this.push_event(format!("MoonRating: {value}"), cx);
                                            });
                                        }
                                    }),
                            ),
                    )
                    .child(MoonSeparator::horizontal().alpha(0.72))
                    .child(
                        h_flex()
                            .items_center()
                            .gap(px(10.0))
                            .flex_wrap()
                            .child(
                                MoonLink::new("new-controls-link", "MoonLink")
                                    .on_click({
                                        let view = view.clone();
                                        move |_, _, app| {
                                            view.update(app, |this, cx| {
                                                this.push_event("MoonLink clicked", cx);
                                            });
                                        }
                                    }),
                            )
                            .child(
                                div()
                                    .w(px(180.0))
                                    .child(
                                        MoonSkeleton::new("new-controls-skeleton")
                                            .height(8.0)
                                            .animated(false),
                                    ),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap(px(14.0))
                            .flex_wrap()
                            .child(
                                MoonRadio::new("new-controls-radio-fast")
                                    .label("fast")
                                    .checked(self.new_radio_index == 0)
                                    .on_change({
                                        let view = view.clone();
                                        move |_, _, app| {
                                            view.update(app, |this, cx| {
                                                this.new_radio_index = 0;
                                                this.push_event("MoonRadio: fast", cx);
                                            });
                                        }
                                    }),
                            )
                            .child(
                                MoonRadio::new("new-controls-radio-balanced")
                                    .label("balanced")
                                    .checked(self.new_radio_index == 1)
                                    .on_change({
                                        let view = view.clone();
                                        move |_, _, app| {
                                            view.update(app, |this, cx| {
                                                this.new_radio_index = 1;
                                                this.push_event("MoonRadio: balanced", cx);
                                            });
                                        }
                                    }),
                            )
                            .child(
                                MoonRadio::new("new-controls-radio-safe")
                                    .label("safe")
                                    .checked(self.new_radio_index == 2)
                                    .disabled(true),
                            ),
                    ),
            )
            .child(
                h_flex()
                    .items_start()
                    .gap(px(12.0))
                    .child(
                        card("Form primitives", cx)
                            .w(px(500.0))
                            .child(
                                MoonSurface::new()
                                    .id("new-controls-surface-card")
                                    .variant(MoonSurfaceVariant::Card)
                                    .child(
                                        v_flex()
                                            .gap(px(10.0))
                                            .p(px(12.0))
                                            .child(
                                                MoonLabel::new("MoonLabel + MoonSurface")
                                                    .color(p.text_soft)
                                                    .font_size(10.5)
                                                    .line_height(13.0)
                                                    .weight(600.0)
                                                    .mono(true)
                                                    .uppercase(false)
                                                    .render(),
                                            )
                                            .child(
                                                MoonGroupBox::new("new-controls-group-box")
                                                    .title("MoonGroupBox")
                                                    .child(
                                                        MoonFormRow::new(
                                                            "new-controls-form-row-selector",
                                                            "Market",
                                                        )
                                                        .label_width(96.0)
                                                        .control(
                                                            MoonSelectorPill::new(
                                                                "new-controls-form-selector",
                                                            )
                                                            .leading_dot(p.green)
                                                            .segment(
                                                                MoonSelectorSegment::new("default")
                                                                    .color(p.text_muted),
                                                            )
                                                            .segment(
                                                                MoonSelectorSegment::new("BTCUSDT")
                                                                    .color(p.text)
                                                                    .weight(600.0),
                                                            )
                                                            .render(),
                                                        ),
                                                    )
                                                    .child(
                                                        MoonFormRow::new(
                                                            "new-controls-form-row-stepper",
                                                            "Risk",
                                                        )
                                                        .label_width(96.0)
                                                        .control(
                                                            MoonStepper::new(
                                                                "new-controls-stepper",
                                                            )
                                                            .value(self.new_stepper_value)
                                                            .range(0.0, 10.0)
                                                            .step(0.5)
                                                            .precision(1)
                                                            .tone(MoonTone::Warning)
                                                            .on_change({
                                                                let view = view.clone();
                                                                move |value, _, app| {
                                                                    view.update(app, |this, cx| {
                                                                        this.new_stepper_value =
                                                                            value;
                                                                        this.push_event(
                                                                            format!(
                                                                                "MoonStepper: {value:.1}"
                                                                            ),
                                                                            cx,
                                                                        );
                                                                    });
                                                                }
                                                            })
                                                            .render(),
                                                        ),
                                                    ),
                                            ),
                                    ),
                            ),
                    )
                    .child(
                        card("Toolbar primitives", cx)
                            .w(px(500.0))
                            .child(
                                MoonSurface::new()
                                    .id("new-controls-surface-sidebar")
                                    .variant(MoonSurfaceVariant::Sidebar)
                                    .child(
                                        v_flex()
                                            .gap(px(10.0))
                                            .p(px(12.0))
                                            .child(
                                                MoonCollapsible::new(
                                                    "new-controls-collapsible",
                                                )
                                                .title("MoonCollapsible")
                                                .default_open(true)
                                                .content(
                                                    MoonText::new(
                                                        "Expanded content keeps the Moon surface, border, typography and spacing rules.",
                                                    )
                                                    .uppercase(false)
                                                    .mono(true)
                                                    .wrap()
                                                    .color(p.text_soft)
                                                    .render(),
                                                ),
                                            )
                                            .child(
                                                h_flex()
                                                    .gap(px(8.0))
                                                    .items_center()
                                                    .child(
                                                        MoonDisclosure::button(
                                                            "new-controls-disclosure",
                                                            self.disclosure_expanded,
                                                        )
                                                        .box_size(14.0)
                                                        .hover_color(p.text)
                                                        .tooltip(if self.disclosure_expanded {
                                                            "Collapse"
                                                        } else {
                                                            "Expand"
                                                        })
                                                        .on_toggle({
                                                            let view = view.clone();
                                                            move |next, _, app| {
                                                                let next = *next;
                                                                view.update(app, |this, cx| {
                                                                    this.disclosure_expanded = next;
                                                                    this.push_event(
                                                                        format!(
                                                                            "MoonDisclosure expanded: {next}"
                                                                        ),
                                                                        cx,
                                                                    );
                                                                });
                                                            }
                                                        }),
                                                    )
                                                    .child(
                                                        MoonText::new(if self.disclosure_expanded {
                                                            "MoonDisclosure — interactive, expanded"
                                                        } else {
                                                            "MoonDisclosure — interactive, collapsed"
                                                        })
                                                        .uppercase(false)
                                                        .mono(true)
                                                        .color(p.text_soft)
                                                        .render(),
                                                    )
                                                    // The row owns this passive caret's click so
                                                    // pressing the glyph exercises event pass-through;
                                                    // an accidental caret hitbox would prevent that
                                                    // press from toggling the row.
                                                    .child(
                                                        h_flex()
                                                            .id("new-controls-disclosure-row")
                                                            .gap(px(6.0))
                                                            .items_center()
                                                            .cursor_pointer()
                                                            .on_click({
                                                                let view = view.clone();
                                                                move |_, _, app| {
                                                                    view.update(app, |this, cx| {
                                                                        this.disclosure_expanded =
                                                                            !this
                                                                                .disclosure_expanded;
                                                                        this.push_event(
                                                                            "MoonDisclosure passive row clicked".to_string(),
                                                                            cx,
                                                                        );
                                                                    });
                                                                }
                                                            })
                                                            .child(
                                                                MoonDisclosure::glyph(
                                                                    self.disclosure_expanded,
                                                                )
                                                                .box_size(14.0),
                                                            )
                                                            .child(
                                                                MoonDisclosure::glyph(
                                                                    self.disclosure_expanded,
                                                                )
                                                                .direction(
                                                                    MoonDisclosureDirection::DownUp,
                                                                )
                                                                .box_size(14.0),
                                                            )
                                                            .child(
                                                                MoonText::new("passive — click me")
                                                                    .uppercase(false)
                                                                    .mono(true)
                                                                    .color(p.text_soft)
                                                                    .render(),
                                                            ),
                                                    ),
                                            )
                                            .child(
                                                MoonPresetStrip::new(
                                                    "new-controls-preset-strip",
                                                )
                                                .slot_width(74.0)
                                                .items([
                                                    MoonPresetItem::new("TP", "F1", "+3.0%"),
                                                    MoonPresetItem::new("SL", "F2", "-2.0%")
                                                        .disabled(true),
                                                    MoonPresetItem::new("F3", "0.05", "size")
                                                        .selected(true),
                                                    MoonPresetItem::new("S3", "3", "+3.0%"),
                                                ])
                                                .render(),
                                            ),
                                    ),
                            ),
                    ),
            )
            .child(
                h_flex()
                    .items_start()
                    .gap(px(12.0))
                    .child(
                        card("Settings layout", cx)
                            .w(px(500.0))
                            .h(px(360.0))
                            .child(
                                MoonSettings::new("new-controls-settings")
                                    .sidebar_width(px(170.0))
                                    .page(
                                        MoonSettingPage::new("Trading")
                                            .description(
                                                "Searchable settings page with typed fields.",
                                            )
                                            .default_open(true)
                                            .group(
                                                MoonSettingGroup::new()
                                                    .title("Main")
                                                    .item(
                                                        MoonSettingItem::new(
                                                            "Enable hints",
                                                            {
                                                                let value =
                                                                    settings_enabled.clone();
                                                                let set_value =
                                                                    settings_enabled.clone();
                                                                MoonSettingField::switch(
                                                                    move |_| value.get(),
                                                                    move |next, app| {
                                                                        set_value.set(next);
                                                                        app.refresh_windows();
                                                                    },
                                                                )
                                                                .default_value(true)
                                                            },
                                                        )
                                                        .description("Switch field uses the same Moon-facing path."),
                                                    )
                                                    .item(
                                                        MoonSettingItem::new(
                                                            "Symbol",
                                                            {
                                                                let value =
                                                                    settings_symbol.clone();
                                                                let set_value =
                                                                    settings_symbol.clone();
                                                                MoonSettingField::input(
                                                                    move |_| value.borrow().clone(),
                                                                    move |next, app| {
                                                                        *set_value.borrow_mut() =
                                                                            next;
                                                                        app.refresh_windows();
                                                                    },
                                                                )
                                                                .default_value("BTCUSDT")
                                                            },
                                                        )
                                                        .description("Editable text field."),
                                                    )
                                                    .item(
                                                        MoonSettingItem::new(
                                                            "Mode",
                                                            {
                                                                let value = settings_mode.clone();
                                                                let set_value =
                                                                    settings_mode.clone();
                                                                MoonSettingField::dropdown(
                                                                    vec![
                                                                        (
                                                                            SharedString::from(
                                                                                "paper",
                                                                            ),
                                                                            SharedString::from(
                                                                                "Paper",
                                                                            ),
                                                                        ),
                                                                        (
                                                                            SharedString::from(
                                                                                "live",
                                                                            ),
                                                                            SharedString::from(
                                                                                "Live",
                                                                            ),
                                                                        ),
                                                                        (
                                                                            SharedString::from(
                                                                                "review",
                                                                            ),
                                                                            SharedString::from(
                                                                                "Review",
                                                                            ),
                                                                        ),
                                                                    ],
                                                                    move |_| value.borrow().clone(),
                                                                    move |next, app| {
                                                                        *set_value.borrow_mut() =
                                                                            next;
                                                                        app.refresh_windows();
                                                                    },
                                                                )
                                                                .default_value("paper")
                                                            },
                                                        )
                                                        .description("Dropdown field keeps menu behavior."),
                                                    )
                                                    .item(
                                                        MoonSettingItem::new(
                                                            "Risk",
                                                            {
                                                                let value = settings_risk.clone();
                                                                let set_value =
                                                                    settings_risk.clone();
                                                                MoonSettingField::number_input(
                                                                    MoonNumberFieldOptions {
                                                                        min: 0.0,
                                                                        max: 10.0,
                                                                        step: 0.5,
                                                                    },
                                                                    move |_| value.get(),
                                                                    move |next, app| {
                                                                        set_value.set(next);
                                                                        app.refresh_windows();
                                                                    },
                                                                )
                                                                .default_value(2.5)
                                                            },
                                                        )
                                                        .description("Number input field."),
                                                    ),
                                            ),
                                    ),
                            ),
                    )
                    .child(
                        card("Resizable panels", cx)
                            .w(px(500.0))
                            .h(px(360.0))
                            .child({
                                let resizable: MoonResizablePanelGroup =
                                    moon_h_resizable("new-controls-resizable")
                                        .child(
                                            moon_resizable_panel()
                                                .size(px(155.0))
                                                .size_range(px(120.0)..px(230.0))
                                                .flex_none()
                                                .child(
                                                    MoonSurface::new()
                                                        .id("new-controls-resizable-left")
                                                        .variant(MoonSurfaceVariant::Sidebar)
                                                        .child(
                                                            v_flex()
                                                                .size_full()
                                                                .p(px(12.0))
                                                                .gap(px(8.0))
                                                                .child(
                                                                    MoonBadge::new("left")
                                                                        .tone(MoonTone::Info)
                                                                        .render(),
                                                                )
                                                                .child(
                                                                    MoonText::new(
                                                                        "Drag the divider.",
                                                                    )
                                                                    .uppercase(false)
                                                                    .mono(true)
                                                                    .wrap()
                                                                    .color(p.text_soft)
                                                                    .render(),
                                                                ),
                                                        ),
                                                ),
                                        )
                                        .child(
                                            moon_resizable_panel().child(
                                                MoonSurface::new()
                                                    .id("new-controls-resizable-right")
                                                    .variant(MoonSurfaceVariant::Card)
                                                    .child(
                                                        v_flex()
                                                            .size_full()
                                                            .p(px(12.0))
                                                            .gap(px(8.0))
                                                            .child(
                                                                MoonBadge::new("content")
                                                                    .tone(MoonTone::Positive)
                                                                    .render(),
                                                            )
                                                            .child(
                                                                MoonText::new(
                                                                    "This is the real Longbridge resizable engine, exposed as MoonResizablePanelGroup.",
                                                                )
                                                                .uppercase(false)
                                                                .mono(true)
                                                                .wrap()
                                                                .color(p.text_soft)
                                                                .render(),
                                                            ),
                                                    ),
                                            ),
                                        );
                                resizable
                            }),
                    ),
            )
            .child(
                h_flex()
                    .items_start()
                    .gap(px(12.0))
                    .child(
                        card("Identity / navigation", cx)
                            .w(px(500.0))
                            .child(
                                h_flex()
                                    .gap(px(14.0))
                                    .items_center()
                                    .child(
                                        MoonAvatarGroup::new()
                                            .size(MoonAvatarSize::Normal)
                                            .limit(3)
                                            .ellipsis(true)
                                            .children([
                                                MoonAvatar::new().name("Moon Operator"),
                                                MoonAvatar::new().name("Risk Desk"),
                                                MoonAvatar::new().name("Quant Lab"),
                                                MoonAvatar::new().name("Ops"),
                                            ])
                                            .render(),
                                    )
                                    .child(MoonProgressCircle::new("new-controls-progress-circle")
                                        .value(72.0)
                                        .tone(MoonTone::Positive)
                                        .size(MoonProgressCircleSize::Large)
                                        .render()),
                            )
                            .child(
                                MoonBreadcrumb::new()
                                    .child(
                                        MoonBreadcrumbItem::new("MoonUI").on_click({
                                            let view = view.clone();
                                            move |_, _, app| {
                                                view.update(app, |this, cx| {
                                                    this.push_event("MoonBreadcrumb: MoonUI", cx);
                                                });
                                            }
                                        }),
                                    )
                                    .child("Components")
                                    .child("NewControls")
                                    .render(),
                            )
                            .child(
                                MoonPagination::new("new-controls-pagination")
                                    .current_page(self.new_pagination_page)
                                    .total_pages(12)
                                    .visible_pages(7)
                                    .small()
                                    .on_click({
                                        let view = view.clone();
                                        move |page, _, app| {
                                            let page = *page;
                                            view.update(app, |this, cx| {
                                                this.new_pagination_page = page;
                                                this.push_event(
                                                    format!("MoonPagination: page {page}"),
                                                    cx,
                                                );
                                            });
                                        }
                                    })
                                    .render(),
                            ),
                    )
                    .child(
                        card("Description data", cx).w(px(500.0)).child(
                            MoonDescriptionList::new()
                                .columns(2)
                                .small()
                                .item("Component class", "MoonReady", 1)
                                .item("Behavior", "Longbridge or MoonCustom", 1)
                                .item("Theme", "MoonTheme tokens", 1)
                                .item("Snapshot", "covered", 1)
                                .render(),
                        ),
                    ),
            )
            .child(
                h_flex()
                    .items_start()
                    .gap(px(12.0))
                    .child(
                        card("Search / date controls", cx)
                            .w(px(500.0))
                            .child(
                                MoonCombobox::new(&self.combobox_state)
                                    .placeholder("Select market")
                                    .search_placeholder("Search symbol")
                                    .cleanable(true)
                                    .menu_width(px(230.0))
                                    .menu_max_h(px(190.0)),
                            )
                            .child(
                                MoonDatePicker::new(&self.date_picker_state)
                                    .placeholder("Pick session date")
                                    .cleanable(true)
                                    .number_of_months(1),
                            )
                            .child(
                                MoonDateTimePicker::new(
                                    "new-controls-date-time-picker",
                                    &self.date_time_picker_state,
                                )
                                .placeholder("Pick session date and time")
                                .cleanable(true)
                                .number_of_months(1)
                                .render(),
                            )
                            .child(
                                MoonTimePicker::new(
                                    "new-controls-time-picker",
                                    &self.time_picker_state,
                                )
                                .render(),
                            )
                            .child(
                                MoonHoverCard::new("new-controls-hover-card")
                                    .open_delay(Duration::from_millis(120))
                                    .close_delay(Duration::from_millis(120))
                                    .trigger(
                                        MoonButton::new("new-controls-hover-trigger")
                                            .label("Hover details")
                                            .variant(MoonButtonVariant::Panel)
                                            .render(),
                                    )
                                    .content(|_, _, app| {
                                        let p = MoonPalette::active(app);
                                        v_flex()
                                            .gap(px(6.0))
                                            .w(px(230.0))
                                            .child(
                                                MoonText::new("MoonHoverCard")
                                                    .uppercase(false)
                                                    .mono(true)
                                                    .weight(700.0)
                                                    .color(p.amber)
                                                    .render(),
                                            )
                                            .child(
                                                MoonText::new(
                                                    "Hover lifecycle stays in the Longbridge engine; the surface uses Moon tokens.",
                                                )
                                                .uppercase(false)
                                                .mono(true)
                                                .wrap()
                                                .color(p.text_soft)
                                                .render(),
                                            )
                                    }),
                            ),
                    )
                    .child(
                        card("Calendar", cx).w(px(500.0)).child(
                            MoonCalendar::new(&self.calendar_state)
                                .number_of_months(1)
                                .w(px(292.0)),
                        ),
                    ),
            )
            .child(
                h_flex()
                    .items_start()
                    .gap(px(12.0))
                    .child(
                        card("MoonList", cx)
                            .w(px(500.0))
                            .h(px(280.0))
                            .child(
                                MoonList::new(&self.list_state)
                                    .search_placeholder("Filter list")
                                    .scrollbar_visible(true),
                            ),
                    )
                    .child(
                        card("MoonTree", cx)
                            .w(px(500.0))
                            .h(px(280.0))
                            .child(MoonTree::new(
                                &self.tree_state,
                                |ix, entry, selected, _window, app| {
                                    let p = MoonPalette::active(app);
                                    let marker = if entry.is_folder() {
                                        if entry.is_expanded() { "v" } else { ">" }
                                    } else {
                                        "-"
                                    };
                                    MoonListItem::new(ix)
                                        .selected(selected)
                                        .child(
                                            h_flex()
                                                .pl(px(12.0 * entry.depth() as f32))
                                                .gap(px(6.0))
                                                .child(
                                                    MoonText::new(marker)
                                                        .uppercase(false)
                                                        .mono(true)
                                                        .color(p.text_muted)
                                                        .render(),
                                                )
                                                .child(
                                                    MoonText::new(entry.item().label().clone())
                                                        .uppercase(false)
                                                        .mono(true)
                                                        .color(if selected {
                                                            p.text
                                                        } else {
                                                            p.text_soft
                                                        })
                                                        .render(),
                                                ),
                                        )
                                },
                            )),
                    ),
            )
            .child(
                h_flex().items_start().gap(px(12.0)).child(
                    card("MoonTree controlled/headless", cx)
                        .w(px(500.0))
                        .h(px(280.0))
                        .child(MoonTree::custom(
                            &self.controlled_tree_state,
                            |entry, meta, _window, app| {
                                let p = MoonPalette::active(app);
                                let marker = if entry.is_folder() {
                                    if entry.is_expanded() { "v" } else { ">" }
                                } else {
                                    "-"
                                };
                                let tone = if meta.selected { p.amber } else { p.text_soft };
                                h_flex()
                                    .id(SharedString::from(format!(
                                        "controlled-tree-row-{}",
                                        entry.item().id()
                                    )))
                                    .h(px(24.0))
                                    .w_full()
                                    .items_center()
                                    .gap(px(6.0))
                                    .pl(px(10.0 + 14.0 * entry.depth() as f32))
                                    .pr(px(8.0))
                                    .rounded(px(4.0))
                                    .border_1()
                                    .border_color(rgba_from(
                                        if meta.selected { p.amber } else { p.border },
                                        if meta.selected { 0.58 } else { 0.20 },
                                    ))
                                    .bg(rgba_from(
                                        if meta.selected { p.amber } else { p.panel },
                                        if meta.selected { 0.15 } else { 0.34 },
                                    ))
                                    .on_mouse_down(MouseButton::Left, |_event, _window, app| {
                                        app.stop_propagation();
                                    })
                                    .child(
                                        MoonText::new(marker)
                                            .mono(true)
                                            .uppercase(false)
                                            .color(p.text_muted)
                                            .render(),
                                    )
                                    .child(
                                        MoonCheckbox::new(SharedString::from(format!(
                                            "controlled-tree-check-{}",
                                            entry.item().id()
                                        )))
                                        .checked(meta.selected)
                                        .size(MoonCheckboxSize::Compact),
                                    )
                                    .child(
                                        div().flex_1().min_w_0().truncate().child(
                                            MoonText::new(entry.item().label().clone())
                                                .mono(true)
                                                .uppercase(false)
                                                .color(tone)
                                                .render(),
                                        ),
                                    )
                                    .child(
                                        MoonBadge::new(if entry.is_folder() {
                                            "folder"
                                        } else {
                                            "strategy"
                                        })
                                        .size(MoonBadgeSize::Tiny),
                                    )
                            },
                        )),
                ),
            )
            .child(
                h_flex()
                    .items_start()
                    .gap(px(12.0))
                    .child(
                        card("MoonSidebar", cx)
                            .w(px(500.0))
                            .child(
                                h_flex()
                                    .items_start()
                                    .gap(px(10.0))
                                    .child(
                                        MoonSidebar::new("new-controls-sidebar")
                                            .w(px(268.0))
                                            .h(px(250.0))
                                            .collapsed(self.new_sidebar_collapsed)
                                            .header(
                                                h_flex()
                                                    .gap(px(8.0))
                                                    .child(MoonBadge::new("UI").render())
                                                    .child("MoonSidebar"),
                                            )
                                            .child(
                                                MoonSidebarGroup::new("Navigation").child(
                                                    MoonSidebarMenu::new().children([
                                                        MoonSidebarMenuItem::new("Controls")
                                                            .active(true),
                                                        MoonSidebarMenuItem::new("Inputs"),
                                                        MoonSidebarMenuItem::new("Overlays")
                                                            .children([
                                                                MoonSidebarMenuItem::new("Dialog"),
                                                                MoonSidebarMenuItem::new("Sheet"),
                                                            ])
                                                            .default_open(true),
                                                    ]),
                                                ),
                                            ),
                                    )
                                    .child(
                                        v_flex()
                                            .gap(px(8.0))
                                            .child(
                                                MoonSidebarToggleButton::new()
                                                    .collapsed(self.new_sidebar_collapsed)
                                                    .on_click({
                                                        let view = view.clone();
                                                        move |_, _, app| {
                                                            view.update(app, |this, cx| {
                                                                this.new_sidebar_collapsed =
                                                                    !this.new_sidebar_collapsed;
                                                                this.push_event(
                                                                    format!(
                                                                        "MoonSidebar collapsed: {}",
                                                                        this.new_sidebar_collapsed
                                                                    ),
                                                                    cx,
                                                                );
                                                            });
                                                        }
                                                    }),
                                            )
                                            .child(
                                                MoonText::new(
                                                    "Collapse state, hierarchy and menu behavior stay in the sidebar engine.",
                                                )
                                                .uppercase(false)
                                                .mono(true)
                                                .wrap()
                                                .color(p.text_soft)
                                                .render(),
                                            ),
                                    ),
                            ),
                    )
                    .child(
                        card("MoonSheet", cx)
                            .w(px(500.0))
                            .child(
                                MoonButton::new("new-controls-sheet")
                                    .label("Open root-owned sheet")
                                    .variant(MoonButtonVariant::Panel)
                                    .on_click(|_, window, app| {
                                        window.open_moon_sheet_at(
                                            MoonPlacement::Right,
                                            app,
                                            |sheet, _window, cx| {
                                                let p = MoonPalette::active(cx);
                                                sheet
                                                    .title(div().child("MoonSheet"))
                                                    .size(px(360.0))
                                                    .child(
                                                        v_flex()
                                                            .gap(px(10.0))
                                                            .child(
                                                                MoonBadge::new("root overlay")
                                                                    .tone(MoonTone::Info)
                                                                    .variant(
                                                                        MoonBadgeVariant::Outline,
                                                                    )
                                                                    .render(),
                                                            )
                                                            .child(
                                                                MoonText::new(
                                                                    "Sheet is opened through MoonWindowExt and Root ownership, not as a local panel fake.",
                                                                )
                                                                .uppercase(false)
                                                                .mono(true)
                                                                .wrap()
                                                                .color(p.text_soft)
                                                                .render(),
                                                            ),
                                                    )
                                            },
                                        );
                                    })
                                    .render(),
                            )
                            .child(
                                MoonText::new(
                                    "The sheet button exercises the same root-owned overlay path application windows should use.",
                                )
                                .uppercase(false)
                                .mono(true)
                                .wrap()
                                .color(p.text_soft)
                                .render(),
                            ),
                    ),
            )
            .child(
                card("Rule", cx).child(
                    MoonText::new(
                        "Useful Longbridge controls still need real Moon styling before they appear here. Thin wrappers stay out of the gallery until the visual work is done.",
                    )
                    .uppercase(false)
                    .mono(true)
                    .wrap()
                    .color(p.text_soft)
                    .render(),
                ),
            )
    }

    fn render_composites(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = MoonPalette::active(cx);
        let settings_enabled = self.settings_enabled.clone();
        let settings_symbol = self.settings_symbol.clone();
        let settings_mode = self.settings_mode.clone();

        section("Composites / Ready Moon adaptations", cx)
            .child(
                card("Rule", cx)
                    .child(
                        MoonText::new(
                            "Composite controls are shown here only after they have a Moon-facing API and a Moon visual contract. This page exists so snapshot tests cover them without manual scrolling.",
                        )
                        .uppercase(false)
                        .mono(true)
                        .wrap()
                        .color(p.text_soft)
                        .render(),
                    )
                    .child(
                        h_flex()
                            .gap(px(8.0))
                            .flex_wrap()
                            .child(MoonBadge::new("MoonReady").tone(MoonTone::Positive).render())
                            .child(MoonBadge::new("Root-owned overlays").tone(MoonTone::Info).render())
                            .child(MoonBadge::new("Stateful controls").tone(MoonTone::Accent).render()),
                    ),
            )
            .child(
                h_flex()
                    .items_start()
                    .gap(px(12.0))
                    .child(
                        card("MoonSettings", cx)
                            .w(px(456.0))
                            .h(px(292.0))
                            .child(
                                MoonSettings::new("composites-settings")
                                    .sidebar_width(px(140.0))
                                    .page(
                                        MoonSettingPage::new("Trading")
                                            .description("Typed fields through MoonSettingField.")
                                            .default_open(true)
                                            .group(
                                                MoonSettingGroup::new()
                                                    .title("Main")
                                                    .item(
                                                        MoonSettingItem::new("Hints", {
                                                            let value = settings_enabled.clone();
                                                            let set_value = settings_enabled.clone();
                                                            MoonSettingField::switch(
                                                                move |_| value.get(),
                                                                move |next, app| {
                                                                    set_value.set(next);
                                                                    app.refresh_windows();
                                                                },
                                                            )
                                                            .default_value(true)
                                                        })
                                                        .description("Switch field."),
                                                    )
                                                    .item(
                                                        MoonSettingItem::new("Symbol", {
                                                            let value = settings_symbol.clone();
                                                            let set_value = settings_symbol.clone();
                                                            MoonSettingField::input(
                                                                move |_| value.borrow().clone(),
                                                                move |next, app| {
                                                                    *set_value.borrow_mut() = next;
                                                                    app.refresh_windows();
                                                                },
                                                            )
                                                            .default_value("BTCUSDT")
                                                        })
                                                        .description("Editable field."),
                                                    )
                                                    .item(
                                                        MoonSettingItem::new("Mode", {
                                                            let value = settings_mode.clone();
                                                            let set_value = settings_mode.clone();
                                                            MoonSettingField::dropdown(
                                                                vec![
                                                                    (
                                                                        SharedString::from("paper"),
                                                                        SharedString::from("Paper"),
                                                                    ),
                                                                    (
                                                                        SharedString::from("live"),
                                                                        SharedString::from("Live"),
                                                                    ),
                                                                ],
                                                                move |_| value.borrow().clone(),
                                                                move |next, app| {
                                                                    *set_value.borrow_mut() = next;
                                                                    app.refresh_windows();
                                                                },
                                                            )
                                                            .default_value("paper")
                                                        })
                                                        .description("Dropdown field."),
                                                    ),
                                            ),
                                    ),
                            ),
                    )
                    .child(
                        card("MoonResizablePanelGroup", cx)
                            .w(px(456.0))
                            .h(px(292.0))
                            .child({
                                let resizable: MoonResizablePanelGroup =
                                    moon_h_resizable("composites-resizable")
                                        .child(
                                            moon_resizable_panel()
                                                .size(px(148.0))
                                                .size_range(px(110.0)..px(220.0))
                                                .flex_none()
                                                .child(
                                                    MoonSurface::new()
                                                        .id("composites-resizable-left")
                                                        .variant(MoonSurfaceVariant::Sidebar)
                                                        .child(
                                                            v_flex()
                                                                .size_full()
                                                                .p(px(10.0))
                                                                .gap(px(8.0))
                                                                .child(
                                                                    MoonBadge::new("left")
                                                                        .tone(MoonTone::Info)
                                                                        .render(),
                                                                )
                                                                .child(
                                                                    MoonText::new("Drag divider.")
                                                                        .uppercase(false)
                                                                        .mono(true)
                                                                        .wrap()
                                                                        .color(p.text_soft)
                                                                        .render(),
                                                                ),
                                                        ),
                                                ),
                                        )
                                        .child(
                                            moon_resizable_panel().child(
                                                MoonSurface::new()
                                                    .id("composites-resizable-right")
                                                    .variant(MoonSurfaceVariant::Card)
                                                    .child(
                                                        v_flex()
                                                            .size_full()
                                                            .p(px(10.0))
                                                            .gap(px(8.0))
                                                            .child(
                                                                MoonBadge::new("content")
                                                                    .tone(MoonTone::Positive)
                                                                    .render(),
                                                            )
                                                            .child(
                                                                MoonText::new(
                                                                    "Longbridge resize behavior, Moon surfaces.",
                                                                )
                                                                .uppercase(false)
                                                                .mono(true)
                                                                .wrap()
                                                                .color(p.text_soft)
                                                                .render(),
                                                            ),
                                                    ),
                                            ),
                                        );
                                resizable
                            }),
                    ),
            )
            .child(
                h_flex()
                    .items_start()
                    .gap(px(12.0))
                    .child(
                        card("Search / date / hover", cx)
                            .w(px(456.0))
                            .h(px(270.0))
                            .child(
                                MoonCombobox::new(&self.combobox_state)
                                    .placeholder("Select market")
                                    .search_placeholder("Search symbol")
                                    .cleanable(true)
                                    .menu_width(px(230.0))
                                    .menu_max_h(px(170.0)),
                            )
                            .child(
                                MoonDatePicker::new(&self.date_picker_state)
                                    .placeholder("Pick session date")
                                    .cleanable(true)
                                    .number_of_months(1),
                            )
                            .child(
                                MoonHoverCard::new("composites-hover-card")
                                    .open_delay(Duration::from_millis(120))
                                    .close_delay(Duration::from_millis(120))
                                    .trigger(
                                        MoonButton::new("composites-hover-trigger")
                                            .label("Hover details")
                                            .variant(MoonButtonVariant::Panel)
                                            .render(),
                                    )
                                    .content(|_, _, app| {
                                        let p = MoonPalette::active(app);
                                        v_flex()
                                            .gap(px(6.0))
                                            .w(px(230.0))
                                            .child(
                                                MoonText::new("MoonHoverCard")
                                                    .uppercase(false)
                                                    .mono(true)
                                                    .weight(700.0)
                                                    .color(p.amber)
                                                    .render(),
                                            )
                                            .child(
                                                MoonText::new(
                                                    "Hover lifecycle stays in the component engine.",
                                                )
                                                .uppercase(false)
                                                .mono(true)
                                                .wrap()
                                                .color(p.text_soft)
                                                .render(),
                                            )
                                    }),
                            ),
                    )
                    .child(
                        card("Calendar / list", cx)
                            .w(px(456.0))
                            .h(px(270.0))
                            .child(
                                h_flex()
                                    .items_start()
                                    .gap(px(10.0))
                                    .child(
                                        MoonCalendar::new(&self.calendar_state)
                                            .number_of_months(1)
                                            .w(px(220.0)),
                                    )
                                    .child(
                                        div()
                                            .flex_1()
                                            .h(px(218.0))
                                            .child(
                                                MoonList::new(&self.list_state)
                                                    .search_placeholder("Filter")
                                                    .scrollbar_visible(true),
                                            ),
                                    ),
                            ),
                    ),
            )
    }

    fn render_stateful(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = MoonPalette::active(cx);
        let view = cx.entity();

        section("Stateful / Ready Moon adaptations", cx)
            .child(
                card("Rule", cx)
                    .child(
                        MoonText::new(
                            "Stateful controls must prove keyboard, expansion, collapse and root-overlay ownership as live widgets, not as static screenshots.",
                        )
                        .uppercase(false)
                        .mono(true)
                        .wrap()
                        .color(p.text_soft)
                        .render(),
                    )
                    .child(
                        h_flex()
                            .gap(px(8.0))
                            .flex_wrap()
                            .child(MoonBadge::new("Tree").tone(MoonTone::Info).render())
                            .child(MoonBadge::new("Sidebar").tone(MoonTone::Accent).render())
                            .child(
                                MoonBadge::new("Root overlay")
                                    .tone(MoonTone::Positive)
                                    .render(),
                            ),
                    ),
            )
            .child(
                h_flex()
                    .items_start()
                    .gap(px(12.0))
                    .child(
                        card("Tree / sidebar", cx)
                            .w(px(456.0))
                            .h(px(430.0))
                            .child(
                                h_flex()
                                    .items_start()
                                    .gap(px(10.0))
                                    .child(
                                        div().w(px(200.0)).h(px(378.0)).child(MoonTree::new(
                                            &self.tree_state,
                                            |ix, entry, selected, _window, app| {
                                                let p = MoonPalette::active(app);
                                                let marker = if entry.is_folder() {
                                                    if entry.is_expanded() { "v" } else { ">" }
                                                } else {
                                                    "-"
                                                };
                                                MoonListItem::new(ix).selected(selected).child(
                                                    h_flex()
                                                        .pl(px(10.0 * entry.depth() as f32))
                                                        .gap(px(6.0))
                                                        .child(
                                                            MoonText::new(marker)
                                                                .uppercase(false)
                                                                .mono(true)
                                                                .color(p.text_muted)
                                                                .render(),
                                                        )
                                                        .child(
                                                            MoonText::new(entry.item().label().clone())
                                                                .uppercase(false)
                                                                .mono(true)
                                                                .color(if selected {
                                                                    p.text
                                                                } else {
                                                                    p.text_soft
                                                                })
                                                                .render(),
                                                        ),
                                                )
                                            },
                                        )),
                                    )
                                    .child(
                                        v_flex()
                                            .gap(px(8.0))
                                            .child(
                                                MoonSidebarToggleButton::new()
                                                    .collapsed(self.new_sidebar_collapsed)
                                                    .on_click({
                                                        let view = view.clone();
                                                        move |_, _, app| {
                                                            view.update(app, |this, cx| {
                                                                this.new_sidebar_collapsed =
                                                                    !this.new_sidebar_collapsed;
                                                                this.push_event(
                                                                    format!(
                                                                        "MoonSidebar collapsed: {}",
                                                                        this.new_sidebar_collapsed
                                                                    ),
                                                                    cx,
                                                                );
                                                            });
                                                        }
                                                    }),
                                            )
                                            .child(
                                                MoonSidebar::new("stateful-sidebar")
                                                    .w(px(220.0))
                                                    .h(px(336.0))
                                                    .collapsed(self.new_sidebar_collapsed)
                                                    .header(
                                                        h_flex()
                                                            .gap(px(8.0))
                                                            .child("MoonSidebar"),
                                                    )
                                                    .child(
                                                        MoonSidebarGroup::new("Navigation").child(
                                                            MoonSidebarMenu::new().children([
                                                                MoonSidebarMenuItem::new("Controls")
                                                                    .active(true),
                                                                MoonSidebarMenuItem::new("Inputs"),
                                                                MoonSidebarMenuItem::new("Overlays")
                                                                    .children([
                                                                        MoonSidebarMenuItem::new(
                                                                            "Dialog",
                                                                        ),
                                                                        MoonSidebarMenuItem::new(
                                                                            "Sheet",
                                                                        ),
                                                                    ])
                                                                    .default_open(true),
                                                            ]),
                                                        ),
                                                    ),
                                            ),
                                    ),
                            ),
                    )
                    .child(
                        card("Root-owned sheet", cx)
                            .w(px(456.0))
                            .h(px(430.0))
                            .child(
                                MoonButton::new("stateful-sheet")
                                    .label("Open MoonSheet")
                                    .variant(MoonButtonVariant::Panel)
                                    .on_click(|_, window, app| {
                                        window.open_moon_sheet_at(
                                            MoonPlacement::Right,
                                            app,
                                            |sheet, _window, cx| {
                                                let p = MoonPalette::active(cx);
                                                sheet
                                                    .title(div().child("MoonSheet"))
                                                    .size(px(360.0))
                                                    .child(
                                                        v_flex()
                                                            .gap(px(10.0))
                                                            .child(
                                                                MoonBadge::new("root overlay")
                                                                    .tone(MoonTone::Info)
                                                                    .variant(
                                                                        MoonBadgeVariant::Outline,
                                                                    )
                                                                    .render(),
                                                            )
                                                            .child(
                                                                MoonText::new(
                                                                    "Sheet is opened through MoonWindowExt and Root ownership.",
                                                                )
                                                                .uppercase(false)
                                                                .mono(true)
                                                                .wrap()
                                                                .color(p.text_soft)
                                                                .render(),
                                                            ),
                                                    )
                                            },
                                        );
                                    })
                                    .render(),
                            )
                            .child(
                                MoonDescriptionList::new()
                                    .columns(1)
                                    .small()
                                    .item("Owner", "MoonRoot", 1)
                                    .item("API", "MoonWindowExt", 1)
                                    .item("Policy", "no local overlay fake", 2)
                                    .item("Behavior", "root layer", 2)
                                    .render(),
                            )
                            .child(
                                MoonText::new(
                                    "The button exercises the same root-owned sheet path application windows should use. It is intentionally not drawn as a panel child overlay.",
                                )
                                .uppercase(false)
                                .mono(true)
                                .wrap()
                                .color(p.text_soft)
                                .render(),
                            ),
                    ),
            )
    }
}

impl Render for Gallery {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = MoonPalette::active(cx);
        self.schedule_snapshot_capture(window, cx);
        for panel_name in std::mem::take(&mut self.pending_detach) {
            self.dock.update(cx, |dock, cx| {
                dock.remove_panel_by_name(panel_name.as_ref(), window, cx);
            });
            self.event_log.insert(
                0,
                SharedString::from(format!("Detached window: {panel_name}")),
            );
            self.event_log.truncate(10);
            cx.defer(move |cx| open_detached_gallery_panel(panel_name.clone(), cx));
        }

        let page = match self.active_page {
            0 => self.render_controls(cx).into_any_element(),
            1 => self.render_inputs(cx).into_any_element(),
            2 => self.render_tables(cx).into_any_element(),
            3 => self.render_menus(cx).into_any_element(),
            4 => self.render_navigation(cx).into_any_element(),
            5 => self.render_new_controls(cx).into_any_element(),
            6 => self.render_composites(cx).into_any_element(),
            _ => self.render_stateful(cx).into_any_element(),
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

struct DetachedGalleryPanel {
    title: SharedString,
}

impl Render for DetachedGalleryPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = MoonPalette::active(cx);
        let frame = MoonWindowFrame::tool("gallery-detached-frame", 0.0)
            .brand(MoonWindowFrameBrand::Mark)
            .controls(MoonWindowFrameControls::MinimizeClose);
        v_flex()
            .size_full()
            .bg(rgba_from(p.shell, 1.0))
            .text_color(rgb(p.text))
            .child(
                h_flex()
                    .h(px(42.0))
                    .px(px(12.0))
                    .border_b_1()
                    .border_color(rgba_from(p.border, 1.0))
                    .bg(rgba_from(p.shell_high, 1.0))
                    .child(frame.title_cluster(format!("Dock / {}", self.title), cx))
                    .child(div().flex_1())
                    .child(frame.visual_controls(cx)),
            )
            .child(
                v_flex()
                    .flex_1()
                    .p(px(16.0))
                    .gap(px(10.0))
                    .child(
                        MoonBadge::new("detached dock panel")
                            .tone(MoonTone::Info)
                            .variant(MoonBadgeVariant::Outline)
                            .render(),
                    )
                    .child(
                        MoonText::new(format!(
                            "{} opened from DockEvent::DetachRequested.",
                            self.title
                        ))
                        .uppercase(false)
                        .mono(true)
                        .wrap()
                        .color(p.text_soft)
                        .render(),
                    ),
            )
    }
}

fn open_detached_gallery_panel(panel_name: SharedString, cx: &mut App) {
    let p = MoonPalette::active(cx);
    let bounds = Bounds::centered(None, size(px(520.0), px(340.0)), cx);
    let title = panel_name.clone();
    if let Err(err) = cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            window_clear_color: Some(rgba((p.shell << 8) | 0xFF)),
            app_id: Some(format!("pro.moonbot.moon-ui-gallery.detached.{panel_name}")),
            ..Default::default()
        },
        move |window, cx| {
            let view = cx.new(|_| DetachedGalleryPanel {
                title: title.clone(),
            });
            cx.new(|cx| {
                Root::new(view, window, cx)
                    .background_policy(MoonBackgroundPolicy::Opaque)
                    .background(MoonPalette::active(cx).shell)
            })
        },
    ) {
        eprintln!("failed to open detached gallery panel {panel_name}: {err}");
    }
}

#[cfg(test)]
mod tests;
