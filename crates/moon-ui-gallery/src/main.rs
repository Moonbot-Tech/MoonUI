use std::{
    cell::{Cell, RefCell},
    path::PathBuf,
    rc::Rc,
    time::{Duration, Instant},
};

use gpui::prelude::*;
use gpui::{
    App, Bounds, Context, Entity, IntoElement, MouseButton, NoAction, ParentElement, Render,
    SharedString, Task, TitlebarOptions, Window, WindowBounds, WindowOptions, div, point, px, rgb,
    rgba, size, svg,
};
use gpui_platform::application;
use moon_ui::foundation::{box_shadow, selected_background};
use moon_ui::{
    DockArea, DockEvent, DockItem, IndexPath, MoonAccent, MoonAccordion, MoonAlert, MoonAvatar,
    MoonAvatarGroup, MoonAvatarSize, MoonBackgroundPolicy, MoonBadge, MoonBadgeSize,
    MoonBadgeVariant, MoonBreadcrumb, MoonBreadcrumbItem, MoonButton, MoonButtonIconSlot,
    MoonButtonSegment, MoonButtonSize, MoonButtonVariant, MoonCalendar, MoonCalendarState,
    MoonCheckbox, MoonCheckboxSize, MoonCollapsible, MoonColorPicker, MoonColorPickerState,
    MoonCombobox, MoonComboboxState, MoonComponentIndexPath, MoonContextMenu,
    MoonContextMenuWindowExt as _, MoonDataCell, MoonDataRow, MoonDataTable, MoonDataTableColumn,
    MoonDataTableState, MoonDatePicker, MoonDatePickerState, MoonDateTimePicker,
    MoonDateTimePickerState, MoonDescriptionList, MoonDisclosure, MoonDisclosureDirection,
    MoonDockPanel, MoonDropdown, MoonFormRow, MoonGroupBox, MoonHotkeyInput, MoonHoverCard,
    MoonInput, MoonInputMaskPattern, MoonKbd, MoonKbdSize, MoonLabel, MoonLink, MoonList,
    MoonListDelegate, MoonListItem, MoonListState, MoonMenuItem, MoonMenuSize, MoonNativeMenu,
    MoonNotification, MoonNumberFieldOptions, MoonPagination, MoonPalette, MoonPlacement,
    MoonPopover, MoonPopoverPlacement, MoonPopupMenu, MoonPresetItem, MoonPresetStrip,
    MoonProgress, MoonProgressCircle, MoonProgressCircleSize, MoonRadio, MoonRadioSize, MoonRating,
    MoonResizablePanelGroup, MoonScrollableElement, MoonScrollbarVisibility, MoonSearchableVec,
    MoonSegmentItem, MoonSegmentedControl, MoonSelect, MoonSelectItem, MoonSelectState,
    MoonSelectorPill, MoonSelectorSegment, MoonSeparator, MoonSettingField, MoonSettingGroup,
    MoonSettingItem, MoonSettingPage, MoonSettings, MoonSidebar, MoonSidebarGroup, MoonSidebarMenu,
    MoonSidebarMenuItem, MoonSidebarToggleButton, MoonSkeleton, MoonSlider, MoonSliderState,
    MoonSpinner, MoonSpinnerSize, MoonStatusBar, MoonStatusIndicator, MoonStatusItem, MoonStepper,
    MoonSurface, MoonSurfaceVariant, MoonSwitch, MoonTabItem, MoonTabStrip, MoonTableCell,
    MoonTableColumn, MoonTableRow, MoonTableStyle, MoonTag, MoonText, MoonTextArea, MoonTheme,
    MoonThemeConfig, MoonTimePicker, MoonTimePickerState, MoonToggle, MoonToggleSize, MoonTone,
    MoonTooltip, MoonTooltipPlacement, MoonTooltipSize, MoonTooltipView, MoonTree, MoonTreeItem,
    MoonTreeSelectionMode, MoonTreeState, MoonVirtualList, MoonVirtualListScrollHandle,
    MoonWindowExt as _, MoonWindowFrame, MoonWindowFrameBrand, MoonWindowFrameControls, PanelView,
    Root, TabPanel, ThemeMode, h_flex, moon_h_resizable, moon_resizable_panel, rgba_from, v_flex,
};

