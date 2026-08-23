//! Shared interactive fixture state and rendering helpers for gallery views.

use super::*;

/// Provides searchable and selectable rows for the gallery list examples.
pub(super) struct GalleryListDelegate {
    items: Vec<SharedString>,
    visible: Vec<usize>,
    selected: Option<MoonComponentIndexPath>,
}

impl GalleryListDelegate {
    /// Creates the deterministic list fixture used by gallery and handoff views.
    pub(super) fn new() -> Self {
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

/// Creates the three panels used by the interactive dock examples.
pub(super) fn gallery_dock_panels() -> Vec<Rc<dyn PanelView>> {
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

/// Creates the two panels used by standalone tab examples.
pub(super) fn gallery_tab_panels() -> Vec<Rc<dyn PanelView>> {
    vec![
        Rc::new(dock_panel("gallery-tab-alpha", "Alpha", MoonTone::Accent)),
        Rc::new(dock_panel("gallery-tab-beta", "Beta", MoonTone::Info)),
    ]
}

/// Creates a reusable gallery dock panel with deterministic content and controls.
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

/// Renders a labeled palette swatch for the supplied RGB color.
pub(super) fn swatch(name: &'static str, color: u32) -> impl IntoElement {
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
