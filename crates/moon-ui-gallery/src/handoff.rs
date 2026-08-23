//! Deterministic design-handoff cases and their snapshot capture flow.

use super::*;

mod cases;

use cases::HANDOFF_CASES;
pub(super) use cases::HandoffCase;

fn selected_handoff_case_indices(case_ids: &[String]) -> Vec<usize> {
    if case_ids.is_empty() {
        return (0..HANDOFF_CASES.len()).collect();
    }

    let mut indices = Vec::new();
    for case_id in case_ids {
        if let Some(ix) = HANDOFF_CASES.iter().position(|case| case.id == case_id) {
            indices.push(ix);
        } else {
            eprintln!("unknown handoff case ignored: {case_id}");
        }
    }
    if indices.is_empty() {
        eprintln!("no requested handoff cases found; falling back to the full set");
        (0..HANDOFF_CASES.len()).collect()
    } else {
        indices
    }
}

pub(super) fn first_handoff_case_for_ids(case_ids: &[String]) -> HandoffCase {
    let indices = selected_handoff_case_indices(case_ids);
    HANDOFF_CASES
        .get(indices[0])
        .copied()
        .unwrap_or(HandoffCase {
            id: "empty",
            width: 320.0,
            height: 180.0,
        })
}

#[cfg_attr(not(feature = "snapshot"), allow(dead_code))]
struct CaseSnapshotRun {
    dir: PathBuf,
    case_indices: Vec<usize>,
    case_ix: usize,
    capture_scheduled: bool,
    settle_frames: usize,
    next_capture_at: Instant,
    cleaned_dir: bool,
    targeted: bool,
}

/// Owns state for the deterministic design-handoff case renderer.
pub(super) struct CaseGallery {
    snapshot: Option<CaseSnapshotRun>,
    #[cfg(feature = "snapshot")]
    theme_mode: ThemeMode,
    #[cfg(feature = "snapshot")]
    dialog_case_open: bool,
    slider_58_state: Entity<MoonSliderState>,
    slider_100_state: Entity<MoonSliderState>,
    range_slider_state: Entity<MoonSliderState>,
    select_state: Entity<MoonSelectState<SharedString>>,
    combobox_state: Entity<MoonComboboxState<MoonSearchableVec<&'static str>>>,
    date_picker_state: Entity<MoonDatePickerState>,
    date_time_picker_state: Entity<MoonDateTimePickerState>,
    open_date_time_picker_state: Entity<MoonDateTimePickerState>,
    time_picker_state: Entity<MoonTimePickerState>,
    open_time_picker_state: Entity<MoonTimePickerState>,
    calendar_state: Entity<MoonCalendarState>,
    list_state: Entity<MoonListState<GalleryListDelegate>>,
    tree_state: Entity<MoonTreeState>,
    color_state: Entity<MoonColorPickerState>,
    data_table_state: Entity<MoonDataTableState>,
    virtual_scroll: MoonVirtualListScrollHandle,
    tooltip_view: Entity<MoonTooltipView>,
    dock: Entity<DockArea>,
}