mod gallery;
mod handoff;

const COMPONENT_COVERAGE: &[&str] = &[
    "MoonRoot",
    "MoonBackgroundPolicy",
    "MoonAccordion",
    "MoonAlert",
    "MoonAvatar",
    "MoonAvatarGroup",
    "MoonButton",
    "MoonButtonSegment",
    "MoonButtonIconSlot",
    "MoonBadge",
    "MoonBreadcrumb",
    "MoonCheckbox",
    "MoonCollapsible",
    "MoonColorPicker",
    "MoonCombobox",
    "MoonContextMenu",
    "MoonDataTable",
    "MoonCalendar",
    "MoonDatePicker",
    "MoonDateTimePicker",
    "MoonTimePicker",
    "MoonDescriptionList",
    "MoonDialog",
    "MoonDisclosure",
    "MoonDockPanel",
    "DockArea",
    "TabPanel",
    "MoonDropdown",
    "MoonFormRow",
    "MoonGroupBox",
    "MoonHoverCard",
    "MoonPopupMenu",
    "MoonMenuItem",
    "MoonInput",
    "MoonInputMaskPattern",
    "MoonHotkeyInput",
    "MoonKbd",
    "MoonLabel",
    "MoonLink",
    "MoonList",
    "MoonNotification",
    "MoonPagination",
    "MoonPlacement",
    "MoonPopover",
    "MoonPresetStrip",
    "MoonProgress",
    "MoonProgressCircle",
    "MoonRadio",
    "MoonRating",
    "MoonResizablePanel",
    "MoonResizablePanelGroup",
    "MoonSegmentedControl",
    "MoonSelectorPill",
    "MoonSelect",
    "MoonSeparator",
    "MoonSettingField",
    "MoonSettingGroup",
    "MoonSettingItem",
    "MoonSettingPage",
    "MoonSettings",
    "MoonSheet",
    "MoonSidebar",
    "MoonSkeleton",
    "MoonSlider",
    "MoonSpinner",
    "MoonStatusBar",
    "MoonStepper",
    "MoonSurface",
    "MoonSwitch",
    "MoonTabStrip",
    "MoonTag",
    "MoonTableCell",
    "MoonTableColumn",
    "MoonTableRow",
    "MoonText",
    "MoonTextArea",
    "MoonTooltip",
    "MoonTooltipView",
    "MoonToggle",
    "MoonTree",
    "MoonVirtualList",
    "MoonWindowFrame",
    "MoonNativeMenu",
    "MoonPalette",
];

const GALLERY_PAGES: &[&str] = &[
    "Controls",
    "Inputs",
    "Data",
    "Overlays",
    "Layout",
    "NewControls",
    "Composites",
    "Stateful",
];

struct GalleryListDelegate {
    items: Vec<SharedString>,
    visible: Vec<usize>,
    selected: Option<MoonComponentIndexPath>,
}

impl GalleryListDelegate {
    fn new() -> Self {
        let items = [
            "Longbridge behavior",
            "Moon theme bridge",
            "Keyboard selection",
            "Virtualized rows",
            "Context-ready state",
            "Search delegate",
        ]
        .into_iter()
        .map(SharedString::from)
        .collect::<Vec<_>>();
        let visible = (0..items.len()).collect();
        Self {
            items,
            visible,
            selected: Some(MoonComponentIndexPath::new(1)),
        }
    }
}

impl MoonListDelegate for GalleryListDelegate {
    type Item = MoonListItem;

    fn perform_search(
        &mut self,
        query: &str,
        _window: &mut Window,
        cx: &mut Context<MoonListState<Self>>,
    ) -> Task<()> {
        let query = query.to_lowercase();
        self.visible = self
            .items
            .iter()
            .enumerate()
            .filter_map(|(ix, item)| item.to_lowercase().contains(&query).then_some(ix))
            .collect();
        if self
            .selected
            .is_some_and(|selected| selected.row >= self.visible.len())
        {
            self.selected = None;
        }
        cx.notify();
        Task::ready(())
    }

