//! Declarative metadata for deterministic design-handoff snapshot cases.

#[derive(Clone, Copy, Debug)]
/// Describes one deterministic design-handoff snapshot case.
pub(crate) struct HandoffCase {
    pub(crate) id: &'static str,
    pub(crate) width: f32,
    pub(crate) height: f32,
}

pub(super) const HANDOFF_CASES: &[HandoffCase] = &[
    HandoffCase {
        id: "theme.palette",
        width: 420.0,
        height: 132.0,
    },
    HandoffCase {
        id: "root.background_policy",
        width: 420.0,
        height: 132.0,
    },
    HandoffCase {
        id: "app.main.three_charts_scroll",
        width: 900.0,
        height: 620.0,
    },
    HandoffCase {
        id: "app.strategy_editor.selected",
        width: 900.0,
        height: 620.0,
    },
    HandoffCase {
        id: "icons.primitives",
        width: 260.0,
        height: 82.0,
    },
    HandoffCase {
        id: "avatar.group",
        width: 330.0,
        height: 82.0,
    },
    HandoffCase {
        id: "window.frame.main_full_logo",
        width: 560.0,
        height: 120.0,
    },
    HandoffCase {
        id: "window.frame.small_logo",
        width: 420.0,
        height: 120.0,
    },
    HandoffCase {
        id: "window.frame.popup_no_logo",
        width: 420.0,
        height: 120.0,
    },
    HandoffCase {
        id: "window.frame.detached_panel",
        width: 460.0,
        height: 120.0,
    },
    HandoffCase {
        id: "window.frame.detached_chart",
        width: 460.0,
        height: 120.0,
    },
    HandoffCase {
        id: "window.frame.debug",
        width: 460.0,
        height: 120.0,
    },
    HandoffCase {
        id: "surface.card",
        width: 320.0,
        height: 120.0,
    },
    HandoffCase {
        id: "surface.sidebar",
        width: 320.0,
        height: 150.0,
    },
    HandoffCase {
        id: "button.neutral",
        width: 240.0,
        height: 58.0,
    },
    HandoffCase {
        id: "button.hover",
        width: 240.0,
        height: 58.0,
    },
    HandoffCase {
        id: "button.active",
        width: 240.0,
        height: 58.0,
    },
    HandoffCase {
        id: "button.disabled",
        width: 240.0,
        height: 58.0,
    },
    HandoffCase {
        id: "button.blue",
        width: 240.0,
        height: 58.0,
    },
    HandoffCase {
        id: "button.green",
        width: 240.0,
        height: 58.0,
    },
    HandoffCase {
        id: "button.danger",
        width: 240.0,
        height: 58.0,
    },
    HandoffCase {
        id: "button.outline_amber",
        width: 240.0,
        height: 58.0,
    },
    HandoffCase {
        id: "button.micro",
        width: 240.0,
        height: 58.0,
    },
    HandoffCase {
        id: "button.action",
        width: 240.0,
        height: 58.0,
    },
    HandoffCase {
        id: "button.pill",
        width: 260.0,
        height: 62.0,
    },
    HandoffCase {
        id: "button.icon_slots",
        width: 300.0,
        height: 64.0,
    },
    HandoffCase {
        id: "button.variants_all",
        width: 520.0,
        height: 82.0,
    },
    HandoffCase {
        id: "input.default",
        width: 340.0,
        height: 62.0,
    },
    HandoffCase {
        id: "input.placeholder",
        width: 340.0,
        height: 62.0,
    },
    HandoffCase {
        id: "input.focus",
        width: 340.0,
        height: 62.0,
    },
    HandoffCase {
        id: "input.mask",
        width: 380.0,
        height: 74.0,
    },
    HandoffCase {
        id: "input.hotkey",
        width: 520.0,
        height: 170.0,
    },
    HandoffCase {
        id: "select.toolbar",
        width: 340.0,
        height: 62.0,
    },
    HandoffCase {
        id: "combobox.symbol",
        width: 360.0,
        height: 74.0,
    },
    HandoffCase {
        id: "color_picker.trigger",
        width: 320.0,
        height: 74.0,
    },
    HandoffCase {
        id: "textarea.memo",
        width: 360.0,
        height: 132.0,
    },
    HandoffCase {
        id: "form.row",
        width: 420.0,
        height: 80.0,
    },
    HandoffCase {
        id: "stepper.normal",
        width: 280.0,
        height: 68.0,
    },
    HandoffCase {
        id: "checkbox.checked",
        width: 260.0,
        height: 58.0,
    },
    HandoffCase {
        id: "checkbox.unchecked",
        width: 260.0,
        height: 58.0,
    },
    HandoffCase {
        id: "checkbox.compact",
        width: 260.0,
        height: 58.0,
    },
    HandoffCase {
        id: "checkbox.indeterminate",
        width: 260.0,
        height: 58.0,
    },
    HandoffCase {
        id: "radio.checked",
        width: 260.0,
        height: 58.0,
    },
    HandoffCase {
        id: "radio.unchecked",
        width: 260.0,
        height: 58.0,
    },
    HandoffCase {
        id: "rating.stars",
        width: 260.0,
        height: 58.0,
    },
    HandoffCase {
        id: "toggle.checked",
        width: 260.0,
        height: 58.0,
    },
    HandoffCase {
        id: "toggle.unchecked",
        width: 260.0,
        height: 58.0,
    },
    HandoffCase {
        id: "switch.checked",
        width: 260.0,
        height: 58.0,
    },
    HandoffCase {
        id: "slider.diffused.58",
        width: 300.0,
        height: 58.0,
    },
    HandoffCase {
        id: "slider.diffused.100",
        width: 300.0,
        height: 58.0,
    },
    HandoffCase {
        id: "slider.range",
        width: 300.0,
        height: 58.0,
    },
    HandoffCase {
        id: "progress.positive",
        width: 300.0,
        height: 58.0,
    },
    HandoffCase {
        id: "progress.loading",
        width: 300.0,
        height: 58.0,
    },
    HandoffCase {
        id: "progress.warning",
        width: 300.0,
        height: 58.0,
    },
    HandoffCase {
        id: "progress_circle.normal",
        width: 220.0,
        height: 70.0,
    },
    HandoffCase {
        id: "preset_strip.default",
        width: 420.0,
        height: 74.0,
    },
    HandoffCase {
        id: "tab_strip.default",
        width: 380.0,
        height: 68.0,
    },
    HandoffCase {
        id: "segmented.presets",
        width: 420.0,
        height: 68.0,
    },
    HandoffCase {
        id: "selector.pill",
        width: 360.0,
        height: 62.0,
    },
    HandoffCase {
        id: "breadcrumb.path",
        width: 380.0,
        height: 68.0,
    },
    HandoffCase {
        id: "pagination.basic",
        width: 420.0,
        height: 74.0,
    },
    HandoffCase {
        id: "table.basic",
        width: 410.0,
        height: 128.0,
    },
    HandoffCase {
        id: "table.primitives",
        width: 420.0,
        height: 104.0,
    },
    HandoffCase {
        id: "list.selected",
        width: 300.0,
        height: 132.0,
    },
    HandoffCase {
        id: "list.full",
        width: 340.0,
        height: 190.0,
    },
    HandoffCase {
        id: "virtual_list.basic",
        width: 360.0,
        height: 190.0,
    },
    HandoffCase {
        id: "tree.basic",
        width: 340.0,
        height: 190.0,
    },
    HandoffCase {
        id: "description_list.basic",
        width: 380.0,
        height: 120.0,
    },
    HandoffCase {
        id: "calendar.month",
        width: 280.0,
        height: 270.0,
    },
    HandoffCase {
        id: "date_picker.trigger",
        width: 340.0,
        height: 74.0,
    },
    HandoffCase {
        id: "date_time_picker.trigger",
        width: 340.0,
        height: 74.0,
    },
    HandoffCase {
        id: "date_time_picker.open",
        width: 340.0,
        height: 420.0,
    },
    HandoffCase {
        id: "time_picker.trigger",
        width: 340.0,
        height: 74.0,
    },
    HandoffCase {
        id: "time_picker.open",
        width: 340.0,
        height: 220.0,
    },
    HandoffCase {
        id: "pickers.parity",
        width: 340.0,
        height: 150.0,
    },
    HandoffCase {
        id: "dock.area",
        width: 520.0,
        height: 260.0,
    },
    HandoffCase {
        id: "tab_panel.default",
        width: 420.0,
        height: 190.0,
    },
    HandoffCase {
        id: "resizable.group",
        width: 420.0,
        height: 160.0,
    },
    HandoffCase {
        id: "scroll_area.overlay",
        width: 420.0,
        height: 180.0,
    },
    HandoffCase {
        id: "popup_menu.scale",
        width: 250.0,
        height: 162.0,
    },
    HandoffCase {
        id: "dropdown.open",
        width: 540.0,
        height: 340.0,
    },
    HandoffCase {
        id: "context_menu.basic",
        width: 300.0,
        height: 190.0,
    },
    HandoffCase {
        id: "popover.open",
        width: 320.0,
        height: 180.0,
    },
    HandoffCase {
        id: "hover_card.basic",
        width: 320.0,
        height: 150.0,
    },
    HandoffCase {
        id: "hover_card.open",
        width: 340.0,
        height: 170.0,
    },
    HandoffCase {
        id: "tooltip.default",
        width: 270.0,
        height: 88.0,
    },
    HandoffCase {
        id: "tooltip_view.entity",
        width: 300.0,
        height: 90.0,
    },
    HandoffCase {
        id: "dialog.confirm",
        width: 360.0,
        height: 150.0,
    },
    HandoffCase {
        id: "dialog.form",
        width: 380.0,
        height: 210.0,
    },
    HandoffCase {
        id: "sheet.trigger",
        width: 300.0,
        height: 90.0,
    },
    HandoffCase {
        id: "sheet.panel",
        width: 360.0,
        height: 220.0,
    },
    HandoffCase {
        id: "native_menu.trigger",
        width: 300.0,
        height: 90.0,
    },
    HandoffCase {
        id: "native_menu.fallback",
        width: 300.0,
        height: 170.0,
    },
    HandoffCase {
        id: "notification.info",
        width: 360.0,
        height: 110.0,
    },
    HandoffCase {
        id: "notification.toast",
        width: 380.0,
        height: 120.0,
    },
    HandoffCase {
        id: "alert.info",
        width: 420.0,
        height: 120.0,
    },
    HandoffCase {
        id: "accordion.basic",
        width: 420.0,
        height: 160.0,
    },
    HandoffCase {
        id: "collapsible.open",
        width: 420.0,
        height: 150.0,
    },
    HandoffCase {
        id: "group_box.basic",
        width: 420.0,
        height: 150.0,
    },
    HandoffCase {
        id: "sidebar.basic",
        width: 280.0,
        height: 260.0,
    },
    HandoffCase {
        id: "settings.page",
        width: 460.0,
        height: 260.0,
    },
    HandoffCase {
        id: "badge.variants",
        width: 330.0,
        height: 70.0,
    },
    HandoffCase {
        id: "tag.variants",
        width: 330.0,
        height: 70.0,
    },
    HandoffCase {
        id: "kbd.spinner.skeleton",
        width: 330.0,
        height: 76.0,
    },
    HandoffCase {
        id: "label.link.text",
        width: 360.0,
        height: 100.0,
    },
    HandoffCase {
        id: "separator.basic",
        width: 280.0,
        height: 90.0,
    },
    HandoffCase {
        id: "status_bar.basic",
        width: 460.0,
        height: 58.0,
    },
];