impl CaseGallery {
    /// Builds the handoff renderer and the shared component states used by its cases.
    pub(super) fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        snapshot_dir: Option<PathBuf>,
        snapshot_case_ids: Vec<String>,
        theme_mode: ThemeMode,
    ) -> Self {
        #[cfg(not(feature = "snapshot"))]
        let _ = theme_mode;

        let first_case = HANDOFF_CASES.first().copied().unwrap_or(HandoffCase {
            id: "empty",
            width: 320.0,
            height: 180.0,
        });
        window.resize(size(px(first_case.width), px(first_case.height)));

        let date_time_picker_state = cx.new(|cx| MoonDateTimePickerState::new(window, cx));
        // A specimen of the popup itself: the calendar and the time drums of one control.
        let open_date_time_picker_state = cx.new(|cx| {
            let mut state = MoonDateTimePickerState::new(window, cx);
            state.set_open(true, cx);
            state
        });
        let time_picker_state = cx.new(|cx| MoonTimePickerState::new(cx));
        let open_time_picker_state = cx.new(|cx| {
            let mut state = MoonTimePickerState::new(cx);
            state.set_open(true, cx);
            state
        });
        let slider_58_state = cx.new(|_| {
            MoonSliderState::new()
                .min(0.0)
                .max(100.0)
                .step(1.0)
                .default_value(58.0)
        });
        let slider_100_state = cx.new(|_| {
            MoonSliderState::new()
                .min(0.0)
                .max(100.0)
                .step(1.0)
                .default_value(100.0)
        });
        let range_slider_state = cx.new(|_| {
            MoonSliderState::new()
                .min(0.0)
                .max(100.0)
                .step(1.0)
                .default_value((18.0, 74.0))
        });
        let select_state = cx.new(|cx| {
            MoonSelectState::new(
                [
                    MoonSelectItem::new(SharedString::from("auto"), "Auto"),
                    MoonSelectItem::new(SharedString::from("50"), "50%"),
                    MoonSelectItem::new(SharedString::from("20"), "20%"),
                ],
                Some(IndexPath::new(0)),
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
        let color_state =
            cx.new(|cx| MoonColorPickerState::new(window, cx).default_value(rgb(0xFFB347).into()));
        let data_table_state = cx.new(|_| MoonDataTableState::new());
        let virtual_scroll = MoonVirtualListScrollHandle::new();
        let tooltip_view =
            cx.new(|_| MoonTooltipView::new("MoonTooltipView entity").max_width(220.0));
        let dock = cx.new(|cx| DockArea::new("handoff-case-dock", Some(1), window, cx));
        let dock_items = gallery_dock_panels();
        let dock_weak = dock.downgrade();
        dock.update(cx, |dock, cx| {
            dock.set_center(
                DockItem::tabs(dock_items, &dock_weak, window, cx),
                window,
                cx,
            );
        });

        let selected_case_indices = selected_handoff_case_indices(&snapshot_case_ids);
        let targeted = !snapshot_case_ids.is_empty();
        if snapshot_dir.is_some() {
            let first_case = HANDOFF_CASES[selected_case_indices[0]];
            window.resize(size(px(first_case.width), px(first_case.height)));
        }

        Self {
            snapshot: snapshot_dir.map(|dir| CaseSnapshotRun {
                dir,
                case_indices: selected_case_indices,
                case_ix: 0,
                capture_scheduled: false,
                settle_frames: 8,
                next_capture_at: Instant::now() + Duration::from_millis(500),
                cleaned_dir: false,
                targeted,
            }),
            #[cfg(feature = "snapshot")]
            theme_mode,
            #[cfg(feature = "snapshot")]
            dialog_case_open: false,
            slider_58_state,
            slider_100_state,
            range_slider_state,
            select_state,
            combobox_state,
            date_picker_state,
            date_time_picker_state,
            open_date_time_picker_state,
            time_picker_state,
            open_time_picker_state,
            calendar_state,
            list_state,
            tree_state,
            color_state,
            data_table_state,
            virtual_scroll,
            tooltip_view,
            dock,
        }
    }

    fn current_case(&self) -> HandoffCase {
        let ix = self.snapshot.as_ref().map_or(0, |snapshot| {
            snapshot
                .case_indices
                .get(snapshot.case_ix)
                .copied()
                .unwrap_or(0)
        });
        HANDOFF_CASES.get(ix).copied().unwrap_or(HandoffCase {
            id: "empty",
            width: 320.0,
            height: 180.0,
        })
    }

    fn schedule_case_snapshot_capture(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(snapshot) = self.snapshot.as_mut() else {
            return;
        };
        if snapshot.capture_scheduled {
            return;
        }
        snapshot.capture_scheduled = true;
        cx.on_next_frame(window, |this, window, cx| {
            this.capture_snapshot_case(window, cx);
        });
    }

    #[cfg(feature = "snapshot")]
    fn capture_snapshot_case(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let case = self.current_case();
        if !self.prepare_case_overlay(case, window, cx) {
            if let Some(snapshot) = self.snapshot.as_mut() {
                snapshot.capture_scheduled = false;
            }
            cx.notify();
            return;
        }

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

        let theme_dir = snapshot
            .dir
            .join(theme_mode_name(self.theme_mode).to_ascii_lowercase());
        if !snapshot.targeted && !snapshot.cleaned_dir {
            if let Err(err) = clear_snapshot_dir(&theme_dir) {
                eprintln!(
                    "failed to clear case snapshot dir {}: {err}",
                    theme_dir.display()
                );
                cx.quit();
                return;
            }
            snapshot.cleaned_dir = true;
        }
        if let Err(err) = std::fs::create_dir_all(&theme_dir) {
            eprintln!(
                "failed to create case snapshot dir {}: {err}",
                theme_dir.display()
            );
            cx.quit();
            return;
        }

        let path = theme_dir.join(format!("{}.png", case.id));
        if snapshot.targeted {
            let _ = std::fs::remove_file(&path);
        }
        let image = match snapshot_window_image(window) {
            Ok(image) => image,
            Err(err) => {
                eprintln!("case snapshot {} failed: {err}", case.id);
                cx.quit();
                return;
            }
        };
        if let Err(err) = image.save(&path) {
            eprintln!(
                "case snapshot {} failed to save {}: {err}",
                case.id,
                path.display()
            );
            cx.quit();
            return;
        }
        eprintln!("case snapshot {} -> {}", case.id, path.display());

        snapshot.case_ix += 1;
        if snapshot.case_ix >= snapshot.case_indices.len() {
            cx.quit();
            return;
        }
        let next_case = HANDOFF_CASES[snapshot.case_indices[snapshot.case_ix]];
        window.resize(size(px(next_case.width), px(next_case.height)));
        snapshot.capture_scheduled = false;
        snapshot.settle_frames = 8;
        snapshot.next_capture_at = Instant::now() + Duration::from_millis(700);
        cx.notify();
    }

    #[cfg(not(feature = "snapshot"))]
    fn capture_snapshot_case(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        eprintln!("moon-ui-gallery --snapshot-case-dir requires `--features snapshot`");
        cx.quit();
    }

    #[cfg(feature = "snapshot")]
    fn prepare_case_overlay(
        &mut self,
        case: HandoffCase,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if case.id == "dialog.confirm" {
            if !self.dialog_case_open {
                window.open_unique_moon_dialog("handoff-dialog-confirm", cx, |dialog, _, cx| {
                    let p = MoonPalette::active(cx);
                    dialog
                        .w(px(320.0))
                        .close_button(true)
                        .title(div().child("Confirm order"))
                        .content(move |content, _, _| {
                            content.child(
                                MoonText::new("Cancel pending BUY?")
                                    .uppercase(false)
                                    .mono(true)
                                    .color(p.text_soft)
                                    .render(),
                            )
                        })
                        .footer(
                            h_flex()
                                .justify_end()
                                .gap(px(8.0))
                                .child(
                                    MoonButton::new("handoff-dialog-cancel")
                                        .label("Cancel")
                                        .variant(MoonButtonVariant::Panel)
                                        .render(),
                                )
                                .child(
                                    MoonButton::new("handoff-dialog-confirm")
                                        .label("Confirm")
                                        .variant(MoonButtonVariant::Amber)
                                        .render(),
                                ),
                        )
                });
                self.dialog_case_open = true;
                return false;
            }
            true
        } else {
            if self.dialog_case_open {
                window.close_dialog(cx);
                self.dialog_case_open = false;
                return false;
            }
            true
        }
    }

    fn render_case_component(
        &self,
        case: HandoffCase,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let p = MoonPalette::active(cx);
        match case.id {
            "theme.palette" => v_flex()
                .gap(px(8.0))
                .child(swatch("shell", p.shell))
                .child(swatch("panel", p.panel))
                .child(swatch("amber", p.amber))
                .child(swatch("green", p.green))
                .into_any_element(),
            "root.background_policy" => h_flex()
                .w(px(360.0))
                .gap(px(10.0))
                .child(
                    MoonSurface::new()
                        .id("handoff-root-opaque")
                        .variant(MoonSurfaceVariant::Card)
                        .background_policy(MoonBackgroundPolicy::Opaque)
                        .child(
                            v_flex()
                                .p(px(10.0))
                                .gap(px(6.0))
                                .child(MoonBadge::new("Opaque").tone(MoonTone::Info).render())
                                .child(
                                    MoonText::new("Root/panel paints")
                                        .uppercase(false)
                                        .mono(true)
                                        .color(p.text_soft)
                                        .render(),
                                ),
                        ),
                )
                .child(
                    MoonSurface::new()
                        .id("handoff-root-nofill")
                        .variant(MoonSurfaceVariant::Card)
                        .background_policy(MoonBackgroundPolicy::NoFill)
                        .border_alpha(0.75)
                        .child(
                            v_flex()
                                .p(px(10.0))
                                .gap(px(6.0))
                                .child(MoonBadge::new("NoFill").tone(MoonTone::Warning).render())
                                .child(
                                    MoonText::new("Chart host no bg")
                                        .uppercase(false)
                                        .mono(true)
                                        .color(p.text_soft)
                                        .render(),
                                ),
                        ),
                )
                .into_any_element(),
            "app.main.three_charts_scroll" => handoff_chart_stack(cx).into_any_element(),
            "app.strategy_editor.selected" => handoff_strategy_editor(cx).into_any_element(),
            "icons.primitives" => h_flex()
                .gap(px(12.0))
                .child(
                    div()
                        .w(px(34.0))
                        .h(px(34.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(5.0))
                        .border_1()
                        .border_color(rgba_from(p.border, 1.0))
                        .bg(rgba_from(p.panel, 1.0))
                        .child(
                            svg()
                                .external_path(moon_ui::MOON_ICON_CHECK)
                                .w(px(16.0))
                                .h(px(16.0))
                                .text_color(rgb(p.blue)),
                        ),
                )
                .child(
                    div()
                        .w(px(34.0))
                        .h(px(34.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(5.0))
                        .border_1()
                        .border_color(rgba_from(p.border, 1.0))
                        .bg(rgba_from(p.panel, 1.0))
                        .child(
                            svg()
                                .external_path(moon_ui::MOON_ICON_CARET_DOWN)
                                .w(px(16.0))
                                .h(px(16.0))
                                .text_color(rgb(p.amber)),
                        ),
                )
                .child(
                    MoonText::new("Moon icon assets")
                        .uppercase(false)
                        .mono(true)
                        .color(p.text_soft)
                        .render(),
                )
                .into_any_element(),
            "avatar.group" => h_flex()
                .gap(px(14.0))
                .child(
                    MoonAvatar::new()
                        .name("Moon Operator")
                        .size(MoonAvatarSize::Large)
                        .render(),
                )
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
                .into_any_element(),
            "window.frame.main_full_logo" => window_frame_row(
                MoonWindowFrame::main("handoff-window-main", 0.0)
                    .brand(MoonWindowFrameBrand::Full)
                    .controls(MoonWindowFrameControls::MinimizeMaximizeClose),
                "main window",
                cx,
            )
            .into_any_element(),
            "window.frame.small_logo" => window_frame_row(
                MoonWindowFrame::tool("handoff-window-frame", 0.0)
                    .brand(MoonWindowFrameBrand::Mark)
                    .controls(MoonWindowFrameControls::MinimizeClose),
                "debug stats",
                cx,
            )
            .into_any_element(),
            "window.frame.popup_no_logo" => window_frame_row(
                MoonWindowFrame::popup("handoff-window-popup", 0.0)
                    .brand(MoonWindowFrameBrand::None)
                    .controls(MoonWindowFrameControls::Close),
                "popup window",
                cx,
            )
            .into_any_element(),
            "window.frame.detached_panel" => window_frame_row(
                MoonWindowFrame::detached_panel("handoff-window-detached-panel", 0.0)
                    .brand(MoonWindowFrameBrand::Mark)
                    .controls(MoonWindowFrameControls::MinimizeClose),
                "detached panel",
                cx,
            )
            .into_any_element(),
            "window.frame.detached_chart" => window_frame_row(
                MoonWindowFrame::detached_chart("handoff-window-detached-chart", 0.0)
                    .brand(MoonWindowFrameBrand::Mark)
                    .controls(MoonWindowFrameControls::MinimizeClose),
                "detached chart",
                cx,
            )
            .into_any_element(),
            "window.frame.debug" => window_frame_row(
                MoonWindowFrame::debug("handoff-window-debug", 0.0)
                    .brand(MoonWindowFrameBrand::Mark)
                    .controls(MoonWindowFrameControls::MinimizeClose),
                "debug window",
                cx,
            )
            .into_any_element(),
            "surface.card" => MoonSurface::new()
                .id("handoff-surface-card")
                .variant(MoonSurfaceVariant::Card)
                .child(
                    v_flex()
                        .w(px(260.0))
                        .p(px(12.0))
                        .gap(px(8.0))
                        .child(MoonBadge::new("MoonSurface").tone(MoonTone::Info).render())
                        .child(
                            MoonText::new(
                                "Card and sidebar surfaces own reusable background policy.",
                            )
                            .uppercase(false)
                            .mono(true)
                            .wrap()
                            .color(p.text_soft)
                            .render(),
                        ),
                )
                .into_any_element(),
            "surface.sidebar" => MoonSurface::new()
                .id("handoff-surface-sidebar")
                .variant(MoonSurfaceVariant::Sidebar)
                .child(
                    v_flex()
                        .w(px(260.0))
                        .h(px(110.0))
                        .p(px(12.0))
                        .gap(px(8.0))
                        .child(
                            MoonBadge::new("Sidebar")
                                .tone(MoonTone::Info)
                                .variant(MoonBadgeVariant::Outline)
                                .render(),
                        )
                        .child(
                            MoonText::new("Sidebar surface variant for left panes and settings.")
                                .uppercase(false)
                                .mono(true)
                                .wrap()
                                .color(p.text_soft)
                                .render(),
                        ),
                )
                .into_any_element(),
            "button.neutral" => MoonButton::new("handoff-button-neutral")
                .label("Neutral")
                .variant(MoonButtonVariant::Neutral)
                .render()
                .into_any_element(),
            "button.hover" => MoonButton::new("handoff-button-hover")
                .label("Hover")
                .variant(MoonButtonVariant::Panel)
                .selected(true)
                .render()
                .into_any_element(),
            "button.active" => MoonButton::new("handoff-button-active")
                .label("Active")
                .variant(MoonButtonVariant::Amber)
                .selected(true)
                .render()
                .into_any_element(),
            "button.disabled" => MoonButton::new("handoff-button-disabled")
                .label("Disabled")
                .disabled(true)
                .render()
                .into_any_element(),
            "button.blue" => MoonButton::new("handoff-button-blue")
                .label("Blue")
                .variant(MoonButtonVariant::Blue)
                .render()
                .into_any_element(),
            "button.green" => MoonButton::new("handoff-button-green")
                .label("Green")
                .variant(MoonButtonVariant::Green)
                .render()
                .into_any_element(),
            "button.danger" => MoonButton::new("handoff-button-danger")
                .label("Danger")
                .variant(MoonButtonVariant::Danger)
                .render()
                .into_any_element(),
            "button.outline_amber" => MoonButton::new("handoff-button-outline-amber")
                .label("Outline")
                .variant(MoonButtonVariant::OutlineAmber)
                .render()
                .into_any_element(),
            "button.micro" => MoonButton::new("handoff-button-micro")
                .label("Micro")
                .size(MoonButtonSize::Micro)
                .render()
                .into_any_element(),
            "button.action" => MoonButton::new("handoff-button-action")
                .label("Action")
                .size(MoonButtonSize::Action)
                .render()
                .into_any_element(),
            "button.pill" => MoonButton::new("handoff-button-pill")
                .label("Pill")
                .size(MoonButtonSize::Pill)
                .variant(MoonButtonVariant::Panel)
                .selected(true)
                .render()
                .into_any_element(),
            "button.icon_slots" => MoonButton::new("handoff-button-icons")
                .leading_icon(MoonButtonIconSlot::new(moon_ui::MOON_ICON_CHECK))
                .segment(MoonButtonSegment::new("F3").color(p.amber).weight(700.0))
                .segment(MoonButtonSegment::new("0.05").color(p.text))
                .trailing_icon(MoonButtonIconSlot::new(moon_ui::MOON_ICON_CARET_DOWN))
                .render()
                .into_any_element(),
            "button.variants_all" => h_flex()
                .gap(px(7.0))
                .child(
                    MoonButton::new("handoff-button-soft")
                        .label("Soft")
                        .variant(MoonButtonVariant::Soft)
                        .render(),
                )
                .child(
                    MoonButton::new("handoff-button-red")
                        .label("Red")
                        .variant(MoonButtonVariant::Red)
                        .render(),
                )
                .child(
                    MoonButton::new("handoff-button-outline-red")
                        .label("Outline Red")
                        .variant(MoonButtonVariant::OutlineRed)
                        .render(),
                )
                .child(
                    MoonButton::new("handoff-button-ghost")
                        .label("Ghost")
                        .variant(MoonButtonVariant::Ghost)
                        .render(),
                )
                .child(
                    MoonButton::new("handoff-button-bare")
                        .label("Bare")
                        .variant(MoonButtonVariant::Bare)
                        .render(),
                )
                .into_any_element(),
            "input.default" => div()
                .w(px(260.0))
                .child(
                    MoonInput::new("handoff-input-default")
                        .placeholder("StrategyName")
                        .default_value("HooksDetect 0.3-1%")
                        .small(),
                )
                .into_any_element(),
            "input.placeholder" => div()
                .w(px(260.0))
                .child(
                    MoonInput::new("handoff-input-placeholder")
                        .placeholder("StrategyName")
                        .small(),
                )
                .into_any_element(),
            "input.focus" => div()
                .w(px(260.0))
                .child(
                    MoonInput::new("handoff-input-focus")
                        .placeholder("StrategyName")
                        .default_value("BTCUSDT")
                        .selected(true)
                        .small(),
                )
                .into_any_element(),
            "input.mask" => v_flex()
                .w(px(330.0))
                .gap(px(8.0))
                .child(
                    MoonInput::new("handoff-input-mask")
                        .placeholder("AAA-999")
                        .default_value(MoonInputMaskPattern::new("AAA-999").mask("BOT123"))
                        .small(),
                )
                .child(
                    MoonText::new(format!(
                        "number: {}",
                        MoonInputMaskPattern::number_with_fraction(Some(' '), Some(2))
                            .mask("1234567.899")
                    ))
                    .uppercase(false)
                    .mono(true)
                    .color(p.text_soft)
                    .render(),
                )
                .into_any_element(),
            "input.hotkey" => v_flex()
                .w(px(420.0))
                .gap(px(9.0))
                .child(
                    MoonHotkeyInput::new("handoff-hotkey-primary")
                        .default_value(gpui::Keystroke::parse("ctrl-alt-k").ok())
                        .placeholder("Click to record shortcut")
                        .width(270.0),
                )
                .child(
                    MoonHotkeyInput::new("handoff-hotkey-recording")
                        .recording(true)
                        .recording_placeholder("Press shortcut...")
                        .width(270.0),
                )
                .child(
                    MoonHotkeyInput::new("handoff-hotkey-conflict")
                        .default_value(gpui::Keystroke::parse("ctrl-k").ok())
                        .conflict_label("used by Search")
                        .width(320.0),
                )
                .child(
                    MoonHotkeyInput::new("handoff-hotkey-disabled")
                        .default_value(gpui::Keystroke::parse("shift-f4").ok())
                        .disabled(true)
                        .width(230.0),
                )
                .into_any_element(),
            "select.toolbar" => div()
                .w(px(220.0))
                .child(
                    MoonSelect::new(&self.select_state)
                        .id("handoff-select-toolbar")
                        .title_prefix("Scale")
                        .trigger_variant(MoonButtonVariant::Panel)
                        .menu_width(160.0),
                )
                .into_any_element(),
            "combobox.symbol" => div()
                .w(px(280.0))
                .child(
                    MoonCombobox::new(&self.combobox_state)
                        .placeholder("Select market")
                        .search_placeholder("Search symbol")
                        .cleanable(true)
                        .menu_width(px(230.0))
                        .menu_max_h(px(190.0)),
                )
                .into_any_element(),
            "color_picker.trigger" => div()
                .w(px(190.0))
                .child(MoonColorPicker::new(&self.color_state).id("handoff-color-picker"))
                .into_any_element(),
            "textarea.memo" => div()
                .w(px(300.0))
                .child(
                    MoonTextArea::new("handoff-textarea-memo")
                        .placeholder("formula / memo")
                        .default_value("CustomEMA(source, fast)\n  and volume > avg(volume, 20)")
                        .formula(),
                )
                .into_any_element(),
            "form.row" => div()
                .w(px(360.0))
                .child(
                    MoonFormRow::new("handoff-form-row", "Risk")
                        .label_width(90.0)
                        .control(
                            MoonInput::new("handoff-form-row-input")
                                .default_value("2.50")
                                .small(),
                        ),
                )
                .into_any_element(),
            "stepper.normal" => MoonStepper::new("handoff-stepper")
                .value(3.0)
                .range(0.0, 10.0)
                .step(0.5)
                .precision(1)
                .tone(MoonTone::Warning)
                .render()
                .into_any_element(),
            "checkbox.checked" => MoonCheckbox::new("handoff-checkbox-checked")
                .label("Risk lock")
                .checked(true)
                .mono(true)
                .into_any_element(),
            "checkbox.unchecked" => MoonCheckbox::new("handoff-checkbox-unchecked")
                .label("Risk lock")
                .checked(false)
                .mono(true)
                .into_any_element(),
            "checkbox.compact" => MoonCheckbox::new("handoff-checkbox-compact")
                .label("compact")
                .checked(true)
                .size(MoonCheckboxSize::Compact)
                .mono(true)
                .into_any_element(),
            "checkbox.indeterminate" => MoonCheckbox::new("handoff-checkbox-indeterminate")
                .label("mixed")
                .indeterminate(true)
                .mono(true)
                .into_any_element(),
            "radio.checked" => MoonRadio::new("handoff-radio-checked")
                .label("Market")
                .checked(true)
                .size(MoonRadioSize::Normal)
                .into_any_element(),
            "radio.unchecked" => MoonRadio::new("handoff-radio-unchecked")
                .label("Market")
                .checked(false)
                .size(MoonRadioSize::Normal)
                .into_any_element(),
            "rating.stars" => MoonRating::new("handoff-rating")
                .value(3)
                .max(5)
                .tone(MoonTone::Warning)
                .render()
                .into_any_element(),
            "toggle.checked" => MoonToggle::new("handoff-toggle-checked")
                .label("Live")
                .checked(true)
                .size(MoonToggleSize::Normal)
                .into_any_element(),
            "toggle.unchecked" => MoonToggle::new("handoff-toggle-unchecked")
                .label("Live")
                .checked(false)
                .size(MoonToggleSize::Normal)
                .into_any_element(),
            "switch.checked" => MoonSwitch::new("handoff-switch-checked")
                .label("Live")
                .checked(true)
                .into_any_element(),
            "slider.diffused.58" => div()
                .w(px(220.0))
                .child(
                    MoonSlider::new(&self.slider_58_state)
                        .id("handoff-slider-58")
                        .height(18.0),
                )
                .into_any_element(),
            "slider.diffused.100" => div()
                .w(px(220.0))
                .child(
                    MoonSlider::new(&self.slider_100_state)
                        .id("handoff-slider-100")
                        .height(18.0),
                )
                .into_any_element(),
            "slider.range" => div()
                .w(px(220.0))
                .child(
                    MoonSlider::new(&self.range_slider_state)
                        .id("handoff-slider-range")
                        .height(18.0),
                )
                .into_any_element(),
            "progress.positive" => div()
                .w(px(220.0))
                .child(
                    MoonProgress::new("handoff-progress-positive")
                        .value(64.0)
                        .tone(MoonTone::Positive)
                        .render(),
                )
                .into_any_element(),
            "progress.loading" => div()
                .w(px(220.0))
                .child(
                    MoonProgress::new("handoff-progress-loading")
                        .value(42.0)
                        .loading(true)
                        .tone(MoonTone::Info)
                        .render(),
                )
                .into_any_element(),
            "progress.warning" => div()
                .w(px(220.0))
                .child(
                    MoonProgress::new("handoff-progress-warning")
                        .value(28.0)
                        .tone(MoonTone::Warning)
                        .render(),
                )
                .into_any_element(),
            "progress_circle.normal" => MoonProgressCircle::new("handoff-progress-circle")
                .value(68.0)
                .tone(MoonTone::Info)
                .size(MoonProgressCircleSize::Normal)
                .render()
                .into_any_element(),
            "preset_strip.default" => MoonPresetStrip::new("handoff-preset-strip")
                .slot_width(82.0)
                .items([
                    MoonPresetItem::new("F1", "1", "0.01"),
                    MoonPresetItem::new("F2", "2", "0.025"),
                    MoonPresetItem::new("F3", "3", "0.05").selected(true),
                    MoonPresetItem::new("F4", "4", "0.10").disabled(true),
                ])
                .render()
                .into_any_element(),
            "tab_strip.default" => div()
                .relative()
                .w(px(340.0))
                .h(px(30.0))
                .child(
                    MoonTabStrip::new("handoff-tab-strip")
                        .bounds(moon_ui::MoonRect::new(0.0, 0.0, 340.0, 30.0))
                        .items([
                            MoonTabItem::new("Main").selected(true).width(86.0),
                            MoonTabItem::new("Assets").width(86.0),
                            MoonTabItem::new("Log").width(70.0),
                        ])
                        .render(),
                )
                .into_any_element(),
            "segmented.presets" => MoonSegmentedControl::new("handoff-segmented")
                .items([
                    MoonSegmentItem::new("F1", "0.01").fit_width(cx, 52.0, 110.0),
                    MoonSegmentItem::new("F2", "0.025").fit_width(cx, 52.0, 110.0),
                    MoonSegmentItem::new("F3", "0.05")
                        .fit_width(cx, 52.0, 110.0)
                        .selected(true),
                    MoonSegmentItem::new("F4", "0.10")
                        .fit_width(cx, 52.0, 110.0)
                        .disabled(true),
                ])
                .replace_item(
                    1,
                    div()
                        .size_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(MoonText::new("EDIT").uppercase(false).mono(true).render()),
                )
                .render()
                .into_any_element(),
            "selector.pill" => MoonSelectorPill::new("handoff-selector-pill")
                .leading_dot(p.green)
                .segment(MoonSelectorSegment::new("default").color(p.text_muted))
                .segment(
                    MoonSelectorSegment::new("BTCUSDT")
                        .color(p.text)
                        .weight(600.0),
                )
                .render()
                .into_any_element(),
            "breadcrumb.path" => MoonBreadcrumb::new()
                .child(MoonBreadcrumbItem::new("MoonUI"))
                .child("Components")
                .child("Inputs")
                .render()
                .into_any_element(),
            "pagination.basic" => MoonPagination::new("handoff-pagination")
                .current_page(4)
                .total_pages(12)
                .visible_pages(7)
                .small()
                .render()
                .into_any_element(),
            "table.basic" => div()
                .w(px(390.0))
                .h(px(110.0))
                .child(
                    MoonDataTable::new("handoff-table-basic", 3, move |ix, _, app| {
                        let p = MoonPalette::active(app);
                        MoonDataRow::new([
                            MoonDataCell::text(format!("MOON/{ix:03}")).weight(600.0),
                            MoonDataCell::text(if ix == 1 { "SHORT" } else { "LONG" }).tone(
                                if ix == 1 {
                                    MoonTone::Danger
                                } else {
                                    MoonTone::Positive
                                },
                            ),
                            MoonDataCell::text(format!("{:.2}", 1200.0 + ix as f32 * 17.5))
                                .text_color(if ix == 1 { p.orange } else { p.green }),
                        ])
                        .selected(ix == 1)
                    })
                    .state(&self.data_table_state)
                    .columns([
                        MoonDataTableColumn::new("market", "MARKET", 120.0),
                        MoonDataTableColumn::new("side", "SIDE", 90.0),
                        MoonDataTableColumn::new("pnl", "PNL", 120.0).right().fill(),
                    ])
                    .row_height(25.0)
                    .header_height(26.0),
                )
                .into_any_element(),
            "table.primitives" => div()
                .w(px(360.0))
                .h(px(84.0))
                .child(
                    MoonDataTable::new("handoff-table-primitives", 2, move |ix, _, app| {
                        let p = MoonPalette::active(app);
                        MoonDataRow::new([
                            MoonDataCell::text(if ix == 0 {
                                "MoonTableRow"
                            } else {
                                "MoonTableCell"
                            })
                            .tone(MoonTone::Info)
                            .weight(600.0),
                            MoonDataCell::element(
                                MoonBadge::new(if ix == 0 { "selected" } else { "element" })
                                    .tone(if ix == 0 {
                                        MoonTone::Warning
                                    } else {
                                        MoonTone::Positive
                                    })
                                    .render(),
                            ),
                            MoonDataCell::text(if ix == 0 { "right" } else { "align" })
                                .text_color(p.text_soft),
                        ])
                        .selected(ix == 0)
                    })
                    .state(&self.data_table_state)
                    .columns([
                        MoonDataTableColumn::new("primitive", "PRIMITIVE", 150.0),
                        MoonDataTableColumn::new("content", "CONTENT", 110.0),
                        MoonDataTableColumn::new("align", "ALIGN", 90.0)
                            .right()
                            .fill(),
                    ])
                    .row_height(25.0)
                    .header_height(26.0),
                )
                .into_any_element(),
            "list.selected" => v_flex()
                .w(px(240.0))
                .gap(px(2.0))
                .child(MoonListItem::new(MoonComponentIndexPath::new(0)).child("server 1"))
                .child(
                    MoonListItem::new(MoonComponentIndexPath::new(1))
                        .selected(true)
                        .child("HooksDetec..."),
                )
                .child(MoonListItem::new(MoonComponentIndexPath::new(2)).child("Moon Hook"))
                .into_any_element(),
            "list.full" => div()
                .w(px(290.0))
                .h(px(150.0))
                .child(
                    MoonList::new(&self.list_state)
                        .search_placeholder("Filter")
                        .scrollbar_visible(true),
                )
                .into_any_element(),
            "virtual_list.basic" => div()
                .w(px(300.0))
                .h(px(150.0))
                .child(
                    MoonVirtualList::new("handoff-virtual-list", 500, 30.0, |ix, _, app| {
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
                    })
                    .track_scroll(&self.virtual_scroll)
                    .scrollbar_visibility(MoonScrollbarVisibility::Always)
                    .background_policy(MoonBackgroundPolicy::Opaque)
                    .tail_fill_color(p.shell),
                )
                .into_any_element(),
            "tree.basic" => div()
                .w(px(290.0))
                .h(px(150.0))
                .child(MoonTree::new(
                    &self.tree_state,
                    |ix, entry, selected, _, app| {
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
                                        .color(if selected { p.text } else { p.text_soft })
                                        .render(),
                                ),
                        )
                    },
                ))
                .into_any_element(),
            "description_list.basic" => div()
                .w(px(330.0))
                .child(
                    MoonDescriptionList::new()
                        .columns(2)
                        .small()
                        .item("Component class", "MoonReady", 1)
                        .item("Behavior", "Longbridge", 1)
                        .item("Theme", "Moon tokens", 1)
                        .item("Snapshot", "covered", 1)
                        .render(),
                )
                .into_any_element(),
            "calendar.month" => div()
                .w(px(230.0))
                .child(
                    MoonCalendar::new(&self.calendar_state)
                        .number_of_months(1)
                        .w(px(220.0)),
                )
                .into_any_element(),
            "date_picker.trigger" => div()
                .w(px(280.0))
                .child(
                    MoonDatePicker::new(&self.date_picker_state)
                        .placeholder("Pick session date")
                        .cleanable(true)
                        .number_of_months(1),
                )
                .into_any_element(),
            "date_time_picker.trigger" => div()
                .w(px(280.0))
                .child(
                    MoonDateTimePicker::new("case-date-time-picker", &self.date_time_picker_state)
                        .placeholder("Pick session date and time")
                        .number_of_months(1)
                        .render(),
                )
                .into_any_element(),
            "date_time_picker.open" => div()
                .w(px(280.0))
                .child(
                    MoonDateTimePicker::new(
                        "case-date-time-picker-open",
                        &self.open_date_time_picker_state,
                    )
                    .placeholder("Pick session date and time")
                    .number_of_months(1)
                    .render(),
                )
                .into_any_element(),
            "time_picker.trigger" => div()
                .w(px(280.0))
                .child(MoonTimePicker::new("case-time-picker", &self.time_picker_state).render())
                .into_any_element(),
            "time_picker.open" => div()
                .w(px(280.0))
                .child(
                    MoonTimePicker::new("case-time-picker-open", &self.open_time_picker_state)
                        .render(),
                )
                .into_any_element(),
            // The three picker fields stacked: they must share one background, border, radius and
            // height, because they sit next to each other in real forms.
            "pickers.parity" => v_flex()
                .w(px(280.0))
                .gap(px(10.0))
                .child(
                    MoonDatePicker::new(&self.date_picker_state)
                        .placeholder("Pick session date")
                        .number_of_months(1),
                )
                .child(
                    MoonDateTimePicker::new(
                        "case-parity-date-time-picker",
                        &self.date_time_picker_state,
                    )
                    .placeholder("Pick session date and time")
                    .number_of_months(1)
                    .render(),
                )
                .child(
                    MoonTimePicker::new("case-parity-time-picker", &self.time_picker_state)
                        .render(),
                )
                .into_any_element(),
            "dock.area" => div()
                .w(px(500.0))
                .h(px(240.0))
                .child(self.dock.clone())
                .into_any_element(),
            "tab_panel.default" => div()
                .w(px(380.0))
                .h(px(160.0))
                .child(
                    TabPanel::new("handoff-tab-panel", gallery_tab_panels())
                        .active_index(1)
                        .background_policy(MoonBackgroundPolicy::Opaque)
                        .content_background_policy(MoonBackgroundPolicy::Transparent)
                        .header_background_policy(MoonBackgroundPolicy::Opaque),
                )
                .into_any_element(),
            "resizable.group" => {
                let resizable: MoonResizablePanelGroup = moon_h_resizable("handoff-resizable")
                    .child(
                        moon_resizable_panel()
                            .size(px(140.0))
                            .size_range(px(110.0)..px(220.0))
                            .flex_none()
                            .child(
                                MoonSurface::new()
                                    .id("handoff-resizable-left")
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
                                                MoonText::new("Drag divider")
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
                                .id("handoff-resizable-right")
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
                                                "Longbridge resize engine, Moon surfaces",
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
                div()
                    .w(px(380.0))
                    .h(px(120.0))
                    .child(resizable)
                    .into_any_element()
            }
            "scroll_area.overlay" => handoff_scroll_area(cx).into_any_element(),
            "popup_menu.scale" => MoonPopupMenu::new("handoff-popup-scale")
                .fit_width(120.0, 240.0)
                .max_height_ui(120.0)
                .items([
                    MoonMenuItem::new("Auto").selected(true),
                    MoonMenuItem::new("50%").right_label("F2"),
                    MoonMenuItem::new("20%"),
                    MoonMenuItem::separator(),
                    MoonMenuItem::new("5%"),
                    MoonMenuItem::new("2%"),
                ])
                .render()
                .into_any_element(),
            "dropdown.open" => MoonDropdown::new("handoff-dropdown")
                .label("Scale Auto")
                .trigger_caret(true)
                .fit_trigger_width(100.0, 180.0)
                .default_open(true)
                .fit_menu_width(120.0, 240.0)
                .items([
                    MoonMenuItem::with_key("Auto", "Auto").selected(true),
                    MoonMenuItem::with_key("50", "50%"),
                    MoonMenuItem::with_key("20", "20%").checked(true),
                    MoonMenuItem::separator(),
                    MoonMenuItem::new("Advanced").selected(true).submenu([
                        MoonMenuItem::new("Adaptive trailing offset"),
                        MoonMenuItem::new("Fixed risk budget").right_label("Ctrl+F"),
                    ]),
                ])
                .into_any_element(),
            "context_menu.basic" => div()
                .relative()
                .w(px(260.0))
                .h(px(150.0))
                .child(
                    MoonButton::new("handoff-context-menu-trigger")
                        .label("Context target")
                        .variant(MoonButtonVariant::Panel)
                        .render(),
                )
                .child(
                    MoonContextMenu::new("handoff-context-menu")
                        .position(point(px(72.0), px(42.0)))
                        .open(true)
                        .width(170.0)
                        .items([
                            MoonMenuItem::new("Root context"),
                            MoonMenuItem::new("Move").right_label("M"),
                            MoonMenuItem::new("Delete").tone(MoonTone::Danger),
                        ]),
                )
                .into_any_element(),
            "popover.open" => div()
                .relative()
                .w(px(240.0))
                .h(px(140.0))
                .child(
                    MoonPopover::new("handoff-popover")
                        .open(true)
                        .placement(MoonPopoverPlacement::BottomStart)
                        .fit_content()
                        .trigger(
                            MoonButton::new("handoff-popover-trigger")
                                .label("Open popover")
                                .variant(MoonButtonVariant::Panel)
                                .render(),
                        )
                        .content(
                            v_flex()
                                .w(px(210.0))
                                .gap(px(8.0))
                                .child(
                                    MoonText::new("Popover content")
                                        .uppercase(false)
                                        .mono(true)
                                        .render(),
                                )
                                .child(
                                    MoonButton::new("handoff-popover-action")
                                        .label("Action")
                                        .variant(MoonButtonVariant::Blue)
                                        .render(),
                                ),
                        ),
                )
                .into_any_element(),
            "hover_card.basic" => MoonHoverCard::new("handoff-hover-card")
                .trigger(
                    MoonButton::new("handoff-hover-card-trigger")
                        .label("Hover card")
                        .variant(MoonButtonVariant::Panel)
                        .render(),
                )
                .content(|_, _, cx| {
                    let p = MoonPalette::active(cx);
                    v_flex()
                        .w(px(220.0))
                        .gap(px(8.0))
                        .child(
                            MoonBadge::new("MoonHoverCard")
                                .tone(MoonTone::Info)
                                .render(),
                        )
                        .child(
                            MoonText::new("HoverCard content is root popover behavior.")
                                .uppercase(false)
                                .mono(true)
                                .wrap()
                                .color(p.text_soft)
                                .render(),
                        )
                })
                .into_any_element(),
            "hover_card.open" => v_flex()
                .w(px(250.0))
                .gap(px(8.0))
                .p(px(12.0))
                .rounded(px(6.0))
                .border_1()
                .border_color(rgba_from(p.border, 1.0))
                .bg(rgba_from(p.panel, 1.0))
                .shadow(vec![box_shadow(
                    px(0.0),
                    px(10.0),
                    px(28.0),
                    px(0.0),
                    rgba_from(p.shadow, 0.36),
                )])
                .child(
                    MoonBadge::new("MoonHoverCard")
                        .tone(MoonTone::Info)
                        .render(),
                )
                .child(
                    MoonText::new("Hover card surface shown as an opened overlay state.")
                        .uppercase(false)
                        .mono(true)
                        .wrap()
                        .color(p.text_soft)
                        .render(),
                )
                .into_any_element(),
            "tooltip.default" => MoonTooltip::new("Scale menu")
                .detail("Long tooltip text wraps in the Moon tooltip body.")
                .shortcut("Ctrl+K")
                .placement(MoonTooltipPlacement::Top)
                .size(MoonTooltipSize::Normal)
                .tone(MoonTone::Info)
                .max_width(230.0)
                .arrow(true)
                .into_any_element(),
            "tooltip_view.entity" => self.tooltip_view.clone().into_any_element(),
            "dialog.confirm" => div().size_full().into_any_element(),
            "dialog.form" => v_flex()
                .w(px(330.0))
                .gap(px(12.0))
                .p(px(14.0))
                .rounded(px(6.0))
                .border_1()
                .border_color(rgba_from(p.border, 1.0))
                .bg(rgba_from(p.shell, 1.0))
                .shadow(vec![box_shadow(
                    px(0.0),
                    px(12.0),
                    px(30.0),
                    px(0.0),
                    rgba_from(p.shadow, 0.42),
                )])
                .child(
                    MoonText::new("Rename strategy")
                        .uppercase(false)
                        .weight(700.0)
                        .render(),
                )
                .child(
                    MoonInput::new("handoff-dialog-form-input").default_value("HooksDetect 0.3-1%"),
                )
                .child(
                    h_flex()
                        .justify_end()
                        .gap(px(8.0))
                        .child(
                            MoonButton::new("handoff-dialog-form-cancel")
                                .label("Cancel")
                                .variant(MoonButtonVariant::Panel)
                                .render(),
                        )
                        .child(
                            MoonButton::new("handoff-dialog-form-save")
                                .label("Save")
                                .variant(MoonButtonVariant::Blue)
                                .render(),
                        ),
                )
                .into_any_element(),
            "sheet.trigger" => MoonButton::new("handoff-sheet-trigger")
                .label("Open MoonSheet")
                .variant(MoonButtonVariant::Panel)
                .on_click(|_, window, app| {
                    window.open_moon_sheet_at(MoonPlacement::Right, app, |sheet, _, cx| {
                        let p = MoonPalette::active(cx);
                        sheet.title(div().child("MoonSheet")).size(px(320.0)).child(
                            v_flex()
                                .gap(px(10.0))
                                .child(
                                    MoonBadge::new("root overlay")
                                        .tone(MoonTone::Info)
                                        .variant(MoonBadgeVariant::Outline)
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
                    });
                })
                .render()
                .into_any_element(),
            "sheet.panel" => h_flex()
                .w(px(320.0))
                .h(px(180.0))
                .justify_end()
                .child(
                    v_flex()
                        .w(px(230.0))
                        .h_full()
                        .gap(px(10.0))
                        .p(px(12.0))
                        .border_l_1()
                        .border_color(rgba_from(p.border, 1.0))
                        .bg(rgba_from(p.shell, 1.0))
                        .shadow(vec![box_shadow(
                            px(-10.0),
                            px(0.0),
                            px(26.0),
                            px(0.0),
                            rgba_from(p.shadow, 0.34),
                        )])
                        .child(
                            MoonText::new("MoonSheet")
                                .uppercase(false)
                                .weight(700.0)
                                .render(),
                        )
                        .child(
                            MoonBadge::new("root overlay")
                                .tone(MoonTone::Info)
                                .variant(MoonBadgeVariant::Outline)
                                .render(),
                        )
                        .child(
                            MoonText::new("Right-side sheet panel, owned by Root/window.")
                                .uppercase(false)
                                .mono(true)
                                .wrap()
                                .color(p.text_soft)
                                .render(),
                        ),
                )
                .into_any_element(),
            "native_menu.trigger" => MoonButton::new("handoff-native-menu-trigger")
                .label("Open native menu")
                .variant(MoonButtonVariant::Panel)
                .on_click(|_, window, app| {
                    MoonNativeMenu::new()
                        .label("MoonNativeMenu")
                        .menu("No-op action", Box::new(NoAction))
                        .menu_with_check("Checked item", true, Box::new(NoAction))
                        .separator()
                        .submenu(
                            "Submenu",
                            MoonNativeMenu::new().menu("Nested item", Box::new(NoAction)),
                        )
                        .show(point(px(180.0), px(180.0)), window, app);
                })
                .render()
                .into_any_element(),
            "native_menu.fallback" => MoonPopupMenu::new("handoff-native-menu-fallback")
                .width(190.0)
                .items([
                    MoonMenuItem::new("MoonNativeMenu"),
                    MoonMenuItem::separator(),
                    MoonMenuItem::new("Open"),
                    MoonMenuItem::new("Detach").right_label("D"),
                    MoonMenuItem::new("Close").tone(MoonTone::Danger),
                ])
                .render()
                .into_any_element(),
            "notification.info" => MoonButton::new("handoff-notification-trigger")
                .label("Push MoonNotification")
                .variant(MoonButtonVariant::Panel)
                .on_click(|_, window, app| {
                    window.push_notification(
                        MoonNotification::info("Root-owned MoonNotification")
                            .title("MoonNotification")
                            .autohide(false),
                        app,
                    );
                })
                .render()
                .into_any_element(),
            "notification.toast" => h_flex()
                .w(px(320.0))
                .gap(px(10.0))
                .p(px(12.0))
                .rounded(px(6.0))
                .border_1()
                .border_color(rgba_from(p.border, 1.0))
                .bg(rgba_from(p.panel, 1.0))
                .shadow(vec![box_shadow(
                    px(0.0),
                    px(8.0),
                    px(24.0),
                    px(0.0),
                    rgba_from(p.shadow, 0.34),
                )])
                .child(
                    div()
                        .w(px(7.0))
                        .h(px(7.0))
                        .rounded(px(999.0))
                        .bg(rgb(p.blue)),
                )
                .child(
                    v_flex()
                        .gap(px(4.0))
                        .child(
                            MoonText::new("MoonNotification")
                                .uppercase(false)
                                .weight(700.0)
                                .render(),
                        )
                        .child(
                            MoonText::new("Operation queued successfully.")
                                .uppercase(false)
                                .mono(true)
                                .color(p.text_soft)
                                .render(),
                        ),
                )
                .into_any_element(),
            "alert.info" => MoonAlert::info(
                "handoff-alert-info",
                "MoonAlert mirrors Longbridge alert behavior behind a Moon-facing API.",
            )
            .title("Info alert")
            .render()
            .into_any_element(),
            "accordion.basic" => MoonAccordion::new("handoff-accordion")
                .multiple(true)
                .item(|item| {
                    item.title("MoonAccordion item").open(true).child(
                        MoonText::new("Expansion behavior, Moon-facing API.")
                            .uppercase(false)
                            .mono(true)
                            .wrap()
                            .color(p.text_soft)
                            .render(),
                    )
                })
                .item(|item| item.title("Second item").child("Closed content"))
                .render()
                .into_any_element(),
            "collapsible.open" => MoonCollapsible::new("handoff-collapsible")
                .title("MoonCollapsible")
                .default_open(true)
                .content(
                    MoonText::new("Expanded content keeps the Moon surface and typography rules.")
                        .uppercase(false)
                        .mono(true)
                        .wrap()
                        .color(p.text_soft)
                        .render(),
                )
                .into_any_element(),
            "group_box.basic" => MoonGroupBox::new("handoff-group-box")
                .title("MoonGroupBox")
                .child(
                    MoonFormRow::new("handoff-group-row", "Mode")
                        .label_width(80.0)
                        .control(
                            MoonSelectorPill::new("handoff-group-selector")
                                .leading_dot(p.green)
                                .label("BTCUSDT")
                                .render(),
                        ),
                )
                .into_any_element(),
            "sidebar.basic" => MoonSidebar::new("handoff-sidebar")
                .w(px(220.0))
                .h(px(220.0))
                .header(h_flex().gap(px(8.0)).child("MoonSidebar"))
                .child(
                    MoonSidebarGroup::new("Navigation").child(
                        MoonSidebarMenu::new().children([
                            MoonSidebarMenuItem::new("Controls").active(true),
                            MoonSidebarMenuItem::new("Inputs"),
                            MoonSidebarMenuItem::new("Overlays")
                                .children([
                                    MoonSidebarMenuItem::new("Dialog"),
                                    MoonSidebarMenuItem::new("Sheet"),
                                ])
                                .default_open(true),
                        ]),
                    ),
                )
                .into_any_element(),
            "settings.page" => {
                let enabled = Rc::new(Cell::new(true));
                let symbol = Rc::new(RefCell::new(SharedString::from("BTCUSDT")));
                let mode = Rc::new(RefCell::new(SharedString::from("paper")));
                let risk = Rc::new(Cell::new(2.5));

                div()
                    .w(px(420.0))
                    .h(px(220.0))
                    .child(
                        MoonSettings::new("handoff-settings")
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
                                                    let value = enabled.clone();
                                                    let set_value = enabled.clone();
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
                                                    let value = symbol.clone();
                                                    let set_value = symbol.clone();
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
                                                    let value = mode.clone();
                                                    let set_value = mode.clone();
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
                                            )
                                            .item(
                                                MoonSettingItem::new("Risk", {
                                                    let value = risk.clone();
                                                    let set_value = risk.clone();
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
                                                })
                                                .description("Number field."),
                                            ),
                                    ),
                            ),
                    )
                    .into_any_element()
            }
            "badge.variants" => h_flex()
                .gap(px(8.0))
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
                .into_any_element(),
            "tag.variants" => h_flex()
                .gap(px(8.0))
                .child(
                    MoonTag::positive()
                        .rounded_full()
                        .label("positive")
                        .render(),
                )
                .child(MoonTag::warning().outline().label("warning").render())
                .child(MoonTag::danger().outline().label("danger").render())
                .into_any_element(),
            "kbd.spinner.skeleton" => h_flex()
                .gap(px(10.0))
                .items_center()
                .child(MoonKbd::new("Ctrl+K").size(MoonKbdSize::Normal))
                .child(MoonSpinner::new().size(MoonSpinnerSize::Normal))
                .child(
                    MoonSkeleton::new("handoff-skeleton")
                        .width(120.0)
                        .height(14.0),
                )
                .into_any_element(),
            "label.link.text" => v_flex()
                .gap(px(8.0))
                .child(
                    MoonLabel::new("MoonLabel")
                        .mono(true)
                        .weight(600.0)
                        .color(p.text)
                        .render(),
                )
                .child(
                    MoonText::new("MoonText wraps designer handoff copy.")
                        .uppercase(false)
                        .mono(true)
                        .wrap()
                        .color(p.text_soft)
                        .render(),
                )
                .child(MoonLink::new("handoff-link", "MoonLink"))
                .into_any_element(),
            "separator.basic" => v_flex()
                .w(px(220.0))
                .gap(px(10.0))
                .child(MoonText::new("Above").uppercase(false).mono(true).render())
                .child(MoonSeparator::horizontal())
                .child(
                    h_flex()
                        .h(px(24.0))
                        .gap(px(10.0))
                        .child(MoonText::new("Left").uppercase(false).mono(true).render())
                        .child(MoonSeparator::vertical())
                        .child(MoonText::new("Right").uppercase(false).mono(true).render()),
                )
                .into_any_element(),
            "status_bar.basic" => div()
                .w(px(420.0))
                .child(
                    MoonStatusBar::new("handoff-status-bar")
                        .indicator(MoonStatusIndicator::new(p.green).glow(8.0, 0.28))
                        .items([
                            MoonStatusItem::new("Binance Futures").tone(MoonTone::Info),
                            MoonStatusItem::separator(),
                            MoonStatusItem::new("Live")
                                .tone(MoonTone::Positive)
                                .weight(700.0),
                        ])
                        .right_item(MoonStatusItem::new("18%").tone(MoonTone::Positive)),
                )
                .into_any_element(),
            _ => MoonText::new(format!("Missing handoff case: {}", case.id))
                .uppercase(false)
                .mono(true)
                .color(p.red)
                .render()
                .into_any_element(),
        }
    }
}

impl Render for CaseGallery {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.schedule_case_snapshot_capture(window, cx);
        let case = self.current_case();
        let p = MoonPalette::active(cx);
        div()
            .id("handoff-case-root")
            .size_full()
            .overflow_hidden()
            .flex()
            .items_center()
            .justify_center()
            .text_color(rgb(p.text))
            .bg(rgba_from(p.shell, 1.0))
            .child(self.render_case_component(case, window, cx))
    }
}

fn handoff_chart_stack(cx: &App) -> impl IntoElement {
    let p = MoonPalette::active(cx);
    v_flex()
        .w(px(860.0))
        .h(px(560.0))
        .border_1()
        .border_color(rgba_from(p.border, 1.0))
        .bg(rgba_from(p.shell, 1.0))
        .text_color(rgb(p.text))
        .child(
            h_flex()
                .h(px(34.0))
                .px(px(10.0))
                .gap(px(10.0))
                .border_b_1()
                .border_color(rgba_from(p.border, 1.0))
                .bg(rgba_from(p.shell_high, 1.0))
                .child(MoonAvatar::new().initials("M").compact().render())
                .child(
                    MoonText::new("Moonbot")
                        .uppercase(false)
                        .weight(700.0)
                        .render(),
                )
                .child(MoonBadge::new("default В· BTCUSDT").render())
                .child(MoonBadge::new("Live").tone(MoonTone::Positive).render())
                .child(div().flex_1())
                .child(
                    MoonText::new("3 charts / scroll")
                        .uppercase(false)
                        .mono(true)
                        .color(p.text_soft)
                        .render(),
                ),
        )
        .child(
            h_flex()
                .h(px(34.0))
                .px(px(10.0))
                .gap(px(8.0))
                .border_b_1()
                .border_color(rgba_from(p.border, 1.0))
                .child(
                    MoonButton::new("handoff-chart-stack-tp")
                        .label("TP +3.0%")
                        .variant(MoonButtonVariant::Blue)
                        .render(),
                )
                .child(
                    MoonButton::new("handoff-chart-stack-sl")
                        .label("SL -2.0%")
                        .variant(MoonButtonVariant::Danger)
                        .render(),
                )
                .child(
                    MoonButton::new("handoff-chart-stack-lev")
                        .label("Lev x1")
                        .variant(MoonButtonVariant::Panel)
                        .render(),
                )
                .child(div().flex_1())
                .child(
                    MoonButton::new("handoff-chart-stack-live")
                        .label("Live")
                        .variant(MoonButtonVariant::Green)
                        .render(),
                ),
        )
        .child(
            div()
                .relative()
                .flex_1()
                .overflow_hidden()
                .child(
                    v_flex()
                        .absolute()
                        .left(px(8.0))
                        .top(px(8.0))
                        .right(px(16.0))
                        .gap(px(8.0))
                        .child(handoff_chart_panel("Chart 1 В· BTCUSDT", p.blue, p))
                        .child(handoff_chart_panel("Chart 2 В· ETHUSDT", p.green, p))
                        .child(handoff_chart_panel("Chart 3 В· SOLUSDT", p.amber, p)),
                )
                .child(
                    div()
                        .absolute()
                        .right(px(5.0))
                        .top(px(10.0))
                        .bottom(px(10.0))
                        .w(px(4.0))
                        .rounded(px(999.0))
                        .bg(rgba_from(p.overlay, 0.05))
                        .child(
                            div()
                                .absolute()
                                .top(px(28.0))
                                .w(px(4.0))
                                .h(px(112.0))
                                .rounded(px(999.0))
                                .bg(rgba_from(p.text_muted, 0.64)),
                        ),
                ),
        )
        .child(
            MoonStatusBar::new("handoff-chart-stack-status")
                .indicator(MoonStatusIndicator::new(p.green).glow(8.0, 0.28))
                .items([
                    MoonStatusItem::new("chart host scroll").tone(MoonTone::Info),
                    MoonStatusItem::separator(),
                    MoonStatusItem::new("plot area uses chartdx, not UI").tone(MoonTone::Warning),
                ])
                .right_item(MoonStatusItem::new("visible: 2.4 / 3").tone(MoonTone::Positive)),
        )
}

fn handoff_chart_panel(title: &'static str, tone: u32, p: MoonPalette) -> impl IntoElement {
    v_flex()
        .h(px(184.0))
        .border_1()
        .border_color(rgba_from(tone, 0.62))
        .bg(rgba_from(p.panel, 1.0))
        .child(
            h_flex()
                .h(px(30.0))
                .px(px(10.0))
                .gap(px(8.0))
                .border_b_1()
                .border_color(rgba_from(p.border, 1.0))
                .bg(rgba_from(p.shell_high, 0.78))
                .child(
                    div()
                        .w(px(22.0))
                        .h(px(16.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(3.0))
                        .border_1()
                        .border_color(rgba_from(tone, 0.86))
                        .child(
                            MoonText::new(match title {
                                "Chart 1 В· BTCUSDT" => "01",
                                "Chart 2 В· ETHUSDT" => "02",
                                _ => "03",
                            })
                            .uppercase(false)
                            .mono(true)
                            .font_size(9.0)
                            .line_height(10.0)
                            .weight(700.0)
                            .color(tone)
                            .render(),
                        ),
                )
                .child(
                    MoonText::new(title)
                        .uppercase(false)
                        .mono(true)
                        .weight(700.0)
                        .color(tone)
                        .render(),
                )
                .child(div().flex_1())
                .child(
                    MoonBadge::new("NoFill chart host")
                        .tone(MoonTone::Warning)
                        .render(),
                ),
        )
        .child(
            div()
                .relative()
                .flex_1()
                .overflow_hidden()
                .bg(rgb(p.chart_bg))
                .child(
                    div()
                        .absolute()
                        .left(px(0.0))
                        .right(px(0.0))
                        .top(px(36.0))
                        .h(px(1.0))
                        .bg(rgba_from(p.overlay, 0.04)),
                )
                .child(
                    div()
                        .absolute()
                        .left(px(0.0))
                        .right(px(0.0))
                        .top(px(74.0))
                        .h(px(1.0))
                        .bg(rgba_from(p.overlay, 0.04)),
                )
                .child(
                    div()
                        .absolute()
                        .left(px(88.0))
                        .top(px(0.0))
                        .bottom(px(0.0))
                        .w(px(1.0))
                        .bg(rgba_from(p.overlay, 0.04)),
                )
                .child(
                    div()
                        .absolute()
                        .left(px(240.0))
                        .top(px(0.0))
                        .bottom(px(0.0))
                        .w(px(1.0))
                        .bg(rgba_from(p.overlay, 0.04)),
                )
                .child(
                    div()
                        .absolute()
                        .left(px(10.0))
                        .top(px(8.0))
                        .h(px(17.0))
                        .px(px(6.0))
                        .flex()
                        .items_center()
                        .rounded(px(3.0))
                        .border_1()
                        .border_color(rgba_from(p.border, 0.86))
                        .bg(rgba_from(p.shell, 0.72))
                        .child(
                            MoonText::new("plot / chartdx")
                                .uppercase(false)
                                .mono(true)
                                .font_size(9.0)
                                .line_height(10.0)
                                .weight(600.0)
                                .color(p.text_muted)
                                .render(),
                        ),
                )
                .child(
                    div()
                        .absolute()
                        .left(px(350.0))
                        .top(px(54.0))
                        .w(px(280.0))
                        .h(px(2.0))
                        .bg(rgb(tone)),
                )
                .child(
                    div()
                        .absolute()
                        .right(px(0.0))
                        .top(px(0.0))
                        .bottom(px(0.0))
                        .w(px(120.0))
                        .bg(rgba_from(tone, 0.12)),
                ),
        )
}

fn handoff_strategy_editor(cx: &App) -> impl IntoElement {
    let p = MoonPalette::active(cx);
    v_flex()
        .w(px(860.0))
        .h(px(560.0))
        .border_1()
        .border_color(rgba_from(p.border, 1.0))
        .bg(rgba_from(p.shell, 1.0))
        .text_color(rgb(p.text))
        .child(
            h_flex()
                .h(px(26.0))
                .px(px(10.0))
                .gap(px(8.0))
                .border_b_1()
                .border_color(rgba_from(p.border, 1.0))
                .bg(rgba_from(p.shell_high, 1.0))
                .child(
                    div()
                        .w(px(7.0))
                        .h(px(7.0))
                        .rounded(px(999.0))
                        .bg(rgb(p.orange)),
                )
                .child(
                    MoonText::new("Strategies")
                        .uppercase(false)
                        .weight(700.0)
                        .render(),
                )
                .child(div().flex_1())
                .child(
                    MoonText::new("fields: 15")
                        .uppercase(false)
                        .mono(true)
                        .color(p.text_soft)
                        .render(),
                )
                .child(MoonText::new("-").mono(true).color(p.text_soft).render())
                .child(MoonText::new("x").mono(true).color(p.orange).render()),
        )
        .child(
            h_flex()
                .flex_1()
                .min_h_0()
                .child(
                    v_flex()
                        .w(px(218.0))
                        .size_full()
                        .p(px(10.0))
                        .gap(px(8.0))
                        .border_1()
                        .border_color(rgba_from(p.border, 1.0))
                        .bg(rgba_from(p.shell_high, 0.72))
                        .child(
                            MoonInput::new("handoff-strategy-search")
                                .placeholder("search")
                                .small(),
                        )
                        .child(
                            h_flex()
                                .gap(px(8.0))
                                .child(
                                    MoonButton::new("handoff-strategy-type")
                                        .label("all types")
                                        .variant(MoonButtonVariant::Panel)
                                        .render(),
                                )
                                .child(
                                    MoonButton::new("handoff-strategy-filter")
                                        .label("all")
                                        .variant(MoonButtonVariant::Panel)
                                        .render(),
                                ),
                        )
                        .child(
                            MoonCheckbox::new("handoff-strategy-active")
                                .label("active only")
                                .checked(true)
                                .size(MoonCheckboxSize::Compact),
                        )
                        .child(strategy_tree_row("v server 1", "8/172", p.blue, false, p))
                        .child(strategy_tree_row("> TestF", "7/21", p.text_soft, false, p))
                        .child(strategy_tree_row(
                            "HooksDetec...  Moon Hook",
                            "",
                            p.text,
                            true,
                            p,
                        ))
                        .child(div().flex_1())
                        .child(
                            h_flex()
                                .gap(px(7.0))
                                .child(MoonBadge::new("marked").tone(MoonTone::Info).render())
                                .child(
                                    MoonBadge::new("canceled")
                                        .tone(MoonTone::Warning)
                                        .variant(MoonBadgeVariant::Outline)
                                        .render(),
                                ),
                        ),
                )
                .child(
                    v_flex()
                        .w(px(220.0))
                        .size_full()
                        .p(px(14.0))
                        .gap(px(4.0))
                        .border_1()
                        .border_color(rgba_from(p.border, 1.0))
                        .bg(rgba_from(p.shell, 1.0))
                        .child(
                            MoonText::new("Sections")
                                .uppercase(false)
                                .weight(700.0)
                                .render(),
                        )
                        .child(strategy_section("Main", true, p))
                        .child(strategy_section("Dynamic White\\Black List", false, p))
                        .child(strategy_section("Filters", false, p))
                        .child(strategy_section("Triggers Master / Slave", false, p))
                        .child(strategy_section("Buy conditions", false, p))
                        .child(strategy_section("Delta Modifiers", false, p))
                        .child(strategy_section("Sell order", false, p))
                        .child(strategy_section("Stops", false, p))
                        .child(strategy_section("Strategy settings", false, p))
                        .child(strategy_section_muted("User Interface", p))
                        .child(strategy_section_muted("Filters / Base", p))
                        .child(strategy_section_muted("Session", p)),
                )
                .child(
                    v_flex()
                        .flex_1()
                        .size_full()
                        .p(px(24.0))
                        .gap(px(11.0))
                        .bg(rgba_from(p.shell, 1.0))
                        .child(
                            h_flex()
                                .child(
                                    MoonText::new("Main")
                                        .uppercase(false)
                                        .weight(700.0)
                                        .render(),
                                )
                                .child(div().flex_1())
                                .child(
                                    MoonText::new("fields: 15")
                                        .uppercase(false)
                                        .mono(true)
                                        .color(p.text_muted)
                                        .render(),
                                ),
                        )
                        .child(
                            MoonCheckbox::new("handoff-strategy-form-active")
                                .label("active only")
                                .checked(true)
                                .size(MoonCheckboxSize::Compact),
                        )
                        .child(div().h(px(1.0)).bg(rgba_from(p.border, 1.0)))
                        .child(strategy_field_input(
                            "StrategyName",
                            "HooksDetect 0.3-1%",
                            p,
                        ))
                        .child(strategy_field_input("Comment", "", p))
                        .child(strategy_field_select("SignalType", "MoonHook", p))
                        .child(strategy_field_memo(
                            "CustomEMA",
                            "ema(close, 12) > ema(close, 36)\nand volume > sma(volume, 20)",
                            p,
                        ))
                        .child(strategy_field_memo(
                            "CommentMemo",
                            "long multiline strategy note / autocompletion target",
                            p,
                        ))
                        .child(
                            h_flex()
                                .gap(px(14.0))
                                .child(
                                    MoonCheckbox::new("handoff-silent")
                                        .label("SilentNoCharts")
                                        .checked(true)
                                        .size(MoonCheckboxSize::Compact),
                                )
                                .child(
                                    MoonCheckbox::new("handoff-debug")
                                        .label("DebugLog")
                                        .size(MoonCheckboxSize::Compact),
                                )
                                .child(
                                    MoonCheckbox::new("handoff-independent")
                                        .label("IndependentSignals")
                                        .checked(true)
                                        .size(MoonCheckboxSize::Compact),
                                ),
                        ),
                ),
        )
}

fn strategy_tree_row(
    label: &'static str,
    meta: &'static str,
    color: u32,
    selected: bool,
    p: MoonPalette,
) -> impl IntoElement {
    h_flex()
        .relative()
        .h(px(22.0))
        .px(px(7.0))
        .gap(px(8.0))
        .rounded(px(3.0))
        .border_1()
        .border_color(if selected {
            rgba_from(p.accent, 1.0)
        } else {
            rgba_from(p.border, 0.0)
        })
        .when(selected, |this| {
            this.bg(selected_background(p))
                .child(
                    div()
                        .absolute()
                        .left(px(0.0))
                        .top(px(3.0))
                        .bottom(px(3.0))
                        .w(px(3.0))
                        .bg(rgb(p.accent)),
                )
                .child(
                    div()
                        .absolute()
                        .left(px(7.0))
                        .top(px(9.0))
                        .w(px(4.0))
                        .h(px(4.0))
                        .rounded_full()
                        .bg(rgb(p.accent)),
                )
        })
        .child(
            MoonText::new(label)
                .uppercase(false)
                .mono(true)
                .color(color)
                .render(),
        )
        .child(div().flex_1())
        .child(
            MoonText::new(meta)
                .uppercase(false)
                .mono(true)
                .color(p.blue)
                .render(),
        )
}

fn strategy_section(label: &'static str, active: bool, p: MoonPalette) -> impl IntoElement {
    div()
        .h(px(24.0))
        .px(px(7.0))
        .flex()
        .items_center()
        .rounded(px(3.0))
        .border_1()
        .border_color(if active {
            rgba_from(p.accent, 1.0)
        } else {
            rgba_from(p.border, 0.0)
        })
        .bg(if active {
            rgba_from(p.accent, 0.24)
        } else {
            rgba_from(p.shell, 0.0)
        })
        .child(
            MoonText::new(label)
                .uppercase(false)
                .mono(true)
                .weight(700.0)
                .color(if active { p.amber } else { p.text })
                .render(),
        )
}

fn strategy_section_muted(label: &'static str, p: MoonPalette) -> impl IntoElement {
    div().h(px(24.0)).px(px(7.0)).flex().items_center().child(
        MoonText::new(label)
            .uppercase(false)
            .mono(true)
            .color(p.text_muted)
            .render(),
    )
}

fn strategy_field_input(
    label: &'static str,
    value: &'static str,
    p: MoonPalette,
) -> impl IntoElement {
    h_flex()
        .gap(px(14.0))
        .items_center()
        .child(strategy_form_label(label, p))
        .child(
            div()
                .flex_1()
                .child(MoonInput::new(label).default_value(value).small()),
        )
}

fn strategy_field_select(
    label: &'static str,
    value: &'static str,
    p: MoonPalette,
) -> impl IntoElement {
    h_flex()
        .gap(px(14.0))
        .items_center()
        .child(strategy_form_label(label, p))
        .child(
            MoonButton::new(format!("handoff-strategy-select-{label}"))
                .label(value)
                .variant(MoonButtonVariant::Panel)
                .width(180.0)
                .render(),
        )
}

fn strategy_field_memo(
    label: &'static str,
    value: &'static str,
    p: MoonPalette,
) -> impl IntoElement {
    h_flex()
        .gap(px(14.0))
        .items_start()
        .child(strategy_form_label(label, p))
        .child(
            div()
                .flex_1()
                .h(px(62.0))
                .p(px(9.0))
                .rounded(px(4.0))
                .border_1()
                .border_color(rgba_from(p.border, 1.0))
                .bg(rgba_from(p.panel, 1.0))
                .child(
                    MoonText::new(value)
                        .uppercase(false)
                        .mono(true)
                        .wrap()
                        .font_size(10.0)
                        .line_height(14.0)
                        .color(p.text_soft)
                        .render(),
                ),
        )
}

fn strategy_form_label(label: &'static str, p: MoonPalette) -> impl IntoElement {
    div().w(px(150.0)).child(
        MoonText::new(label)
            .uppercase(false)
            .mono(true)
            .color(p.text_soft)
            .render(),
    )
}

fn handoff_scroll_area(cx: &App) -> impl IntoElement {
    let p = MoonPalette::active(cx);
    div()
        .relative()
        .w(px(360.0))
        .h(px(136.0))
        .overflow_hidden()
        .rounded(px(5.0))
        .border_1()
        .border_color(rgba_from(p.border, 1.0))
        .bg(rgba_from(p.shell, 1.0))
        .child(
            v_flex()
                .absolute()
                .left(px(10.0))
                .top(px(10.0))
                .right(px(18.0))
                .bottom(px(18.0))
                .p(px(10.0))
                .gap(px(7.0))
                .border_1()
                .border_color(rgba_from(p.border, 0.7))
                .bg(rgba_from(p.panel, 1.0))
                .child(
                    MoonText::new("MoonScrollArea")
                        .uppercase(false)
                        .weight(700.0)
                        .render(),
                )
                .child(
                    MoonText::new("vertical overlay")
                        .uppercase(false)
                        .mono(true)
                        .color(p.text_soft)
                        .render(),
                )
                .child(
                    MoonText::new("horizontal overlay")
                        .uppercase(false)
                        .mono(true)
                        .color(p.text_soft)
                        .render(),
                )
                .child(
                    MoonText::new("always visible thumb")
                        .uppercase(false)
                        .mono(true)
                        .color(p.text_soft)
                        .render(),
                )
                .child(
                    MoonText::new("hover/drag states share tokens")
                        .uppercase(false)
                        .mono(true)
                        .color(p.text_soft)
                        .render(),
                ),
        )
        .child(
            div()
                .absolute()
                .top(px(8.0))
                .right(px(6.0))
                .bottom(px(16.0))
                .w(px(7.0))
                .rounded(px(999.0))
                .bg(rgba_from(p.panel_head, 0.34))
                .child(
                    div()
                        .absolute()
                        .top(px(22.0))
                        .w(px(7.0))
                        .h(px(52.0))
                        .rounded(px(999.0))
                        .bg(rgba_from(p.text_soft, 0.60)),
                ),
        )
        .child(
            div()
                .absolute()
                .left(px(10.0))
                .right(px(18.0))
                .bottom(px(6.0))
                .h(px(7.0))
                .rounded(px(999.0))
                .bg(rgba_from(p.panel_head, 0.34))
                .child(
                    div()
                        .absolute()
                        .left(px(74.0))
                        .h(px(7.0))
                        .w(px(116.0))
                        .rounded(px(999.0))
                        .bg(rgba_from(p.text_soft, 0.60)),
                ),
        )
}

pub(super) fn window_frame_row(
    frame: MoonWindowFrame,
    title: &'static str,
    cx: &App,
) -> impl IntoElement {
    let p = MoonPalette::active(cx);
    let header_h = frame.header_height_value();
    h_flex()
        .w_full()
        .h(px(header_h))
        .px(px(8.0))
        .rounded(px(5.0))
        .border_1()
        .border_color(rgba_from(p.border, 1.0))
        .bg(rgba_from(p.panel, 0.86))
        .child(frame.title_cluster(title, cx))
        .child(div().flex_1())
        .child(frame.visual_controls(cx))
}

#[cfg(test)]
mod tests;