    fn items_count(&self, _section: usize, _cx: &App) -> usize {
        self.visible.len()
    }

    fn render_item(
        &mut self,
        ix: MoonComponentIndexPath,
        _window: &mut Window,
        _cx: &mut Context<MoonListState<Self>>,
    ) -> Option<Self::Item> {
        let item_ix = *self.visible.get(ix.row)?;
        let label = self.items.get(item_ix)?.clone();
        Some(
            MoonListItem::new(ix)
                .selected(self.selected == Some(ix))
                .child(label),
        )
    }

    fn set_selected_index(
        &mut self,
        ix: Option<MoonComponentIndexPath>,
        _window: &mut Window,
        cx: &mut Context<MoonListState<Self>>,
    ) {
        self.selected = ix;
        cx.notify();
    }
}

#[cfg(feature = "snapshot")]
fn clear_snapshot_dir(dir: &std::path::Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|err| format!("create {}: {err}", dir.display()))?;
    let entries = std::fs::read_dir(dir).map_err(|err| format!("read {}: {err}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|err| format!("read dir entry {}: {err}", dir.display()))?;
        if entry.path().extension().and_then(|ext| ext.to_str()) == Some("png") {
            std::fs::remove_file(entry.path())
                .map_err(|err| format!("remove {}: {err}", entry.path().display()))?;
        }
    }
    Ok(())
}

#[cfg(feature = "snapshot")]
fn snapshot_window_image(window: &mut Window) -> Result<image::RgbaImage, String> {
    match window.render_to_image() {
        Ok(image) => Ok(image),
        Err(err) => snapshot_window_image_fallback(window)
            .map_err(|fallback| format!("{err}; fallback failed: {fallback}")),
    }
}

