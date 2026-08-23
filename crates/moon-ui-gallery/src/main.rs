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
mod gallery_fixtures;
mod handoff;
mod launch;
#[cfg(feature = "snapshot")]
mod snapshot_capture;

use gallery_fixtures::{GalleryListDelegate, gallery_dock_panels, gallery_tab_panels, swatch};

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

#[cfg(feature = "snapshot")]
/// Clears existing PNG files through the snapshot capture module.
fn clear_snapshot_dir(dir: &std::path::Path) -> Result<(), String> {
    snapshot_capture::clear_snapshot_dir(dir)
}

#[cfg(feature = "snapshot")]
/// Captures the gallery window through the snapshot capture module.
fn snapshot_window_image(window: &mut Window) -> Result<image::RgbaImage, String> {
    snapshot_capture::snapshot_window_image(window)
}

/// Starts the interactive gallery or its command-line snapshot workflow.
fn main() {
    launch::run_gallery();
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