#[cfg(all(feature = "snapshot", target_os = "windows"))]
fn snapshot_window_image_fallback(window: &Window) -> Result<image::RgbaImage, String> {
    use windows::Win32::Foundation::{HWND, LPARAM, POINT};
    use windows::Win32::Graphics::Gdi::{
        BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BitBlt, ClientToScreen, CreateCompatibleBitmap,
        CreateCompatibleDC, DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, GetDIBits, ReleaseDC,
        SRCCOPY, SelectObject,
    };
    use windows::Win32::System::Threading::GetCurrentProcessId;
    use windows::Win32::UI::WindowsAndMessaging::{
        BringWindowToTop, EnumWindows, GetClientRect, GetWindowThreadProcessId, HWND_NOTOPMOST,
        HWND_TOPMOST, SW_SHOW, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW, SetCursorPos,
        SetForegroundWindow, SetWindowPos, ShowWindow,
    };

    struct TopmostGuard {
        hwnd: Option<HWND>,
    }

    impl Drop for TopmostGuard {
        fn drop(&mut self) {
            if let Some(hwnd) = self.hwnd {
                unsafe {
                    let _ = SetWindowPos(
                        hwnd,
                        Some(HWND_NOTOPMOST),
                        0,
                        0,
                        0,
                        0,
                        SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW,
                    );
                }
            }
        }
    }

    struct EnumState {
        pid: u32,
        hwnd: Option<HWND>,
    }

    unsafe extern "system" fn enum_windows_proc(hwnd: HWND, lparam: LPARAM) -> windows::core::BOOL {
        let state = unsafe { &mut *(lparam.0 as *mut EnumState) };
        let mut pid = 0;
        unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
        if pid == state.pid {
            state.hwnd = Some(hwnd);
            return windows::core::BOOL(0);
        }
        windows::core::BOOL(1)
    }

    unsafe fn find_gallery_window() -> Option<HWND> {
        let mut state = EnumState {
            pid: unsafe { GetCurrentProcessId() },
            hwnd: None,
        };
        let state_ptr = &mut state as *mut EnumState;
        let _ = unsafe { EnumWindows(Some(enum_windows_proc), LPARAM(state_ptr as isize)) };
        state.hwnd
    }

    let mut topmost_guard = TopmostGuard { hwnd: None };
    let (x, y, width, height) = unsafe {
        match find_gallery_window() {
            Some(hwnd) => {
                topmost_guard.hwnd = Some(hwnd);
                let _ = ShowWindow(hwnd, SW_SHOW);
                let _ = SetWindowPos(
                    hwnd,
                    Some(HWND_TOPMOST),
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW,
                );
                let _ = BringWindowToTop(hwnd);
                let _ = SetForegroundWindow(hwnd);

                let mut rect = Default::default();
                if GetClientRect(hwnd, &mut rect).is_err() {
                    return Err("GetClientRect failed".to_string());
                }
                let mut origin = POINT { x: 0, y: 0 };
                if !ClientToScreen(hwnd, &mut origin).as_bool() {
                    return Err("ClientToScreen failed".to_string());
                }
                let width = (rect.right - rect.left).max(1);
                let height = (rect.bottom - rect.top).max(1);

                // The Windows fallback captures real desktop pixels. Keep the
                // cursor away from the taskbar so thumbnail previews or other
                // shell overlays cannot be baked into component snapshots.
                let _ = SetCursorPos(origin.x + 8, origin.y + 8);
                std::thread::sleep(std::time::Duration::from_millis(350));
                (origin.x, origin.y, width, height)
            }
            None => {
                let bounds = window.bounds();
                (
                    f32::from(bounds.origin.x).round() as i32,
                    f32::from(bounds.origin.y).round() as i32,
                    f32::from(bounds.size.width).round().max(1.0) as i32,
                    f32::from(bounds.size.height).round().max(1.0) as i32,
                )
            }
        }
    };

    unsafe {
        let screen = GetDC(None);
        if screen.is_invalid() {
            return Err("GetDC returned invalid HDC".to_string());
        }
        let memory = CreateCompatibleDC(Some(screen));
        if memory.is_invalid() {
            ReleaseDC(None, screen);
            return Err("CreateCompatibleDC returned invalid HDC".to_string());
        }
        let bitmap = CreateCompatibleBitmap(screen, width, height);
        if bitmap.is_invalid() {
            let _ = DeleteDC(memory);
            ReleaseDC(None, screen);
            return Err("CreateCompatibleBitmap returned invalid HBITMAP".to_string());
        }

        let previous = SelectObject(memory, bitmap.into());
        let bitblt_ok = BitBlt(memory, 0, 0, width, height, Some(screen), x, y, SRCCOPY).is_ok();
        let _ = SelectObject(memory, previous);
        if !bitblt_ok {
            let _ = DeleteObject(bitmap.into());
            let _ = DeleteDC(memory);
            ReleaseDC(None, screen);
            return Err("BitBlt failed".to_string());
        }

        let mut info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut pixels = vec![0_u8; (width as usize) * (height as usize) * 4];
        let lines = GetDIBits(
            memory,
            bitmap,
            0,
            height as u32,
            Some(pixels.as_mut_ptr().cast()),
            &mut info,
            DIB_RGB_COLORS,
        );

        let _ = DeleteObject(bitmap.into());
        let _ = DeleteDC(memory);
        ReleaseDC(None, screen);

        if lines == 0 {
            return Err("GetDIBits returned 0 lines".to_string());
        }

        for px in pixels.chunks_exact_mut(4) {
            px.swap(0, 2);
        }
        image::RgbaImage::from_raw(width as u32, height as u32, pixels)
            .ok_or_else(|| "image::RgbaImage::from_raw failed".to_string())
    }
}

#[cfg(all(feature = "snapshot", not(target_os = "windows")))]
fn snapshot_window_image_fallback(_window: &Window) -> Result<image::RgbaImage, String> {
    Err("no platform fallback; implement backend render_to_image for this target".to_string())
}

fn gallery_dock_panels() -> Vec<Rc<dyn PanelView>> {
    vec![
        // Log carries a tab suffix so the dock's badge slot is visible in the gallery: announcing
        // unread work on a tab is what `Panel::title_suffix` exists for. The dock asks only the
        // BACKGROUND tabs, so selecting Log makes its own badge go away.
        Rc::new(dock_panel("gallery-dock-orders", "Orders", MoonTone::Info)),
        Rc::new(
            dock_panel("gallery-dock-log", "Log", MoonTone::Warning).tab_suffix(|_, _| {
                MoonBadge::new("")
                    .count(7)
                    .size(MoonBadgeSize::Tiny)
                    .variant(MoonBadgeVariant::Solid)
                    .tone(MoonTone::Warning)
                    .render()
                    .into_any_element()
            }),
        ),
        Rc::new(dock_panel(
            "gallery-dock-assets",
            "Assets",
            MoonTone::Positive,
        )),
    ]
}

fn gallery_tab_panels() -> Vec<Rc<dyn PanelView>> {
    vec![
        Rc::new(dock_panel("gallery-tab-alpha", "Alpha", MoonTone::Accent)),
        Rc::new(dock_panel("gallery-tab-beta", "Beta", MoonTone::Info)),
    ]
}

fn dock_panel(name: &'static str, title: &'static str, tone: MoonTone) -> MoonDockPanel {
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
                MoonText::new("MoonDockPanel content with panel controls and background policy.")
                    .uppercase(false)
                    .mono(true)
                    .wrap()
                    .color(p.text_soft)
                    .render(),
            )
            .into_any_element()
    })
    .detachable(true)
    .show_dock_header(true)
    .closable(false)
    .zoomable(true)
    .background_policy(MoonBackgroundPolicy::Opaque)
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

fn run_gallery() {
    let args = gallery_args_from_cli();
    let initial_page = args.page;
    let snapshot_dir = args.snapshot_dir;
    let case_snapshot_dir = args.case_snapshot_dir;
    let snapshot_case_ids = args.snapshot_case_ids;
    let theme_mode = args.theme_mode;
    application()
        .with_assets(moon_ui::MoonAssets)
        .run(move |cx: &mut App| {
            moon_ui::foundation::init(cx);
            let mut theme_config = MoonThemeConfig::moon_terminal();
            theme_config.mode = theme_mode;
            MoonTheme::install_config(theme_config, cx);

            let p = MoonPalette::active(cx);
            if let Some(case_dir) = case_snapshot_dir.clone() {
                let first_case = handoff::first_handoff_case_for_ids(&snapshot_case_ids);
                let bounds =
                    Bounds::centered(None, size(px(first_case.width), px(first_case.height)), cx);
                cx.open_window(
                    WindowOptions {
                        window_bounds: Some(WindowBounds::Windowed(bounds)),
                        titlebar: Some(TitlebarOptions {
                            title: Some(SharedString::from("MoonUI Handoff Cases")),
                            appears_transparent: true,
                            traffic_light_position: None,
                        }),
                        window_clear_color: Some(rgba((p.shell << 8) | 0xFF)),
                        app_id: Some("pro.moonbot.moon-ui-handoff-cases".to_string()),
                        ..Default::default()
                    },
                    move |window, cx| {
                        let view = cx.new(|cx| {
                            handoff::CaseGallery::new(
                                window,
                                cx,
                                Some(case_dir.clone()),
                                snapshot_case_ids.clone(),
                                theme_mode,
                            )
                        });
                        cx.new(|cx| {
                            Root::new(view, window, cx)
                                .bordered(false)
                                .background_policy(MoonBackgroundPolicy::Opaque)
                                .background(MoonPalette::active(cx).shell)
                        })
                    },
                )
                .expect("open MoonUI handoff case window");
            } else {
                let bounds = Bounds::centered(None, size(px(1280.0), px(900.0)), cx);
                cx.open_window(
                    WindowOptions {
                        window_bounds: Some(WindowBounds::Windowed(bounds)),
                        titlebar: Some(TitlebarOptions {
                            title: Some(SharedString::from("MoonUI Gallery")),
                            appears_transparent: true,
                            traffic_light_position: None,
                        }),
                        window_clear_color: Some(rgba((p.shell << 8) | 0xFF)),
                        app_id: Some("pro.moonbot.moon-ui-gallery".to_string()),
                        ..Default::default()
                    },
                    move |window, cx| {
                        let view = cx.new(|cx| {
                            gallery::Gallery::new(
                                window,
                                cx,
                                initial_page,
                                snapshot_dir.clone(),
                                theme_mode,
                            )
                        });
                        cx.new(|cx| {
                            Root::new(view, window, cx)
                                .background_policy(MoonBackgroundPolicy::Opaque)
                                .background(MoonPalette::active(cx).shell)
                        })
                    },
                )
                .expect("open MoonUI gallery window");
            }
            cx.activate(true);
        });
}

fn main() {
    run_gallery();
}

#[derive(Clone)]
struct GalleryArgs {
    page: usize,
    snapshot_dir: Option<PathBuf>,
    case_snapshot_dir: Option<PathBuf>,
    snapshot_case_ids: Vec<String>,
    theme_mode: ThemeMode,
}

fn gallery_args_from_cli() -> GalleryArgs {
    let mut args = std::env::args().skip(1);
    let mut page = 0;
    let mut snapshot_dir = None;
    let mut case_snapshot_dir = None;
    let mut snapshot_case_ids = Vec::new();
    let mut theme_mode = ThemeMode::Dark;
    while let Some(arg) = args.next() {
        if arg == "--page" {
            if let Some(page_name) = args.next() {
                page = page_index(&page_name).unwrap_or(0);
            }
        } else if let Some(page_name) = arg.strip_prefix("--page=") {
            page = page_index(page_name).unwrap_or(0);
        } else if arg == "--snapshot-dir" {
            if let Some(dir) = args.next() {
                snapshot_dir = Some(PathBuf::from(dir));
            }
        } else if let Some(dir) = arg.strip_prefix("--snapshot-dir=") {
            snapshot_dir = Some(PathBuf::from(dir));
        } else if arg == "--snapshot-case-dir" {
            if let Some(dir) = args.next() {
                case_snapshot_dir = Some(PathBuf::from(dir));
            }
        } else if let Some(dir) = arg.strip_prefix("--snapshot-case-dir=") {
            case_snapshot_dir = Some(PathBuf::from(dir));
        } else if arg == "--snapshot-cases" {
            if let Some(cases) = args.next() {
                snapshot_case_ids.extend(parse_snapshot_case_ids(&cases));
            }
        } else if let Some(cases) = arg.strip_prefix("--snapshot-cases=") {
            snapshot_case_ids.extend(parse_snapshot_case_ids(cases));
        } else if arg == "--theme" {
            if let Some(mode) = args.next() {
                theme_mode = parse_theme_mode(&mode).unwrap_or(ThemeMode::Dark);
            }
        } else if let Some(mode) = arg.strip_prefix("--theme=") {
            theme_mode = parse_theme_mode(mode).unwrap_or(ThemeMode::Dark);
        }
    }
    if snapshot_dir.is_some() {
        page = 0;
    }
    GalleryArgs {
        page,
        snapshot_dir,
        case_snapshot_dir,
        snapshot_case_ids,
        theme_mode,
    }
}

fn parse_snapshot_case_ids(cases: &str) -> impl Iterator<Item = String> + '_ {
    cases
        .split(',')
        .map(str::trim)
        .filter(|case| !case.is_empty())
        .map(str::to_string)
}

fn page_index(page: &str) -> Option<usize> {
    GALLERY_PAGES
        .iter()
        .position(|candidate| candidate.eq_ignore_ascii_case(page))
}

fn parse_theme_mode(mode: &str) -> Option<ThemeMode> {
    match mode.to_ascii_lowercase().as_str() {
        "light" => Some(ThemeMode::Light),
        "dark" => Some(ThemeMode::Dark),
        "system" => Some(ThemeMode::System),
        _ => None,
    }
}

fn theme_mode_name(mode: ThemeMode) -> &'static str {
    match mode {
        ThemeMode::Light => "Light",
        ThemeMode::Dark => "Dark",
        ThemeMode::System => "System",
    }
}
