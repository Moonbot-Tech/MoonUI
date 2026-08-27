//! Live dock topology, panel lifecycle, persistence state, and user layout interactions.
//!
//! Name-based projections share topology while retaining each dock's local panel identities.

use std::{
    borrow::BorrowMut,
    collections::{HashMap, HashSet},
    rc::Rc,
    time::{Duration, Instant},
};

use gpui::prelude::FluentBuilder;
use gpui::*;
use serde::{Deserialize, Serialize};

use super::{
    background::MoonBackgroundPolicy,
    text::MoonText,
    theme::MoonTheme,
    tokens::{MoonPalette, rgba_from},
};

mod drag;
mod panel;
mod state;
mod tab_panel;
mod tree;

use drag::{
    DockResizeTarget, DockTabDrag, DockTileDrag, DockTileDragKind, DockTileDragStart,
    tab_interaction_policy,
};
use panel::MoonPanelRegistry;
pub use panel::{DockItem, MoonDockPanel, Panel, PanelView, register_panel};
use state::default_tile_meta;
pub use state::{
    DockAreaState, DockNamedLayout, DockState, DockTopologyByName, DockTopologyNode,
    DockTopologySide, PanelInfo, PanelState, TileMeta,
};

const DOCK_RESIZE_HIT_SIZE: f32 = 6.0;

/// Smallest gap between two repaints while a dock handle is being dragged.
///
/// See `DockArea::on_resize_drag_move`, which is the only user and carries the reasoning.
const RESIZE_NOTIFY_MIN_INTERVAL: Duration = Duration::from_millis(16);
const DOCK_MIN_SIDE_SIZE: f32 = 112.0;
const DOCK_MIN_CENTER_SIZE: f32 = 220.0;
const DOCK_MIN_BOTTOM_SIZE: f32 = 104.0;
const DOCK_TILE_MIN_W: f32 = 160.0;
const DOCK_TILE_MIN_H: f32 = 96.0;
const DOCK_TILE_SNAP: f32 = 4.0;

pub enum DockEvent {
    LayoutChanged,
    /// A successful dock interaction or named activation made this stable panel name active.
    ///
    /// User tab clicks and drag/drop moves emit this before their accompanying
    /// [`DockEvent::LayoutChanged`]. [`DockArea::activate_panel_by_name`] also emits it whenever
    /// the named panel is found, including when activation only repairs keyed runtime state.
    PanelActivated {
        panel_name: SharedString,
    },
    DetachRequested {
        panel_name: SharedString,
    },
    /// The close (×) button of a panel was clicked. The dock does NOT remove the panel
    /// itself — the host app decides what to do (e.g. move it back to its home tab strip
    /// instead of destroying it). Emitted instead of an internal `remove_panel_by_name`.
    PanelCloseRequested {
        panel_name: SharedString,
    },
    /// A dock tab was right-clicked, at `position` in window coordinates.
    ///
    /// The dock carries no menu of its own: what a tab offers on right-click is host policy
    /// (per-panel display switches, for instance), so the host opens its own context menu.
    /// Mirrors the `DetachRequested` route taken by a double-click on the same tab, `panel_name`
    /// included — like that event, it identifies the tab only as far as panel names are unique
    /// within the dock, which is what the registry-based hosts guarantee.
    TabContextMenu {
        panel_name: SharedString,
        position: Point<Pixels>,
    },
}

pub enum PanelEvent {
    LayoutChanged,
    ZoomIn,
    ZoomOut,
    Close,
    Detach,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DockPlacement {
    #[default]
    Center,
    Left,
    Right,
    Bottom,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DockSplitPlacement {
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum DockRoot {
    Center,
    Left,
    Right,
    Bottom,
}

/// Identifies how a named panel participates in one live dock node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DockPanelSlot {
    Panel,
    Tab(usize),
    Tile(usize),
}

/// Locates a named panel without exposing the dock's private topology to consumers.
#[derive(Clone, Debug, PartialEq, Eq)]
struct DockPanelLocation {
    root: DockRoot,
    path: Vec<usize>,
    slot: DockPanelSlot,
}

#[derive(Default)]
struct MoonTabPanelRuntimeState {
    active_ix: usize,
    /// What this tab group last announced through `Panel::set_active`, as `(front panel name, tab
    /// count)`, so the call stays an EDGE notification instead of a per-frame poll.
    ///
    /// The name identifies the front tab across an insertion or removal that shifts indices under a
    /// panel that never moved; the count is what catches a tab joining or leaving while the front
    /// tab stays put — that new tab has never been told it is hidden.
    notified_active: Option<(Option<SharedString>, usize)>,
}

#[derive(IntoElement)]
/// Renders one dock tab group and its active panel surface.
pub struct TabPanel {
    id: SharedString,
    items: Vec<Rc<dyn PanelView>>,
    active_ix: usize,
    dock_area: Option<WeakEntity<DockArea>>,
    dock_root: Option<DockRoot>,
    dock_path: Vec<usize>,
    background_policy: MoonBackgroundPolicy,
    content_background_policy: Option<MoonBackgroundPolicy>,
    header_background_policy: MoonBackgroundPolicy,
    show_header: bool,
    show_panel_controls: bool,
    layout_editable: bool,
    detach_allowed: bool,
    close_allowed: bool,
    pinned_leading_panels: Vec<SharedString>,
}

impl TabPanel {
    /// Create a tab group whose first item is active and whose layout controls are enabled.
    pub fn new(id: impl Into<SharedString>, items: Vec<Rc<dyn PanelView>>) -> Self {
        Self {
            id: id.into(),
            items,
            active_ix: 0,
            dock_area: None,
            dock_root: None,
            dock_path: Vec::new(),
            background_policy: MoonBackgroundPolicy::Opaque,
            content_background_policy: None,
            header_background_policy: MoonBackgroundPolicy::Opaque,
            show_header: true,
            show_panel_controls: true,
            layout_editable: true,
            detach_allowed: true,
            close_allowed: true,
            pinned_leading_panels: Vec::new(),
        }
    }

    pub fn active_index(mut self, active_ix: usize) -> Self {
        self.active_ix = active_ix;
        self
    }

    pub fn background_policy(mut self, policy: MoonBackgroundPolicy) -> Self {
        self.background_policy = policy;
        self
    }

    pub fn content_background_policy(mut self, policy: MoonBackgroundPolicy) -> Self {
        self.content_background_policy = Some(policy);
        self
    }

    pub fn header_background_policy(mut self, policy: MoonBackgroundPolicy) -> Self {
        self.header_background_policy = policy;
        self
    }

    pub fn show_header(mut self, show_header: bool) -> Self {
        self.show_header = show_header;
        self
    }

    pub fn show_panel_controls(mut self, show_panel_controls: bool) -> Self {
        self.show_panel_controls = show_panel_controls;
        self
    }

    /// Set whether tab drag and close affordances may edit the dock topology.
    fn layout_editable(mut self, layout_editable: bool) -> Self {
        self.layout_editable = layout_editable;
        self
    }

    /// Set whether this tab group exposes or accepts detach gestures.
    fn detach_allowed(mut self, detach_allowed: bool) -> Self {
        self.detach_allowed = detach_allowed;
        self
    }

    /// Set whether this tab group exposes user close controls.
    fn close_allowed(mut self, close_allowed: bool) -> Self {
        self.close_allowed = close_allowed;
        self
    }

    /// Mark stable panel names as emphasized non-draggable leading tabs.
    fn pinned_leading_panels(mut self, panel_names: Vec<SharedString>) -> Self {
        self.pinned_leading_panels = panel_names;
        self
    }

    fn dock_context(
        mut self,
        dock_area: WeakEntity<DockArea>,
        root: DockRoot,
        path: Vec<usize>,
    ) -> Self {
        self.dock_area = Some(dock_area);
        self.dock_root = Some(root);
        self.dock_path = path;
        self
    }
}

/// Fully resolved live layout assembled from local panel identities.
struct ResolvedDockLayout {
    center: DockItem,
    left: Option<(DockItem, f32, bool)>,
    right: Option<(DockItem, f32, bool)>,
    bottom: Option<(DockItem, f32, bool)>,
}

impl ResolvedDockLayout {
    /// Collect every resolved panel identity once in deterministic root order.
    fn unique_panels(&self) -> Vec<Rc<dyn PanelView>> {
        let mut panels = Vec::new();
        self.center.append_unique_panels(&mut panels);
        for side in [&self.left, &self.right, &self.bottom] {
            if let Some((item, _, _)) = side {
                item.append_unique_panels(&mut panels);
            }
        }
        panels
    }
}

impl DockArea {
    /// Report whether the raw tree repeats an identity or stable logical panel name.
    fn has_duplicate_panel_occurrences(&self, cx: &App) -> bool {
        let occurrences = self.center.panel_occurrence_count()
            + [&self.left, &self.right, &self.bottom]
                .into_iter()
                .flatten()
                .map(|(item, _, _)| item.panel_occurrence_count())
                .sum::<usize>();
        let panels = self.unique_panels();
        let unique_names = panels
            .iter()
            .map(|panel| panel.panel_name(cx).to_string())
            .collect::<HashSet<_>>();
        occurrences != panels.len() || unique_names.len() != panels.len()
    }

    /// Project resolved live parts to a canonical serializable topology.
    fn topology_for_parts(parts: &ResolvedDockLayout, cx: &App) -> DockTopologyByName {
        let side = |value: &Option<(DockItem, f32, bool)>| {
            value.as_ref().map(|(item, size, open)| DockTopologySide {
                item: item.topology_by_name(cx),
                size: *size,
                open: *open,
            })
        };
        DockTopologyByName {
            center: parts.center.topology_by_name(cx),
            left: side(&parts.left),
            right: side(&parts.right),
            bottom: side(&parts.bottom),
        }
        .normalized()
    }

    /// Resolve requested topology against current and explicitly supplied local panel identities.
    fn resolve_named_topology(
        &self,
        topology: &DockTopologyByName,
        additional_panels: Vec<Rc<dyn PanelView>>,
        retain_unmentioned: bool,
        cx: &App,
    ) -> ResolvedDockLayout {
        let topology = topology.normalized();
        let mut resolver = NamedPanelResolver::new(self.unique_panels(), additional_panels, cx);
        let mut center = resolver.resolve_node(&topology.center, cx);
        let mut resolve_side = |side: Option<&DockTopologySide>, minimum: f32| {
            let side = side?;
            let item = resolver.resolve_node(&side.item, cx);
            (!item.is_empty()).then(|| (item, side.size.max(minimum), side.open))
        };
        let left = resolve_side(topology.left.as_ref(), DOCK_MIN_SIDE_SIZE);
        let right = resolve_side(topology.right.as_ref(), DOCK_MIN_SIDE_SIZE);
        let bottom = resolve_side(topology.bottom.as_ref(), DOCK_MIN_BOTTOM_SIZE);
        if retain_unmentioned {
            center.append_repaired_panels(resolver.remaining());
        }
        ResolvedDockLayout {
            center,
            left,
            right,
            bottom,
        }
    }

    /// Collect active panel names for all tab groups, keyed by root and split path.
    fn active_tabs_for_parts(parts: &ResolvedDockLayout, cx: &App) -> HashMap<String, String> {
        let mut active = HashMap::new();
        Self::collect_active_tabs(
            &parts.center,
            DockRoot::Center,
            &mut Vec::new(),
            cx,
            &mut active,
        );
        for (root, side) in [
            (DockRoot::Left, &parts.left),
            (DockRoot::Right, &parts.right),
            (DockRoot::Bottom, &parts.bottom),
        ] {
            if let Some((item, _, _)) = side {
                Self::collect_active_tabs(item, root, &mut Vec::new(), cx, &mut active);
            }
        }
        active
    }

    /// Traverse tab groups and record each selected panel by stable name.
    fn collect_active_tabs(
        item: &DockItem,
        root: DockRoot,
        path: &mut Vec<usize>,
        cx: &App,
        active: &mut HashMap<String, String>,
    ) {
        match item {
            DockItem::Tabs { items, active_ix } => {
                if let Some(panel) = items.get((*active_ix).min(items.len().saturating_sub(1))) {
                    active.insert(
                        Self::split_key(root, path),
                        panel.panel_name(cx).to_string(),
                    );
                }
            }
            DockItem::Split { items, .. } => {
                for (ix, item) in items.iter().enumerate() {
                    path.push(ix);
                    Self::collect_active_tabs(item, root, path, cx, active);
                    path.pop();
                }
            }
            DockItem::Empty | DockItem::Panel(_) | DockItem::Tiles { .. } => {}
        }
    }

    /// Return all currently selected tab names independent of their topology paths.
    fn active_panel_names(&self, cx: &App) -> HashSet<String> {
        Self::active_tabs_for_parts(
            &ResolvedDockLayout {
                center: self.center.clone(),
                left: self.left.clone(),
                right: self.right.clone(),
                bottom: self.bottom.clone(),
            },
            cx,
        )
        .into_values()
        .collect()
    }

    /// Restore exact path-keyed tab activity into a newly resolved name-based layout.
    fn apply_named_active_tabs(
        parts: &mut ResolvedDockLayout,
        active: &HashMap<String, String>,
        cx: &App,
    ) {
        Self::apply_named_active_item(
            &mut parts.center,
            DockRoot::Center,
            &mut Vec::new(),
            active,
            cx,
        );
        for (root, side) in [
            (DockRoot::Left, &mut parts.left),
            (DockRoot::Right, &mut parts.right),
            (DockRoot::Bottom, &mut parts.bottom),
        ] {
            if let Some((item, _, _)) = side {
                Self::apply_named_active_item(item, root, &mut Vec::new(), active, cx);
            }
        }
    }

    /// Apply one exact active-tab name at each split path.
    fn apply_named_active_item(
        item: &mut DockItem,
        root: DockRoot,
        path: &mut Vec<usize>,
        active: &HashMap<String, String>,
        cx: &App,
    ) {
        match item {
            DockItem::Tabs { items, active_ix } => {
                if let Some(name) = active.get(&Self::split_key(root, path)) {
                    if let Some(ix) = items
                        .iter()
                        .position(|panel| panel.panel_name(cx).as_ref() == name)
                    {
                        *active_ix = ix;
                    }
                }
            }
            DockItem::Split { items, .. } => {
                for (ix, item) in items.iter_mut().enumerate() {
                    path.push(ix);
                    Self::apply_named_active_item(item, root, path, active, cx);
                    path.pop();
                }
            }
            DockItem::Empty | DockItem::Panel(_) | DockItem::Tiles { .. } => {}
        }
    }

    /// Preserve any locally active panel that remains inside a newly resolved tab group.
    fn apply_active_name_set(
        parts: &mut ResolvedDockLayout,
        active_names: &HashSet<String>,
        cx: &App,
    ) {
        fn apply(item: &mut DockItem, active_names: &HashSet<String>, cx: &App) {
            match item {
                DockItem::Tabs { items, active_ix } => {
                    if let Some(ix) = items
                        .iter()
                        .position(|panel| active_names.contains(panel.panel_name(cx).as_ref()))
                    {
                        *active_ix = ix;
                    }
                }
                DockItem::Split { items, .. } => {
                    for item in items {
                        apply(item, active_names, cx);
                    }
                }
                DockItem::Empty | DockItem::Panel(_) | DockItem::Tiles { .. } => {}
            }
        }
        apply(&mut parts.center, active_names, cx);
        for side in [&mut parts.left, &mut parts.right, &mut parts.bottom]
            .into_iter()
            .flatten()
        {
            apply(&mut side.0, active_names, cx);
        }
    }

    /// Reorder pinned panels within tabs and to the leading side of horizontal split ancestors.
    fn enforce_pinned_on_parts(&self, parts: &mut ResolvedDockLayout, cx: &App) -> bool {
        let mut changed = parts
            .center
            .enforce_pinned_leading(&self.pinned_leading_panels, cx);
        for side in [&mut parts.left, &mut parts.right, &mut parts.bottom]
            .into_iter()
            .flatten()
        {
            changed |= side
                .0
                .enforce_pinned_leading(&self.pinned_leading_panels, cx);
        }
        changed
    }

    /// Find a named panel in resolved parts without installing them into the dock.
    fn parts_find_panel(
        parts: &ResolvedDockLayout,
        panel_name: &str,
        cx: &App,
    ) -> Option<Rc<dyn PanelView>> {
        parts.center.find_panel_named(panel_name, cx).or_else(|| {
            [&parts.left, &parts.right, &parts.bottom]
                .into_iter()
                .flatten()
                .find_map(|(item, _, _)| item.find_panel_named(panel_name, cx))
        })
    }

    /// Install resolved local identities, reconcile lifecycle state, and emit one coherent event.
    fn install_resolved_layout(
        &mut self,
        parts: ResolvedDockLayout,
        zoomed_name: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let previous_panels = self.unique_panels();
        let next_panels = parts.unique_panels();
        let previous_zoomed = self
            .zoomed_panel
            .as_ref()
            .and_then(|name| self.find_panel_named(name, cx));
        let next_zoomed = zoomed_name
            .as_ref()
            .and_then(|name| Self::parts_find_panel(&parts, name, cx));
        let same_zoomed = previous_zoomed
            .as_ref()
            .zip(next_zoomed.as_ref())
            .is_some_and(|(previous, next)| Rc::ptr_eq(previous, next));

        // Removal callbacks may tear down zoom resources, so close the old zoom edge first.
        if !same_zoomed && let Some(panel) = previous_zoomed.as_ref() {
            panel.set_zoomed(false, window, cx.borrow_mut());
        }

        for panel in &previous_panels {
            if !next_panels.iter().any(|next| Rc::ptr_eq(panel, next)) {
                panel.set_active(false, window, cx.borrow_mut());
                panel.on_removed(window, cx.borrow_mut());
            }
        }

        self.center = parts.center;
        self.left = parts.left;
        self.right = parts.right;
        self.bottom = parts.bottom;
        self.zoomed_panel = zoomed_name.map(SharedString::from);
        self.tile_drag_start = None;

        let dock_area = cx.entity().downgrade();
        for panel in &next_panels {
            if !previous_panels
                .iter()
                .any(|previous| Rc::ptr_eq(previous, panel))
            {
                panel.on_added_to(dock_area.clone(), window, cx.borrow_mut());
            }
        }

        if !same_zoomed {
            if let Some(panel) = next_zoomed {
                panel.set_zoomed(true, window, cx.borrow_mut());
            }
        }
        self.sync_layout_active(window, cx);
        cx.emit(DockEvent::LayoutChanged);
        cx.notify();
    }
}

/// Resolves stable panel names to existing local identities without invoking panel factories.
struct NamedPanelResolver {
    panels: Vec<Rc<dyn PanelView>>,
    used: Vec<bool>,
}

impl NamedPanelResolver {
    /// Merge current and supplied pools while retaining one identity per stable panel name.
    fn new(current: Vec<Rc<dyn PanelView>>, additional: Vec<Rc<dyn PanelView>>, cx: &App) -> Self {
        let mut panels = Vec::new();
        for panel in current.into_iter().chain(additional) {
            let name = panel.panel_name(cx);
            if !panels
                .iter()
                .any(|known: &Rc<dyn PanelView>| known.panel_name(cx) == name)
            {
                panels.push(panel);
            }
        }
        let used = vec![false; panels.len()];
        Self { panels, used }
    }

    /// Consume the first unused local panel with the requested stable name.
    fn take_named(&mut self, name: &str, cx: &App) -> Option<Rc<dyn PanelView>> {
        let ix = self
            .panels
            .iter()
            .enumerate()
            .position(|(ix, panel)| !self.used[ix] && panel.panel_name(cx).as_ref() == name)?;
        self.used[ix] = true;
        Some(self.panels[ix].clone())
    }

    /// Resolve one normalized topology node, skipping names absent from the local pool.
    fn resolve_node(&mut self, node: &DockTopologyNode, cx: &App) -> DockItem {
        match node {
            DockTopologyNode::Empty => DockItem::Empty,
            DockTopologyNode::Panel { name } => self
                .take_named(name, cx)
                .map(DockItem::Panel)
                .unwrap_or(DockItem::Empty),
            DockTopologyNode::Tabs { names } => {
                let mut items = names
                    .iter()
                    .filter_map(|name| self.take_named(name, cx))
                    .collect::<Vec<_>>();
                match items.len() {
                    0 => DockItem::Empty,
                    1 => DockItem::Panel(items.remove(0)),
                    _ => DockItem::Tabs {
                        items,
                        active_ix: 0,
                    },
                }
            }
            DockTopologyNode::Tiles { names, metas } => {
                let mut items = Vec::new();
                let mut kept_metas = Vec::new();
                for (ix, name) in names.iter().enumerate() {
                    if let Some(panel) = self.take_named(name, cx) {
                        items.push(panel);
                        kept_metas.push(
                            metas
                                .get(ix)
                                .copied()
                                .unwrap_or_else(|| default_tile_meta(ix)),
                        );
                    }
                }
                match items.len() {
                    0 => DockItem::Empty,
                    1 => DockItem::Panel(items.remove(0)),
                    _ => DockItem::Tiles {
                        items,
                        metas: kept_metas,
                    },
                }
            }
            DockTopologyNode::Split {
                horizontal,
                items,
                sizes,
            } => {
                let mut resolved_items = Vec::new();
                let mut resolved_sizes = Vec::new();
                for (ix, item) in items.iter().enumerate() {
                    let item = self.resolve_node(item, cx);
                    if !item.is_empty() {
                        resolved_items.push(item);
                        resolved_sizes.push(sizes.get(ix).copied().flatten());
                    }
                }
                match resolved_items.len() {
                    0 => DockItem::Empty,
                    1 => resolved_items.remove(0),
                    _ => DockItem::Split {
                        horizontal: *horizontal,
                        items: resolved_items,
                        sizes: resolved_sizes,
                    },
                }
            }
        }
    }

    /// Return every local panel not named by the requested topology in stable pool order.
    fn remaining(&self) -> Vec<Rc<dyn PanelView>> {
        self.panels
            .iter()
            .zip(&self.used)
            .filter_map(|(panel, used)| (!*used).then(|| panel.clone()))
            .collect()
    }
}

/// Owns a live panel topology, its interaction state, and persistence-facing layout events.
pub struct DockArea {
    id: SharedString,
    version: Option<usize>,
    center: DockItem,
    left: Option<(DockItem, f32, bool)>,
    right: Option<(DockItem, f32, bool)>,
    bottom: Option<(DockItem, f32, bool)>,
    zoomed_panel: Option<SharedString>,
    background_policy: MoonBackgroundPolicy,
    tab_background_policy: MoonBackgroundPolicy,
    content_background_policy: Option<MoonBackgroundPolicy>,
    root_bounds: Bounds<Pixels>,
    row_bounds: Bounds<Pixels>,
    split_bounds: HashMap<String, Bounds<Pixels>>,
    tile_bounds: HashMap<String, Bounds<Pixels>>,
    /// Keyed tab runtimes registered during render so named activation can update them directly.
    tab_runtime_states: HashMap<String, WeakEntity<MoonTabPanelRuntimeState>>,
    tile_drag_start: Option<DockTileDragStart>,
    /// The handle currently held down, or `None` when no resize is in flight.
    ///
    /// A resize is driven by plain mouse events rather than by GPUI's drag machinery, and this
    /// is the whole state that makes it a gesture: see [`DockArea::resize_pointer_hook`]. The
    /// cursor rides along because the gesture outlives the pointer's stay over the thin handle
    /// that started it, and the axis is known only there.
    resize_active: Option<(DockResizeTarget, CursorStyle)>,
    /// When the last resize repaint was asked for; `None` outside a gesture.
    resize_notify_at: Option<Instant>,
    /// When false, slots do not expose split drop-zones — dragging a tab can only reorder
    /// it within a tab strip or move it into another existing tab strip, not create a new
    /// split anywhere (which lets panels land in e.g. a chart slot and wedge the layout).
    enable_split_drop: bool,
    /// Whether pointer-driven controls may change the topology; tab activation remains enabled.
    layout_editable: bool,
    /// Whether user detach requests are accepted independently of other structural edits.
    detach_allowed: bool,
    /// Whether user close requests are accepted independently of other structural edits.
    close_allowed: bool,
    /// Stable panel names that remain leading, emphasized, and non-draggable in tab groups.
    pinned_leading_panels: Vec<SharedString>,
}

impl EventEmitter<DockEvent> for DockArea {}

impl DockArea {
    /// Create an empty editable dock with the supplied persistence version.
    pub fn new(
        id: impl Into<SharedString>,
        version: Option<usize>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Self {
        Self {
            id: id.into(),
            version,
            center: DockItem::Empty,
            left: None,
            right: None,
            bottom: None,
            zoomed_panel: None,
            background_policy: MoonBackgroundPolicy::Opaque,
            tab_background_policy: MoonBackgroundPolicy::Opaque,
            content_background_policy: None,
            root_bounds: Bounds::default(),
            row_bounds: Bounds::default(),
            split_bounds: HashMap::new(),
            tile_bounds: HashMap::new(),
            tab_runtime_states: HashMap::new(),
            tile_drag_start: None,
            resize_active: None,
            resize_notify_at: None,
            enable_split_drop: true,
            layout_editable: true,
            detach_allowed: true,
            close_allowed: true,
            pinned_leading_panels: Vec::new(),
        }
    }

    #[cfg(test)]
    /// Create an editable dock around a supplied center item for structural unit tests.
    fn test_with_center(center: DockItem) -> Self {
        Self {
            id: "test-dock".into(),
            version: None,
            center,
            left: None,
            right: None,
            bottom: None,
            zoomed_panel: None,
            background_policy: MoonBackgroundPolicy::Opaque,
            tab_background_policy: MoonBackgroundPolicy::Opaque,
            content_background_policy: None,
            root_bounds: Bounds::default(),
            row_bounds: Bounds::default(),
            split_bounds: HashMap::new(),
            tile_bounds: HashMap::new(),
            tab_runtime_states: HashMap::new(),
            tile_drag_start: None,
            resize_active: None,
            resize_notify_at: None,
            enable_split_drop: true,
            layout_editable: true,
            detach_allowed: true,
            close_allowed: true,
            pinned_leading_panels: Vec::new(),
        }
    }

    /// Recreate a dock from serialized state through the registered panel factories.
    pub fn from_state(
        id: impl Into<SharedString>,
        state: DockAreaState,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let center = state.center.to_item(window, cx);
        let left = state
            .left_dock
            .map(|dock| (dock.panel.to_item(window, cx), dock.size, dock.open));
        let right = state
            .right_dock
            .map(|dock| (dock.panel.to_item(window, cx), dock.size, dock.open));
        let bottom = state
            .bottom_dock
            .map(|dock| (dock.panel.to_item(window, cx), dock.size, dock.open));

        Self {
            id: id.into(),
            version: state.version,
            center,
            left,
            right,
            bottom,
            zoomed_panel: None,
            background_policy: MoonBackgroundPolicy::Opaque,
            tab_background_policy: MoonBackgroundPolicy::Opaque,
            content_background_policy: None,
            root_bounds: Bounds::default(),
            row_bounds: Bounds::default(),
            split_bounds: HashMap::new(),
            tile_bounds: HashMap::new(),
            tab_runtime_states: HashMap::new(),
            tile_drag_start: None,
            resize_active: None,
            resize_notify_at: None,
            enable_split_drop: true,
            layout_editable: true,
            detach_allowed: true,
            close_allowed: true,
            pinned_leading_panels: Vec::new(),
        }
    }

    /// Load serialized state, recreate panels, and notify their addition lifecycle once.
    ///
    /// Loading replaces panel membership and requests a repaint, but does not emit
    /// [`DockEvent::LayoutChanged`].
    pub fn load(
        &mut self,
        state: DockAreaState,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        self.version = state.version;
        self.center = state.center.to_item(window, cx);
        self.left = state
            .left_dock
            .map(|dock| (dock.panel.to_item(window, cx), dock.size, dock.open));
        self.right = state
            .right_dock
            .map(|dock| (dock.panel.to_item(window, cx), dock.size, dock.open));
        self.bottom = state
            .bottom_dock
            .map(|dock| (dock.panel.to_item(window, cx), dock.size, dock.open));
        self.zoomed_panel = None;

        let dock_area = cx.entity().downgrade();
        self.center
            .notify_added(&dock_area, window, cx.borrow_mut());
        if let Some((item, _, _)) = &self.left {
            item.notify_added(&dock_area, window, cx.borrow_mut());
        }
        if let Some((item, _, _)) = &self.right {
            item.notify_added(&dock_area, window, cx.borrow_mut());
        }
        if let Some((item, _, _)) = &self.bottom {
            item.notify_added(&dock_area, window, cx.borrow_mut());
        }
        cx.notify();
        Ok(())
    }

    /// Project the current layout to normalized serializable topology by stable panel name.
    pub fn topology_by_name(&self, cx: &App) -> DockTopologyByName {
        Self::topology_for_parts(
            &ResolvedDockLayout {
                center: self.center.clone(),
                left: self.left.clone(),
                right: self.right.clone(),
                bottom: self.bottom.clone(),
            },
            cx,
        )
    }

    /// Capture full local topology, active tabs, and zoom by name without retaining panel owners.
    pub fn named_layout(&self, cx: &App) -> DockNamedLayout {
        let parts = ResolvedDockLayout {
            center: self.center.clone(),
            left: self.left.clone(),
            right: self.right.clone(),
            bottom: self.bottom.clone(),
        };
        DockNamedLayout {
            topology: Self::topology_for_parts(&parts, cx),
            active_tabs: Self::active_tabs_for_parts(&parts, cx),
            zoomed_panel: self.zoomed_panel.as_ref().map(ToString::to_string),
        }
    }

    /// Apply shared topology onto local panel identities and repair stale panel-name references.
    ///
    /// Unknown and duplicate requested names are discarded. The first live identity for each
    /// stable panel name wins across this dock and `additional_panels`; unrequested names are
    /// appended deterministically to the center. Local
    /// active-tab and zoom state are preserved where their named panels survive. Returns `true`
    /// only when live topology actually changes and one [`DockEvent::LayoutChanged`] is emitted.
    pub fn apply_topology_by_name(
        &mut self,
        topology: &DockTopologyByName,
        additional_panels: Vec<Rc<dyn PanelView>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let active_names = self.active_panel_names(cx);
        let zoomed_panel = self.zoomed_panel.as_ref().map(ToString::to_string);
        let mut resolved = self.resolve_named_topology(topology, additional_panels, true, cx);
        Self::apply_active_name_set(&mut resolved, &active_names, cx);
        self.enforce_pinned_on_parts(&mut resolved, cx);

        let next_topology = Self::topology_for_parts(&resolved, cx);
        if self.topology_by_name(cx) == next_topology && !self.has_duplicate_panel_occurrences(cx) {
            return false;
        }
        self.install_resolved_layout(resolved, zoomed_panel, window, cx);
        true
    }

    /// Restore an exact local name-based layout, including active tabs and zoom state.
    ///
    /// Current or supplied panels absent from the captured layout are removed from this dock;
    /// named panels reuse their existing local `Rc` identity. Missing names are skipped safely.
    /// Returns `true` only when topology, active tabs, zoom, or panel membership changes.
    pub fn apply_named_layout(
        &mut self,
        layout: &DockNamedLayout,
        additional_panels: Vec<Rc<dyn PanelView>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let mut resolved =
            self.resolve_named_topology(&layout.topology, additional_panels, false, cx);
        Self::apply_named_active_tabs(&mut resolved, &layout.active_tabs, cx);
        self.enforce_pinned_on_parts(&mut resolved, cx);
        let zoomed_panel = layout
            .zoomed_panel
            .as_ref()
            .filter(|name| Self::parts_find_panel(&resolved, name, cx).is_some())
            .cloned();
        let effective = DockNamedLayout {
            topology: Self::topology_for_parts(&resolved, cx),
            active_tabs: Self::active_tabs_for_parts(&resolved, cx),
            zoomed_panel: zoomed_panel.clone(),
        };
        if self.named_layout(cx) == effective && !self.has_duplicate_panel_occurrences(cx) {
            return false;
        }
        self.install_resolved_layout(resolved, zoomed_panel, window, cx);
        true
    }

    /// Activate a named panel, clearing zoom and synchronizing all activation stores.
    ///
    /// Returns `false` when the panel is absent. A found panel returns `true` even when it was
    /// already active, because its keyed runtime and [`Panel::set_active`] state are repaired. Any
    /// zoomed surface is cleared first so the requested normal-topology panel becomes visible.
    /// A found panel always emits [`DockEvent::PanelActivated`] with its exact stable name, then
    /// emits one [`DockEvent::LayoutChanged`] only when zoom, side-root visibility, or the
    /// topology's active tab changes. Activation never adds, removes, or recreates panels.
    pub fn activate_panel_by_name(
        &mut self,
        panel_name: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(location) = self.panel_location(panel_name, cx) else {
            return false;
        };

        let previous_zoomed = self.zoomed_panel.take();
        let zoom_cleared = previous_zoomed.is_some();
        if let Some(previous_zoomed) = previous_zoomed {
            if let Some(panel) = self.find_panel_named(previous_zoomed.as_ref(), cx) {
                panel.set_zoomed(false, window, cx.borrow_mut());
            }
        }
        let root_opened = self.open_root(location.root);
        let mut layout_changed = zoom_cleared || root_opened;
        if let DockPanelSlot::Tab(active_ix) = location.slot {
            layout_changed |= self.set_tabs_active_index(location.root, &location.path, active_ix);
        }

        if zoom_cleared {
            self.sync_layout_active(window, cx);
        } else if root_opened {
            if let Some(item) = self.root_item(location.root) {
                self.sync_item_active(location.root, &[], item, window, cx);
            }
        } else if let Some(item) = self
            .root_item(location.root)
            .and_then(|item| Self::item_at_path(item, &location.path))
        {
            self.sync_item_active(location.root, &location.path, item, window, cx);
        }
        cx.emit(DockEvent::PanelActivated {
            panel_name: panel_name.to_string().into(),
        });
        if layout_changed {
            cx.emit(DockEvent::LayoutChanged);
        }
        cx.notify();
        true
    }

    /// Enable or suppress pointer-driven structural layout edits.
    ///
    /// Programmatic layout APIs and user tab activation remain available while editing is locked.
    pub fn set_layout_editable(&mut self, editable: bool, cx: &mut Context<Self>) {
        if self.layout_editable == editable {
            return;
        }
        self.layout_editable = editable;
        self.tile_drag_start = None;
        cx.notify();
    }

    /// Independently allow or reject user detach requests while retaining other layout edits.
    pub fn set_detach_allowed(&mut self, allowed: bool, cx: &mut Context<Self>) {
        if self.detach_allowed == allowed {
            return;
        }
        self.detach_allowed = allowed;
        cx.notify();
    }

    /// Independently allow or reject user close requests while retaining other layout edits.
    pub fn set_close_allowed(&mut self, allowed: bool, cx: &mut Context<Self>) {
        if self.close_allowed == allowed {
            return;
        }
        self.close_allowed = allowed;
        cx.notify();
    }

    /// Configure stable panel names that lead and receive emphasis in every tab group.
    ///
    /// Duplicate names are removed in caller order. Existing tab groups are reordered immediately
    /// while retaining their active panel identity. Returns whether live topology changed.
    pub fn set_pinned_leading_panels(
        &mut self,
        panel_names: Vec<SharedString>,
        cx: &mut Context<Self>,
    ) -> bool {
        let mut unique = Vec::new();
        for name in panel_names {
            if !name.is_empty() && !unique.iter().any(|known: &SharedString| known == &name) {
                unique.push(name);
            }
        }
        if self.pinned_leading_panels == unique {
            return false;
        }
        self.pinned_leading_panels = unique;
        let mut changed = self
            .center
            .enforce_pinned_leading(&self.pinned_leading_panels, cx);
        for side in [&mut self.left, &mut self.right, &mut self.bottom]
            .into_iter()
            .flatten()
        {
            changed |= side
                .0
                .enforce_pinned_leading(&self.pinned_leading_panels, cx);
        }
        if changed {
            cx.emit(DockEvent::LayoutChanged);
        }
        cx.notify();
        changed
    }

    /// Emit a detach request only if both layout editing and detachment remain enabled.
    fn request_detach_from_user(&self, panel_name: SharedString, cx: &mut Context<Self>) {
        if self.layout_editable && self.detach_allowed {
            cx.emit(DockEvent::DetachRequested { panel_name });
        }
    }

    /// Emit a close request only if both layout editing and closing remain enabled.
    fn request_close_from_user(&self, panel_name: SharedString, cx: &mut Context<Self>) {
        if self.layout_editable && self.close_allowed {
            cx.emit(DockEvent::PanelCloseRequested { panel_name });
        }
    }

    pub fn background_policy(mut self, policy: MoonBackgroundPolicy) -> Self {
        self.background_policy = policy;
        self
    }

    /// Disable split drop-zones (default enabled). With this off, dragging a tab can only
    /// reorder within / move between existing tab strips — it cannot create a new split
    /// (so panels can't be dropped into a chart slot and wedge the layout).
    pub fn enable_split_drop(mut self, enable: bool) -> Self {
        self.enable_split_drop = enable;
        self
    }

    pub fn tab_background_policy(mut self, policy: MoonBackgroundPolicy) -> Self {
        self.tab_background_policy = policy;
        self
    }

    pub fn content_background_policy(mut self, policy: MoonBackgroundPolicy) -> Self {
        self.content_background_policy = Some(policy);
        self
    }

    /// Replace the center topology and notify the supplied panels that they belong to this dock.
    ///
    /// This emits one [`DockEvent::LayoutChanged`] and requests a repaint. It does not run removal
    /// lifecycle hooks for the previous center item.
    pub fn set_center(&mut self, item: DockItem, window: &mut Window, cx: &mut Context<Self>) {
        item.notify_added(&cx.entity().downgrade(), window, cx.borrow_mut());
        self.center = item;
        cx.emit(DockEvent::LayoutChanged);
        cx.notify();
    }

    /// Add one panel to a dock root, opening side roots and preserving existing panels as tabs.
    ///
    /// The panel receives one addition lifecycle callback. The change emits one
    /// [`DockEvent::LayoutChanged`] and requests a repaint.
    pub fn add_panel(
        &mut self,
        panel: Rc<dyn PanelView>,
        placement: DockPlacement,
        size: Option<f32>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        panel.on_added_to(cx.entity().downgrade(), window, cx.borrow_mut());
        let item = DockItem::panel(panel.clone());
        match placement {
            DockPlacement::Center => {
                self.center =
                    std::mem::replace(&mut self.center, DockItem::Empty).with_panel_added(panel);
            }
            DockPlacement::Left => {
                let item = if let Some((existing, _, _)) = self.left.take() {
                    existing.with_panel_added(panel)
                } else {
                    item
                };
                self.left = Some((item, size.unwrap_or(220.0), true));
            }
            DockPlacement::Right => {
                let item = if let Some((existing, _, _)) = self.right.take() {
                    existing.with_panel_added(panel)
                } else {
                    item
                };
                self.right = Some((item, size.unwrap_or(280.0), true));
            }
            DockPlacement::Bottom => {
                let item = if let Some((existing, _, _)) = self.bottom.take() {
                    existing.with_panel_added(panel)
                } else {
                    item
                };
                self.bottom = Some((item, size.unwrap_or(160.0), true));
            }
        }
        cx.emit(DockEvent::LayoutChanged);
        cx.notify();
    }

    /// Restore `panel` into its "home" tab strip — the `Tabs` node inside `center` that
    /// already holds one of `sibling_names` — at `ix` (clamped). Unlike `add_panel(Center)`,
    /// this does NOT collapse the surrounding split. Returns false if no such strip exists
    /// (e.g. every sibling is detached); the caller may then fall back to `add_panel`.
    /// Success runs the addition lifecycle callback, emits one [`DockEvent::LayoutChanged`], and
    /// requests a repaint.
    pub fn insert_panel_into_home_tabs(
        &mut self,
        panel: Rc<dyn PanelView>,
        ix: usize,
        sibling_names: &[&str],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let weak = cx.entity().downgrade();
        let ok = self.center.insert_into_named_tabs(
            panel,
            ix,
            sibling_names,
            &weak,
            window,
            cx.borrow_mut(),
        );
        if ok {
            cx.emit(DockEvent::LayoutChanged);
            cx.notify();
        }
        ok
    }

    /// Restore `panel` back BESIDE its former split neighbours (side-by-side / stacked), instead
    /// of merging it into a tab strip. Handles arbitrary NESTED splits (rows of columns etc.).
    ///
    /// `sibling_names` — every panel that shared the returning panel's immediate parent split (any
    /// present one anchors that split). `slot_panels` — the panels of the ADJACENT slot the panel
    /// sat next to (that slot may itself be a nested split). `index`/`placement` — the panel's
    /// former position & side; `panel_size`/`sibling_size` — the pre-detach pixel slot sizes
    /// (`None` = flex) so it returns at its old proportion.
    ///
    /// Two outcomes:
    /// - the parent split of the SAME orientation still exists (panel was one of 3+ members) →
    ///   insert `panel` as a new member at `index`, in place;
    /// - that split collapsed (panel was one of two) → wrap the WHOLE adjacent slot subtree (the
    ///   smallest node holding all present `slot_panels`, so a nested column-stack is wrapped as a
    ///   unit, not one leaf inside it) into a fresh split with `panel` on `placement`.
    ///
    /// Returns false if neither anchor nor slot survives — the caller falls back to tab restore.
    /// Success runs the addition lifecycle callback, emits one [`DockEvent::LayoutChanged`], and
    /// requests a repaint.
    pub fn insert_panel_beside_sibling(
        &mut self,
        panel: Rc<dyn PanelView>,
        sibling_names: &[&str],
        slot_panels: &[&str],
        index: usize,
        placement: DockSplitPlacement,
        panel_size: Option<f32>,
        sibling_size: Option<f32>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        // Orientation implied by the target side: Left/Right → horizontal split, Top/Bottom → vertical.
        let want_horizontal = matches!(
            placement,
            DockSplitPlacement::Left | DockSplitPlacement::Right
        );

        // Case 1: an anchor sibling still sits inside a Split of the SAME orientation — that split
        // survived (3+ members). Insert `panel` as a new member at `index`, in place.
        let mut anchor: Option<(DockRoot, Vec<usize>)> = None;
        'roots: for root in [
            DockRoot::Bottom,
            DockRoot::Center,
            DockRoot::Left,
            DockRoot::Right,
        ] {
            if let Some(item) = self.root_item_mut(root) {
                for name in sibling_names {
                    if let Some(path) = item.find_panel_path(name, cx) {
                        anchor = Some((root, path));
                        break 'roots;
                    }
                }
            }
        }
        if let Some((root, path)) = &anchor {
            if let Some((_, parent_path)) = path.split_last() {
                let parent_matches = self
                    .root_item_mut(*root)
                    .and_then(|it| Self::item_at_path_mut(it, parent_path))
                    .map(|p| matches!(p, DockItem::Split { horizontal, .. } if *horizontal == want_horizontal))
                    .unwrap_or(false);
                if parent_matches {
                    let weak = cx.entity().downgrade();
                    panel.on_added_to(weak, window, cx);
                    if let Some(DockItem::Split { items, sizes, .. }) = self
                        .root_item_mut(*root)
                        .and_then(|it| Self::item_at_path_mut(it, parent_path))
                    {
                        let ix = index.min(items.len());
                        items.insert(ix, DockItem::Panel(panel));
                        // Keep `sizes` aligned with `items`; give the returning slot its former size.
                        while sizes.len() + 1 < items.len() {
                            sizes.push(None);
                        }
                        sizes.insert(ix.min(sizes.len()), panel_size);
                    }
                    cx.emit(DockEvent::LayoutChanged);
                    cx.notify();
                    return true;
                }
            }
        }

        // Case 2: wrap the whole adjacent slot subtree (smallest node holding all present
        // `slot_panels`) into a fresh split with `panel` on `placement`.
        let mut wrap: Option<(DockRoot, Vec<usize>)> = None;
        for root in [
            DockRoot::Bottom,
            DockRoot::Center,
            DockRoot::Left,
            DockRoot::Right,
        ] {
            if let Some(item) = self.root_item_mut(root) {
                if let Some(path) = item.smallest_subtree_with_all(slot_panels, cx) {
                    wrap = Some((root, path));
                    break;
                }
            }
        }
        let Some((root, path)) = wrap else {
            return false;
        };
        let weak = cx.entity().downgrade();
        panel.on_added_to(weak, window, cx);
        let ok = self.split_item_with_panel(root, &path, placement, panel);
        if ok {
            // `split_item_with_panel` orders the new panel first for Left/Top, second for
            // Right/Bottom, with even (empty) sizes. Overwrite with the remembered proportions
            // in that same order so the returning panel reclaims its former slot size.
            if panel_size.is_some() || sibling_size.is_some() {
                if let Some(DockItem::Split { sizes, .. }) = self
                    .root_item_mut(root)
                    .and_then(|it| Self::item_at_path_mut(it, &path))
                {
                    let panel_first = matches!(
                        placement,
                        DockSplitPlacement::Left | DockSplitPlacement::Top
                    );
                    *sizes = if panel_first {
                        vec![panel_size, sibling_size]
                    } else {
                        vec![sibling_size, panel_size]
                    };
                }
            }
            cx.emit(DockEvent::LayoutChanged);
            cx.notify();
        }
        ok
    }

    /// Remove a named panel from any root and run its removal lifecycle callback.
    ///
    /// A successful removal also clears matching zoom state, emits one
    /// [`DockEvent::LayoutChanged`], requests a repaint, and returns `true`.
    /// An absent name returns `false` without those effects.
    pub fn remove_panel_by_name(
        &mut self,
        panel_name: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let (center, removed_center) = std::mem::replace(&mut self.center, DockItem::Empty)
            .remove_panel_named(panel_name, window, cx.borrow_mut());
        self.center = center;

        let mut removed = removed_center;
        self.left = self.left.take().and_then(|(item, size, open)| {
            let (item, did_remove) = item.remove_panel_named(panel_name, window, cx.borrow_mut());
            removed |= did_remove;
            (!item.is_empty()).then_some((item, size, open))
        });
        self.right = self.right.take().and_then(|(item, size, open)| {
            let (item, did_remove) = item.remove_panel_named(panel_name, window, cx.borrow_mut());
            removed |= did_remove;
            (!item.is_empty()).then_some((item, size, open))
        });
        self.bottom = self.bottom.take().and_then(|(item, size, open)| {
            let (item, did_remove) = item.remove_panel_named(panel_name, window, cx.borrow_mut());
            removed |= did_remove;
            (!item.is_empty()).then_some((item, size, open))
        });

        if removed {
            if self.zoomed_panel.as_ref().map(|name| name.as_ref()) == Some(panel_name) {
                self.zoomed_panel = None;
            }
            cx.emit(DockEvent::LayoutChanged);
            cx.notify();
        }
        removed
    }

    /// Extract every live occurrence of a stable panel name and return its canonical identity.
    ///
    /// The center, left, right, and bottom roots are traversed in that order. The first distinct
    /// `Rc` is returned for later restoration, while every matching occurrence is removed. Zoom is
    /// closed before removal, and inactive/removal lifecycle callbacks run exactly once per unique
    /// removed identity. A successful take emits one [`DockEvent::LayoutChanged`] and requests a
    /// repaint; an absent name has no effects.
    ///
    /// Args:
    ///     panel_name: Stable logical name to extract from every dock root.
    ///     window: Window used for zoom, activation, and removal lifecycle callbacks.
    ///     cx: Dock context used to emit the layout event and request a repaint.
    ///
    /// Returns:
    ///     The first unique removed identity in root traversal order, or `None` when absent.
    pub fn take_panel_by_name(
        &mut self,
        panel_name: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<Rc<dyn PanelView>> {
        self.find_panel_named(panel_name, cx)?;
        let panels = self.take_panels_by_name_from_all_roots(panel_name, cx);
        let canonical = panels.first()?.clone();

        if self.zoomed_panel.as_ref().map(|name| name.as_ref()) == Some(panel_name) {
            self.zoomed_panel = None;
            canonical.set_zoomed(false, window, cx.borrow_mut());
        }
        for panel in &panels {
            panel.set_active(false, window, cx.borrow_mut());
            panel.on_removed(window, cx.borrow_mut());
        }
        self.sync_layout_active(window, cx);
        cx.emit(DockEvent::LayoutChanged);
        cx.notify();
        Some(canonical)
    }

    pub fn set_dock_open(
        &mut self,
        placement: DockPlacement,
        open: bool,
        cx: &mut Context<Self>,
    ) -> bool {
        let slot = match placement {
            DockPlacement::Center => return false,
            DockPlacement::Left => self.left.as_mut(),
            DockPlacement::Right => self.right.as_mut(),
            DockPlacement::Bottom => self.bottom.as_mut(),
        };
        let Some((_, _, current_open)) = slot else {
            return false;
        };
        if *current_open == open {
            return false;
        }
        *current_open = open;
        cx.emit(DockEvent::LayoutChanged);
        cx.notify();
        true
    }

    pub fn toggle_dock(&mut self, placement: DockPlacement, cx: &mut Context<Self>) -> bool {
        let open = match placement {
            DockPlacement::Center => return false,
            DockPlacement::Left => self.left.as_ref().map(|(_, _, open)| !*open),
            DockPlacement::Right => self.right.as_ref().map(|(_, _, open)| !*open),
            DockPlacement::Bottom => self.bottom.as_ref().map(|(_, _, open)| !*open),
        };
        open.map(|open| self.set_dock_open(placement, open, cx))
            .unwrap_or(false)
    }

    /// Toggle one panel as the sole visible active surface and notify one layout change.
    pub fn toggle_zoom_panel(
        &mut self,
        panel_name: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let panel_name = panel_name.into();
        let next = if self.zoomed_panel.as_ref() == Some(&panel_name) {
            None
        } else {
            Some(panel_name.clone())
        };
        if let Some(current) = self.zoomed_panel.as_ref() {
            if let Some(panel) = self.find_panel_named(current.as_ref(), cx) {
                panel.set_zoomed(false, window, cx.borrow_mut());
            }
        }
        if let Some(next_name) = next.as_ref() {
            if let Some(panel) = self.find_panel_named(next_name.as_ref(), cx) {
                panel.set_zoomed(true, window, cx.borrow_mut());
            }
        }
        self.zoomed_panel = next;
        self.sync_layout_active(window, cx);
        cx.emit(DockEvent::LayoutChanged);
        cx.notify();
    }

    /// Return from a zoomed surface to the active normal topology.
    pub fn clear_zoom(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(current) = self.zoomed_panel.take() {
            if let Some(panel) = self.find_panel_named(current.as_ref(), cx) {
                panel.set_zoomed(false, window, cx.borrow_mut());
            }
            self.sync_layout_active(window, cx);
            cx.emit(DockEvent::LayoutChanged);
            cx.notify();
        }
    }

    pub fn dump(&self, cx: &App) -> DockAreaState {
        DockAreaState {
            version: self.version,
            center: self.center.dump(cx),
            left_dock: self.left.as_ref().map(|(panel, size, open)| DockState {
                panel: panel.dump(cx),
                placement: DockPlacement::Left,
                size: *size,
                open: *open,
            }),
            right_dock: self.right.as_ref().map(|(panel, size, open)| DockState {
                panel: panel.dump(cx),
                placement: DockPlacement::Right,
                size: *size,
                open: *open,
            }),
            bottom_dock: self.bottom.as_ref().map(|(panel, size, open)| DockState {
                panel: panel.dump(cx),
                placement: DockPlacement::Bottom,
                size: *size,
                open: *open,
            }),
        }
    }

    fn split_key(root: DockRoot, path: &[usize]) -> String {
        let root = match root {
            DockRoot::Center => "center",
            DockRoot::Left => "left",
            DockRoot::Right => "right",
            DockRoot::Bottom => "bottom",
        };
        if path.is_empty() {
            root.to_string()
        } else {
            format!(
                "{}:{}",
                root,
                path.iter()
                    .map(|ix| ix.to_string())
                    .collect::<Vec<_>>()
                    .join(".")
            )
        }
    }

    /// Render a pointer resize handle, or no element while structural editing is locked.
    fn resize_handle(
        &self,
        id: SharedString,
        horizontal_split: bool,
        target: DockResizeTarget,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if !self.layout_editable {
            return Empty.into_any_element();
        }
        let p = MoonPalette::active(cx);
        let grab_cursor = if horizontal_split {
            CursorStyle::ResizeColumn
        } else {
            CursorStyle::ResizeRow
        };

        div()
            .id(ElementId::from(id))
            .flex_none()
            .relative()
            .bg(rgba_from(p.shell, 1.0))
            .occlude()
            .cursor(if horizontal_split {
                CursorStyle::ResizeColumn
            } else {
                CursorStyle::ResizeRow
            })
            .when(horizontal_split, |this| {
                this.w(px(DOCK_RESIZE_HIT_SIZE))
                    .h_full()
                    .items_center()
                    .justify_center()
                    .child(div().w(px(1.0)).h_full().bg(rgba_from(p.border, 1.0)))
            })
            .when(!horizontal_split, |this| {
                this.h(px(DOCK_RESIZE_HIT_SIZE))
                    .w_full()
                    .items_center()
                    .justify_center()
                    .child(div().h(px(1.0)).w_full().bg(rgba_from(p.border, 1.0)))
            })
            .hover(|style| style.bg(rgba_from(p.shell_high, 1.0)))
            // A press ARMS the gesture; the pointer is then followed by window-level
            // listeners (`resize_pointer_hook`). Deliberately NOT `on_drag`: GPUI's drag
            // machinery exists to float a preview under the cursor, and pays for it with a
            // `Window::refresh()` on EVERY mouse move (`window.rs`, "redraw the window so that
            // the active drag can follow the mouse cursor"). A refresh clears `refreshing`,
            // which invalidates every cached view in the window — so dragging one handle was
            // rebuilding, re-laying-out and repainting every panel in the dock, including the
            // ones nowhere near it. This handle's preview renders `Empty`; it was paying a
            // full-window refresh per move to move nothing.
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this: &mut Self, _, _, cx| {
                    if !this.layout_editable {
                        return;
                    }
                    this.resize_active = Some((target.clone(), grab_cursor));
                    this.resize_notify_at = None;
                    cx.stop_propagation();
                }),
            )
            .into_any_element()
    }

    /// Collect every live panel identity once in deterministic root order.
    fn unique_panels(&self) -> Vec<Rc<dyn PanelView>> {
        let mut panels = Vec::new();
        self.center.append_unique_panels(&mut panels);
        for slot in [&self.left, &self.right, &self.bottom] {
            if let Some((item, _, _)) = slot {
                item.append_unique_panels(&mut panels);
            }
        }
        panels
    }

    /// Return an immutable root item when that dock slot exists.
    fn root_item(&self, root: DockRoot) -> Option<&DockItem> {
        match root {
            DockRoot::Center => Some(&self.center),
            DockRoot::Left => self.left.as_ref().map(|(item, _, _)| item),
            DockRoot::Right => self.right.as_ref().map(|(item, _, _)| item),
            DockRoot::Bottom => self.bottom.as_ref().map(|(item, _, _)| item),
        }
    }

    /// Return an immutable topology node at a split-only path.
    fn item_at_path<'a>(item: &'a DockItem, path: &[usize]) -> Option<&'a DockItem> {
        let mut current = item;
        for ix in path {
            let DockItem::Split { items, .. } = current else {
                return None;
            };
            current = items.get(*ix)?;
        }
        Some(current)
    }

    /// Find a named panel across all roots and retain its owning node coordinates.
    fn panel_location(&self, panel_name: &str, cx: &App) -> Option<DockPanelLocation> {
        for root in [
            DockRoot::Center,
            DockRoot::Left,
            DockRoot::Right,
            DockRoot::Bottom,
        ] {
            let Some(item) = self.root_item(root) else {
                continue;
            };
            if let Some((path, slot)) = item.panel_location(panel_name, cx, &mut Vec::new()) {
                return Some(DockPanelLocation { root, path, slot });
            }
        }
        None
    }

    /// Open a side root in place without emitting an intermediate layout event.
    fn open_root(&mut self, root: DockRoot) -> bool {
        let slot = match root {
            DockRoot::Center => return false,
            DockRoot::Left => self.left.as_mut(),
            DockRoot::Right => self.right.as_mut(),
            DockRoot::Bottom => self.bottom.as_mut(),
        };
        let Some((_, _, open)) = slot else {
            return false;
        };
        if *open {
            return false;
        }
        *open = true;
        true
    }

    /// Build the rendered element id for a topology node.
    fn item_element_id(&self, root: DockRoot, path: &[usize]) -> SharedString {
        let mut id = format!("{}:{}", self.id, Self::split_key(root, &[]));
        for ix in path {
            id.push_str(":split:");
            id.push_str(&ix.to_string());
        }
        id.into()
    }

    /// Synchronize one keyed tab runtime and every panel's active edge notification.
    fn sync_tab_runtime(
        &self,
        id: SharedString,
        items: &[Rc<dyn PanelView>],
        active_ix: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let active_ix = active_ix.min(items.len().saturating_sub(1));
        for (ix, panel) in items.iter().enumerate() {
            panel.set_active(ix == active_ix, window, cx.borrow_mut());
        }
        let announced = (
            items.get(active_ix).map(|panel| panel.panel_name(cx)),
            items.len(),
        );
        if let Some(state) = self
            .tab_runtime_states
            .get(id.as_ref())
            .and_then(WeakEntity::upgrade)
        {
            state.update(cx, |state, _| {
                state.active_ix = active_ix;
                state.notified_active = Some(announced);
            });
        }
    }

    /// Synchronize activation for a visible topology subtree and its keyed tab runtimes.
    fn sync_item_active(
        &self,
        root: DockRoot,
        path: &[usize],
        item: &DockItem,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let id = self.item_element_id(root, path);
        match item {
            DockItem::Empty => {}
            DockItem::Panel(panel) => {
                self.sync_tab_runtime(id, std::slice::from_ref(panel), 0, window, cx);
            }
            DockItem::Tabs { items, active_ix } => {
                self.sync_tab_runtime(id, items, *active_ix, window, cx);
            }
            DockItem::Tiles { items, .. } => {
                for (ix, panel) in items.iter().enumerate() {
                    self.sync_tab_runtime(
                        format!("{id}:tile-panel:{ix}").into(),
                        std::slice::from_ref(panel),
                        0,
                        window,
                        cx,
                    );
                }
            }
            DockItem::Split { items, .. } => {
                for (ix, item) in items.iter().enumerate() {
                    let mut child_path = path.to_vec();
                    child_path.push(ix);
                    self.sync_item_active(root, &child_path, item, window, cx);
                }
            }
        }
    }

    /// Mark a hidden side subtree inactive while keeping its tab index ready for reopening.
    fn sync_item_inactive(
        &self,
        root: DockRoot,
        path: &[usize],
        item: &DockItem,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let id = self.item_element_id(root, path);
        match item {
            DockItem::Empty => {}
            DockItem::Panel(panel) => {
                panel.set_active(false, window, cx.borrow_mut());
                self.mark_tab_runtime_inactive(id, 0, 1, cx);
            }
            DockItem::Tabs { items, active_ix } => {
                for panel in items {
                    panel.set_active(false, window, cx.borrow_mut());
                }
                self.mark_tab_runtime_inactive(id, *active_ix, items.len(), cx);
            }
            DockItem::Tiles { items, .. } => {
                for (ix, panel) in items.iter().enumerate() {
                    panel.set_active(false, window, cx.borrow_mut());
                    self.mark_tab_runtime_inactive(
                        format!("{id}:tile-panel:{ix}").into(),
                        0,
                        1,
                        cx,
                    );
                }
            }
            DockItem::Split { items, .. } => {
                for (ix, item) in items.iter().enumerate() {
                    let mut child_path = path.to_vec();
                    child_path.push(ix);
                    self.sync_item_inactive(root, &child_path, item, window, cx);
                }
            }
        }
    }

    /// Store an inactive edge marker so a reopened tab group announces its front panel again.
    fn mark_tab_runtime_inactive(
        &self,
        id: SharedString,
        active_ix: usize,
        item_count: usize,
        cx: &mut Context<Self>,
    ) {
        if let Some(state) = self
            .tab_runtime_states
            .get(id.as_ref())
            .and_then(WeakEntity::upgrade)
        {
            state.update(cx, |state, _| {
                state.active_ix = active_ix.min(item_count.saturating_sub(1));
                state.notified_active = Some((None, item_count));
            });
        }
    }

    /// Synchronize only the topology currently visible through normal or zoomed rendering.
    fn sync_layout_active(&self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(zoomed_panel) = self
            .zoomed_panel
            .as_ref()
            .and_then(|name| self.find_panel_named(name.as_ref(), cx))
        {
            self.sync_item_inactive(DockRoot::Center, &[], &self.center, window, cx);
            for (root, slot) in [
                (DockRoot::Left, &self.left),
                (DockRoot::Right, &self.right),
                (DockRoot::Bottom, &self.bottom),
            ] {
                if let Some((item, _, _)) = slot {
                    self.sync_item_inactive(root, &[], item, window, cx);
                }
            }
            self.sync_tab_runtime(
                format!("{}:zoom", self.id).into(),
                std::slice::from_ref(&zoomed_panel),
                0,
                window,
                cx,
            );
            return;
        }

        self.mark_tab_runtime_inactive(format!("{}:zoom", self.id).into(), 0, 1, cx);
        self.sync_item_active(DockRoot::Center, &[], &self.center, window, cx);
        for (root, slot) in [
            (DockRoot::Left, &self.left),
            (DockRoot::Right, &self.right),
            (DockRoot::Bottom, &self.bottom),
        ] {
            if let Some((item, _, open)) = slot {
                if *open {
                    self.sync_item_active(root, &[], item, window, cx);
                } else {
                    self.sync_item_inactive(root, &[], item, window, cx);
                }
            }
        }
    }

    /// Return a mutable root item when that dock slot exists.
    fn root_item_mut(&mut self, root: DockRoot) -> Option<&mut DockItem> {
        match root {
            DockRoot::Center => Some(&mut self.center),
            DockRoot::Left => self.left.as_mut().map(|(item, _, _)| item),
            DockRoot::Right => self.right.as_mut().map(|(item, _, _)| item),
            DockRoot::Bottom => self.bottom.as_mut().map(|(item, _, _)| item),
        }
    }

    /// Return a mutable topology node at a split-only path.
    fn item_at_path_mut<'a>(item: &'a mut DockItem, path: &[usize]) -> Option<&'a mut DockItem> {
        let mut current = item;
        for ix in path {
            let DockItem::Split { items, .. } = current else {
                return None;
            };
            current = items.get_mut(*ix)?;
        }
        Some(current)
    }

    fn find_panel_named(&self, panel_name: &str, cx: &App) -> Option<Rc<dyn PanelView>> {
        self.center
            .find_panel_named(panel_name, cx)
            .or_else(|| {
                self.left
                    .as_ref()
                    .and_then(|(item, _, _)| item.find_panel_named(panel_name, cx))
            })
            .or_else(|| {
                self.right
                    .as_ref()
                    .and_then(|(item, _, _)| item.find_panel_named(panel_name, cx))
            })
            .or_else(|| {
                self.bottom
                    .as_ref()
                    .and_then(|(item, _, _)| item.find_panel_named(panel_name, cx))
            })
    }

    fn set_tabs_active_index(&mut self, root: DockRoot, path: &[usize], active_ix: usize) -> bool {
        let Some(DockItem::Tabs {
            items,
            active_ix: current,
        }) = self
            .root_item_mut(root)
            .and_then(|item| Self::item_at_path_mut(item, path))
        else {
            return false;
        };
        let active_ix = active_ix.min(items.len().saturating_sub(1));
        if *current == active_ix {
            return false;
        }
        *current = active_ix;
        true
    }

    /// Publish the paired activation/layout events for one successful user dock mutation.
    ///
    /// Args:
    ///     panel_name: Exact stable name made active by the interaction.
    ///     cx: Dock context used to emit events in contract order and request a repaint.
    fn emit_user_panel_activation(&self, panel_name: SharedString, cx: &mut Context<Self>) {
        cx.emit(DockEvent::PanelActivated { panel_name });
        cx.emit(DockEvent::LayoutChanged);
        cx.notify();
    }

    /// Apply a user tab click and emit the exact newly active stable panel name.
    ///
    /// Args:
    ///     root: Dock root containing the clicked tab group.
    ///     path: Split-only path to that tab group.
    ///     active_ix: Clicked tab index, clamped to the available items.
    ///     cx: Dock context used to mutate state and emit activation/layout events.
    ///
    /// Returns:
    ///     `true` only when the selected index changes successfully.
    fn activate_tab_from_user(
        &mut self,
        root: DockRoot,
        path: &[usize],
        active_ix: usize,
        cx: &mut Context<Self>,
    ) -> bool {
        let panel_name = self
            .root_item(root)
            .and_then(|item| Self::item_at_path(item, path))
            .and_then(|item| match item {
                DockItem::Tabs { items, .. } => items
                    .get(active_ix.min(items.len().saturating_sub(1)))
                    .map(|panel| panel.panel_name(cx)),
                _ => None,
            });
        let Some(panel_name) = panel_name else {
            return false;
        };
        if !self.set_tabs_active_index(root, path, active_ix) {
            return false;
        }
        self.emit_user_panel_activation(panel_name, cx);
        true
    }

    fn move_tab_before(
        &mut self,
        root: DockRoot,
        path: &[usize],
        panel_name: &str,
        target_ix: usize,
        cx: &App,
    ) -> bool {
        let Some(DockItem::Tabs { items, active_ix }) = self
            .root_item_mut(root)
            .and_then(|item| Self::item_at_path_mut(item, path))
        else {
            return false;
        };
        let Some(from_ix) = items
            .iter()
            .position(|panel| panel.panel_name(cx).to_string() == panel_name)
        else {
            return false;
        };
        let panel = items.remove(from_ix);
        let target_ix = if from_ix < target_ix {
            target_ix.saturating_sub(1)
        } else {
            target_ix
        }
        .min(items.len());
        items.insert(target_ix, panel);
        *active_ix = target_ix;
        true
    }

    /// Reorder and activate a tab only when the pointer-driven layout editor is enabled.
    ///
    /// Args:
    ///     root: Dock root containing the source tab group.
    ///     path: Split-only path to that tab group.
    ///     panel_name: Stable name of the dragged tab.
    ///     target_ix: Requested insertion index before pinned-prefix clamping.
    ///     cx: Dock context used for mutation and activation/layout events.
    ///
    /// Returns:
    ///     Whether the reorder succeeded and emitted both events.
    fn move_tab_before_from_user(
        &mut self,
        root: DockRoot,
        path: &[usize],
        panel_name: &str,
        target_ix: usize,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.layout_editable || self.is_pinned_panel(panel_name) {
            return false;
        }
        let target_ix = target_ix.max(self.pinned_count_at(root, path, cx));
        if !self.move_tab_before(root, path, panel_name, target_ix, cx) {
            return false;
        }
        self.emit_user_panel_activation(panel_name.to_string().into(), cx);
        true
    }

    /// Remove every occurrence from every root and deduplicate identities in traversal order.
    ///
    /// Args:
    ///     panel_name: Stable logical name to extract.
    ///     cx: Application context used to resolve panel names.
    ///
    /// Returns:
    ///     Unique removed identities in deterministic center, left, right, bottom tree order.
    fn take_panels_by_name_from_all_roots(
        &mut self,
        panel_name: &str,
        cx: &App,
    ) -> Vec<Rc<dyn PanelView>> {
        let mut occurrences = Vec::new();
        let (center, removed) = std::mem::replace(&mut self.center, DockItem::Empty)
            .extract_panels_named(panel_name, cx, default_tile_meta);
        self.center = center;
        occurrences.extend(removed);
        self.left = self.left.take().and_then(|(item, size, open)| {
            let (item, removed) = item.extract_panels_named(panel_name, cx, default_tile_meta);
            occurrences.extend(removed);
            (!item.is_empty()).then_some((item, size, open))
        });
        self.right = self.right.take().and_then(|(item, size, open)| {
            let (item, removed) = item.extract_panels_named(panel_name, cx, default_tile_meta);
            occurrences.extend(removed);
            (!item.is_empty()).then_some((item, size, open))
        });
        self.bottom = self.bottom.take().and_then(|(item, size, open)| {
            let (item, removed) = item.extract_panels_named(panel_name, cx, default_tile_meta);
            occurrences.extend(removed);
            (!item.is_empty()).then_some((item, size, open))
        });

        let mut unique = Vec::new();
        for panel in occurrences {
            if !unique
                .iter()
                .any(|known: &Rc<dyn PanelView>| Rc::ptr_eq(known, &panel))
            {
                unique.push(panel);
            }
        }
        unique
    }

    fn take_panel_named_for_move(
        &mut self,
        panel_name: &str,
        cx: &App,
    ) -> Option<Rc<dyn PanelView>> {
        let (center, taken) =
            std::mem::replace(&mut self.center, DockItem::Empty).take_panel_named(panel_name, cx);
        self.center = center;
        if taken.is_some() {
            return taken;
        }

        if let Some((item, size, open)) = self.left.take() {
            let (item, taken) = item.take_panel_named(panel_name, cx);
            if !item.is_empty() {
                self.left = Some((item, size, open));
            }
            if taken.is_some() {
                return taken;
            }
        }
        if let Some((item, size, open)) = self.right.take() {
            let (item, taken) = item.take_panel_named(panel_name, cx);
            if !item.is_empty() {
                self.right = Some((item, size, open));
            }
            if taken.is_some() {
                return taken;
            }
        }
        if let Some((item, size, open)) = self.bottom.take() {
            let (item, taken) = item.take_panel_named(panel_name, cx);
            if !item.is_empty() {
                self.bottom = Some((item, size, open));
            }
            if taken.is_some() {
                return taken;
            }
        }

        None
    }

    fn insert_panel_into_tabs(
        &mut self,
        root: DockRoot,
        path: &[usize],
        target_ix: usize,
        panel: Rc<dyn PanelView>,
    ) -> bool {
        let Some(item) = self
            .root_item_mut(root)
            .and_then(|item| Self::item_at_path_mut(item, path))
        else {
            return false;
        };

        match item {
            DockItem::Empty => {
                *item = DockItem::Panel(panel);
                true
            }
            DockItem::Panel(existing) => {
                let existing = existing.clone();
                *item = DockItem::Tabs {
                    items: vec![existing, panel],
                    active_ix: 1,
                };
                true
            }
            DockItem::Tabs { items, active_ix } => {
                let target_ix = target_ix.min(items.len());
                items.insert(target_ix, panel);
                *active_ix = target_ix;
                true
            }
            DockItem::Split { .. } | DockItem::Tiles { .. } => false,
        }
    }

    fn move_panel_to_tabs(
        &mut self,
        panel_name: &str,
        root: DockRoot,
        path: &[usize],
        target_ix: usize,
        cx: &App,
    ) -> bool {
        let anchor = self
            .root_item_mut(root)
            .and_then(|it| Self::item_at_path_mut(it, path))
            .and_then(|it| it.first_panel_name_excluding(panel_name, cx));
        if anchor.is_none() {
            return false;
        }
        let Some(panel) = self.take_panel_named_for_move(panel_name, cx) else {
            return false;
        };
        let target_path = anchor
            .as_deref()
            .and_then(|a| {
                self.root_item_mut(root)
                    .and_then(|it| it.find_panel_path(a, cx))
            })
            .unwrap_or_else(|| path.to_vec());
        if self.insert_panel_into_tabs(root, &target_path, target_ix, panel.clone()) {
            true
        } else {
            let pushed = self
                .root_item_mut(root)
                .map(|it| it.try_push_into_first_tabs(panel.clone()))
                .unwrap_or(false);
            if !pushed {
                let existing = std::mem::replace(&mut self.center, DockItem::Empty);
                self.center = DockItem::Split {
                    horizontal: false,
                    items: vec![existing, DockItem::Panel(panel)],
                    sizes: Vec::new(),
                };
            }
            false
        }
    }

    /// Move and activate a panel between tab groups only when the layout editor is enabled.
    ///
    /// Args:
    ///     panel_name: Stable name of the dragged panel.
    ///     root: Destination dock root.
    ///     path: Split-only path to the destination tab group.
    ///     target_ix: Requested insertion index before pinned-prefix clamping.
    ///     cx: Dock context used for mutation and activation/layout events.
    ///
    /// Returns:
    ///     Whether the transfer succeeded and emitted both events.
    fn move_panel_to_tabs_from_user(
        &mut self,
        panel_name: &str,
        root: DockRoot,
        path: &[usize],
        target_ix: usize,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.layout_editable || self.is_pinned_panel(panel_name) {
            return false;
        }
        let target_ix = target_ix.max(self.pinned_count_at(root, path, cx));
        if !self.move_panel_to_tabs(panel_name, root, path, target_ix, cx) {
            return false;
        }
        self.emit_user_panel_activation(panel_name.to_string().into(), cx);
        true
    }

    /// Return whether a stable panel name has the configured pinned-leading role.
    fn is_pinned_panel(&self, panel_name: &str) -> bool {
        self.pinned_leading_panels
            .iter()
            .any(|name| name.as_ref() == panel_name)
    }

    /// Count the leading pinned tabs at one target node for insertion clamping.
    fn pinned_count_at(&self, root: DockRoot, path: &[usize], cx: &App) -> usize {
        let Some(DockItem::Tabs { items, .. }) = self
            .root_item(root)
            .and_then(|item| Self::item_at_path(item, path))
        else {
            return 0;
        };
        items
            .iter()
            .take_while(|panel| self.is_pinned_panel(panel.panel_name(cx).as_ref()))
            .count()
    }

    /// Return whether a target subtree contains any configured pinned-leading panel.
    ///
    /// Args:
    ///     root: Dock root containing the prospective split target.
    ///     path: Split-only path to the target subtree.
    ///     cx: Application context used to resolve stable panel names.
    ///
    /// Returns:
    ///     `true` when a pinned panel would be displaced by a leading split insertion.
    fn target_contains_pinned_panel(&self, root: DockRoot, path: &[usize], cx: &App) -> bool {
        let Some(item) = self
            .root_item(root)
            .and_then(|item| Self::item_at_path(item, path))
        else {
            return false;
        };
        self.pinned_leading_panels
            .iter()
            .any(|name| item.find_panel_named(name.as_ref(), cx).is_some())
    }

    /// Apply a user split drop without letting an operational panel precede a pinned subtree.
    ///
    /// Args:
    ///     panel_name: Stable name of the dragged panel.
    ///     root: Dock root containing the target subtree.
    ///     path: Split-only path to the target subtree.
    ///     placement: Requested edge placement around that subtree.
    ///     cx: Application context used for panel lookup and topology mutation.
    ///
    /// Returns:
    ///     Whether the live topology changed and emitted activation/layout events.
    fn move_panel_to_split_from_user(
        &mut self,
        panel_name: &str,
        root: DockRoot,
        path: &[usize],
        placement: DockSplitPlacement,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.layout_editable
            || self.is_pinned_panel(panel_name)
            || (placement == DockSplitPlacement::Left
                && self.target_contains_pinned_panel(root, path, cx))
        {
            return false;
        }
        if !self.move_panel_to_split(panel_name, root, path, placement, cx) {
            return false;
        }
        self.emit_user_panel_activation(panel_name.to_string().into(), cx);
        true
    }

    fn split_item_with_panel(
        &mut self,
        root: DockRoot,
        path: &[usize],
        placement: DockSplitPlacement,
        panel: Rc<dyn PanelView>,
    ) -> bool {
        let Some(item) = self
            .root_item_mut(root)
            .and_then(|item| Self::item_at_path_mut(item, path))
        else {
            return false;
        };
        let existing = std::mem::replace(item, DockItem::Empty);
        if existing.is_empty() {
            *item = DockItem::Panel(panel);
            return true;
        }
        let new_panel = DockItem::Panel(panel);
        let (horizontal, items) = match placement {
            DockSplitPlacement::Left => (true, vec![new_panel, existing]),
            DockSplitPlacement::Right => (true, vec![existing, new_panel]),
            DockSplitPlacement::Top => (false, vec![new_panel, existing]),
            DockSplitPlacement::Bottom => (false, vec![existing, new_panel]),
        };
        *item = DockItem::Split {
            horizontal,
            items,
            sizes: Vec::new(),
        };
        true
    }

    fn move_panel_to_split(
        &mut self,
        panel_name: &str,
        root: DockRoot,
        path: &[usize],
        placement: DockSplitPlacement,
        cx: &App,
    ) -> bool {
        // «Якорь» целевого слота — имя соседней панели в нём (до take). take может схлопнуть
        // узел и сдвинуть path; по якорю находим целевой слот заново уже после take.
        let anchor = self
            .root_item_mut(root)
            .and_then(|it| Self::item_at_path_mut(it, path))
            .and_then(|it| it.first_panel_name_excluding(panel_name, cx));
        // anchor=None → целевой слот содержит ТОЛЬКО саму перетаскиваемую панель (дроп на
        // себя): split бессмыслен. Отменяем ДО take, чтобы не остаться без якоря и не
        // разрушить раскладку. Панель остаётся на месте.
        if anchor.is_none() {
            return false;
        }
        let Some(panel) = self.take_panel_named_for_move(panel_name, cx) else {
            return false;
        };
        // Пересчитываем путь к целевому слоту по якорю (path мог устареть после take).
        let target_path = anchor
            .as_deref()
            .and_then(|a| {
                self.root_item_mut(root)
                    .and_then(|it| it.find_panel_path(a, cx))
            })
            .unwrap_or_else(|| path.to_vec());
        if self.split_item_with_panel(root, &target_path, placement, panel.clone()) {
            true
        } else {
            // НЕ разрушаем центр (старый with_panel_added схлопывал всё в Tabs[panel]):
            // возвращаем панель вкладкой в первый Tabs.
            let pushed = self
                .root_item_mut(root)
                .map(|it| it.try_push_into_first_tabs(panel.clone()))
                .unwrap_or(false);
            if !pushed {
                // Нет ни одного Tabs — добавляем панель отдельным слотом снизу, НЕ заменяя
                // центр (with_panel_added схлопывал всё в Tabs[panel] → на весь экран).
                let existing = std::mem::replace(&mut self.center, DockItem::Empty);
                self.center = DockItem::Split {
                    horizontal: false,
                    items: vec![existing, DockItem::Panel(panel)],
                    sizes: Vec::new(),
                };
            }
            false
        }
    }

    fn set_changed_size(slot: &mut f32, size: f32) -> bool {
        if (*slot - size).abs() <= 0.5 {
            return false;
        }
        *slot = size;
        true
    }

    fn snap_tile_value(value: f32) -> f32 {
        (value / DOCK_TILE_SNAP).round() * DOCK_TILE_SNAP
    }

    fn clamp_tile_meta(mut meta: TileMeta, bounds: Bounds<Pixels>) -> TileMeta {
        let max_w = f32::from(bounds.size.width).max(DOCK_TILE_MIN_W);
        let max_h = f32::from(bounds.size.height).max(DOCK_TILE_MIN_H);
        meta.w = Self::snap_tile_value(meta.w).clamp(DOCK_TILE_MIN_W, max_w);
        meta.h = Self::snap_tile_value(meta.h).clamp(DOCK_TILE_MIN_H, max_h);
        meta.x = Self::snap_tile_value(meta.x).clamp(0.0, (max_w - meta.w).max(0.0));
        meta.y = Self::snap_tile_value(meta.y).clamp(0.0, (max_h - meta.h).max(0.0));
        meta
    }

    fn tile_key(root: DockRoot, path: &[usize]) -> String {
        format!("tiles:{}", Self::split_key(root, path))
    }

    fn update_tile_meta(
        &mut self,
        root: DockRoot,
        path: &[usize],
        ix: usize,
        meta: TileMeta,
    ) -> bool {
        let Some(DockItem::Tiles { metas, .. }) = self
            .root_item_mut(root)
            .and_then(|item| Self::item_at_path_mut(item, path))
        else {
            return false;
        };

        if metas.len() <= ix {
            metas.resize(
                ix + 1,
                TileMeta {
                    x: 12.0,
                    y: 12.0,
                    w: 320.0,
                    h: 200.0,
                    z_index: 0,
                },
            );
        }

        if metas[ix] == meta {
            return false;
        }
        metas[ix] = meta;
        true
    }

    fn start_tile_drag(
        &mut self,
        root: DockRoot,
        path: Vec<usize>,
        ix: usize,
        cursor: Point<Pixels>,
        meta: TileMeta,
    ) {
        self.tile_drag_start = Some(DockTileDragStart {
            root,
            path,
            ix,
            cursor,
            meta,
        });
    }

    fn clear_tile_drag(
        &mut self,
        _event: &MouseUpEvent,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        self.tile_drag_start = None;
    }

    /// Apply one user tile drag while layout editing remains enabled.
    fn on_tile_drag_move(
        &mut self,
        event: &DragMoveEvent<DockTileDrag>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.layout_editable {
            return;
        }
        let drag = event.drag(cx);
        if drag.dock_id != cx.entity_id() {
            return;
        }
        let Some(start) = self.tile_drag_start.as_ref() else {
            return;
        };
        if start.root != drag.root || start.path != drag.path || start.ix != drag.ix {
            return;
        }

        let dx = f32::from(event.event.position.x - start.cursor.x);
        let dy = f32::from(event.event.position.y - start.cursor.y);
        let mut meta = start.meta;
        match drag.kind {
            DockTileDragKind::Move => {
                meta.x += dx;
                meta.y += dy;
            }
            DockTileDragKind::ResizeRight => {
                meta.w += dx;
            }
            DockTileDragKind::ResizeBottom => {
                meta.h += dy;
            }
            DockTileDragKind::ResizeBottomRight => {
                meta.w += dx;
                meta.h += dy;
            }
        }

        let bounds = self
            .tile_bounds
            .get(&Self::tile_key(drag.root, &drag.path))
            .copied()
            .unwrap_or(self.root_bounds);
        meta = Self::clamp_tile_meta(meta, bounds);
        if self.update_tile_meta(drag.root, &drag.path, drag.ix, meta) {
            cx.emit(DockEvent::LayoutChanged);
            cx.notify();
        }
    }

    /// Render one directional drop zone with a stale-event editability guard.
    fn split_drop_zone(
        &self,
        id: SharedString,
        root: DockRoot,
        path: Vec<usize>,
        placement: DockSplitPlacement,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let dock = cx.entity();
        let dock_id = cx.entity_id();
        let tokens = MoonTheme::active_tokens(cx);
        let mut zone = div()
            .id(ElementId::from(id))
            .absolute()
            .when(matches!(placement, DockSplitPlacement::Left), |this| {
                this.left(px(0.0))
                    .top(px(0.0))
                    .bottom(px(0.0))
                    .w(px(tokens.ui(42.0)))
                    .border_l(px(tokens.ui(2.0)))
            })
            .when(matches!(placement, DockSplitPlacement::Right), |this| {
                this.right(px(0.0))
                    .top(px(0.0))
                    .bottom(px(0.0))
                    .w(px(tokens.ui(42.0)))
                    .border_r(px(tokens.ui(2.0)))
            })
            .when(matches!(placement, DockSplitPlacement::Top), |this| {
                this.top(px(0.0))
                    .left(px(tokens.ui(42.0)))
                    .right(px(tokens.ui(42.0)))
                    .h(px(tokens.ui(34.0)))
                    .border_t(px(tokens.ui(2.0)))
            })
            .when(matches!(placement, DockSplitPlacement::Bottom), |this| {
                this.bottom(px(0.0))
                    .left(px(tokens.ui(42.0)))
                    .right(px(tokens.ui(42.0)))
                    .h(px(tokens.ui(34.0)))
                    .border_b(px(tokens.ui(2.0)))
            })
            .drag_over::<DockTabDrag>(move |style, drag, _, cx| {
                let p = MoonPalette::active(cx);
                if drag.dock_id == dock_id && drag.splittable {
                    style
                        .bg(rgba_from(p.accent, p.accent_tint_a))
                        .border_color(rgba_from(p.accent, 0.88))
                } else {
                    style
                }
            });

        zone = zone.on_drop(move |drag: &DockTabDrag, _window, cx| {
            if drag.dock_id != dock_id || !drag.splittable {
                return;
            }
            _ = dock.update(cx, |dock, cx| {
                dock.move_panel_to_split_from_user(
                    drag.panel_name.as_ref(),
                    root,
                    &path,
                    placement,
                    cx,
                );
            });
        });

        zone.into_any_element()
    }

    /// Add user split targets only when both split drops and layout editing are enabled.
    fn add_split_drop_zones(
        &self,
        mut host: Div,
        id_text: &str,
        root: DockRoot,
        path: Vec<usize>,
        target_splittable: bool,
        cx: &mut Context<Self>,
    ) -> Div {
        // Slot accepts split drops only if it is itself splittable (a bottom dock panel),
        // so a chart/detect slot never gets split zones.
        if !self.layout_editable || !self.enable_split_drop || !target_splittable {
            return host;
        }
        host = host
            .child(self.split_drop_zone(
                SharedString::from(format!("{id_text}:drop-left")),
                root,
                path.clone(),
                DockSplitPlacement::Left,
                cx,
            ))
            .child(self.split_drop_zone(
                SharedString::from(format!("{id_text}:drop-right")),
                root,
                path.clone(),
                DockSplitPlacement::Right,
                cx,
            ))
            .child(self.split_drop_zone(
                SharedString::from(format!("{id_text}:drop-top")),
                root,
                path.clone(),
                DockSplitPlacement::Top,
                cx,
            ))
            .child(self.split_drop_zone(
                SharedString::from(format!("{id_text}:drop-bottom")),
                root,
                path,
                DockSplitPlacement::Bottom,
                cx,
            ));
        host
    }

    fn resize_outer_left(&mut self, position: Point<Pixels>) -> bool {
        let row_w = f32::from(self.row_bounds.size.width);
        if row_w <= 1.0 {
            return false;
        }
        let right_w = self
            .right
            .as_ref()
            .filter(|(_, _, open)| *open)
            .map(|(_, size, _)| *size)
            .unwrap_or(0.0);
        let max = (row_w - right_w - DOCK_MIN_CENTER_SIZE).max(DOCK_MIN_SIDE_SIZE);
        let size = (f32::from(position.x) - f32::from(self.row_bounds.origin.x))
            .clamp(DOCK_MIN_SIDE_SIZE, max);
        self.left
            .as_mut()
            .map(|(_, current, _)| Self::set_changed_size(current, size))
            .unwrap_or(false)
    }

    fn resize_outer_right(&mut self, position: Point<Pixels>) -> bool {
        let row_w = f32::from(self.row_bounds.size.width);
        if row_w <= 1.0 {
            return false;
        }
        let left_w = self
            .left
            .as_ref()
            .filter(|(_, _, open)| *open)
            .map(|(_, size, _)| *size)
            .unwrap_or(0.0);
        let max = (row_w - left_w - DOCK_MIN_CENTER_SIZE).max(DOCK_MIN_SIDE_SIZE);
        let row_right = f32::from(self.row_bounds.origin.x) + row_w;
        let size = (row_right - f32::from(position.x)).clamp(DOCK_MIN_SIDE_SIZE, max);
        self.right
            .as_mut()
            .map(|(_, current, _)| Self::set_changed_size(current, size))
            .unwrap_or(false)
    }

    fn resize_outer_bottom(&mut self, position: Point<Pixels>) -> bool {
        let root_h = f32::from(self.root_bounds.size.height);
        if root_h <= 1.0 {
            return false;
        }
        let root_bottom = f32::from(self.root_bounds.origin.y) + root_h;
        let max = (root_h - DOCK_MIN_CENTER_SIZE).max(DOCK_MIN_BOTTOM_SIZE);
        let size = (root_bottom - f32::from(position.y)).clamp(DOCK_MIN_BOTTOM_SIZE, max);
        self.bottom
            .as_mut()
            .map(|(_, current, _)| Self::set_changed_size(current, size))
            .unwrap_or(false)
    }

    fn resize_split(
        &mut self,
        root: DockRoot,
        path: &[usize],
        after_ix: usize,
        position: Point<Pixels>,
    ) -> bool {
        let key = Self::split_key(root, path);
        let Some(bounds) = self.split_bounds.get(&key).cloned() else {
            return false;
        };
        let Some(DockItem::Split {
            horizontal,
            items,
            sizes,
        }) = self
            .root_item_mut(root)
            .and_then(|item| Self::item_at_path_mut(item, path))
        else {
            return false;
        };
        if after_ix == 0 || after_ix >= items.len() {
            return false;
        }

        if sizes.len() < items.len() {
            sizes.resize(items.len(), None);
        }

        let total = if *horizontal {
            f32::from(bounds.size.width)
        } else {
            f32::from(bounds.size.height)
        };
        if total <= 1.0 {
            return false;
        }
        if total <= DOCK_MIN_SIDE_SIZE * 2.0 {
            return false;
        }

        let local = if *horizontal {
            f32::from(position.x) - f32::from(bounds.origin.x)
        } else {
            f32::from(position.y) - f32::from(bounds.origin.y)
        }
        .clamp(DOCK_MIN_SIDE_SIZE, total - DOCK_MIN_SIDE_SIZE);

        let target_ix = if after_ix + 1 == items.len() {
            after_ix
        } else {
            after_ix - 1
        };
        let size = if target_ix == after_ix {
            (total - local).clamp(DOCK_MIN_SIDE_SIZE, total - DOCK_MIN_SIDE_SIZE)
        } else {
            local.clamp(DOCK_MIN_SIDE_SIZE, total - DOCK_MIN_SIDE_SIZE)
        };

        let previous = sizes[target_ix].unwrap_or(-1.0);
        if (previous - size).abs() <= 0.5 {
            return false;
        }
        sizes[target_ix] = Some(size);
        true
    }

    /// Apply one user resize drag while layout editing remains enabled.
    /// Zero-sized element that follows the pointer for the whole resize gesture.
    ///
    /// `Window::on_mouse_event` is a PAINT-phase API, so the listeners are installed from a
    /// `canvas` paint closure rather than from `render`, which runs a phase earlier. Window-level
    /// and not on the dock's own element: on Windows the platform captures the pointer for the
    /// life of the press (`SetCapture` on mouse-down, `ReleaseCapture` on mouse-up), so a handle
    /// dragged past the edge of the window keeps receiving moves — an element-scoped listener
    /// would simply stop hearing them and the panel would freeze under the cursor.
    ///
    /// Only present while a gesture is in flight, so an idle dock installs nothing.
    fn resize_pointer_hook(&self, cursor: CursorStyle, cx: &mut Context<Self>) -> AnyElement {
        let moved = cx.entity().downgrade();
        let released = moved.clone();
        canvas(
            |_, _, _| (),
            move |_, (), window, _| {
                // Held for the gesture, not for the hitbox: once the pointer leaves the thin
                // handle the cursor would otherwise flip to whatever is underneath, mid-drag.
                // The request is per-frame, so it lapses on its own when the hook goes away.
                window.set_window_cursor_style(cursor);
                window.on_mouse_event({
                    let dock = moved.clone();
                    move |event: &MouseMoveEvent, phase, window, cx| {
                        if !phase.bubble() {
                            return;
                        }
                        let position = event.position;
                        let dragging = event.dragging();
                        dock.update(cx, |this, cx| {
                            this.on_resize_pointer_move(position, dragging, window, cx)
                        })
                        .ok();
                    }
                });
                window.on_mouse_event({
                    let dock = released.clone();
                    move |_: &MouseUpEvent, phase, _, cx| {
                        if !phase.bubble() {
                            return;
                        }
                        dock.update(cx, |this, cx| this.end_resize(cx)).ok();
                    }
                });
            },
        )
        .absolute()
        .size_0()
        .into_any_element()
    }

    /// Apply one pointer move of an armed resize gesture.
    ///
    /// `dragging` is the button state carried by the move itself. A move without it means the
    /// release was never delivered — the pointer was over another window, or the platform ate the
    /// event — and the gesture is ended here rather than left armed forever.
    fn on_resize_pointer_move(
        &mut self,
        position: Point<Pixels>,
        dragging: bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some((target, _)) = self.resize_active.clone() else {
            return;
        };
        if !dragging || !self.layout_editable {
            self.end_resize(cx);
            return;
        }

        let changed = match &target {
            DockResizeTarget::OuterLeft => self.resize_outer_left(position),
            DockResizeTarget::OuterRight => self.resize_outer_right(position),
            DockResizeTarget::OuterBottom => self.resize_outer_bottom(position),
            DockResizeTarget::Split {
                root,
                path,
                after_ix,
            } => self.resize_split(*root, path, *after_ix, position),
        };

        if !changed {
            return;
        }
        cx.emit(DockEvent::LayoutChanged);
        // PACED. One notify here is not a repaint of the two panels beside the handle: it dirties
        // this view and every ancestor, and a re-rendered root re-lays-out and repaints the WHOLE
        // window. Measured in a host terminal on a 120 Hz display, one splitter drag drove about
        // 100 full window draws a second at 7.7 ms each — 92% of an 8.33 ms frame budget, leaving
        // nothing for anything else, and the drag stuttered because any hiccup then overflowed it.
        //
        // 60 Hz, not the vblank rate: a pointer-following drag has to look continuous, and 60 is
        // where it already does. Painting at every vblank of a high-refresh panel buys nothing an
        // eye can see and spends the headroom that keeps the cadence steady.
        //
        // The layout itself is NOT paced — `resize_*` ran above on this very move, so the sizes
        // are always current and the handle never trails the pointer. Only the repaint is thinned.
        // A dropped one needs no settling: releasing the button ends the drag, and GPUI answers a
        // mouse-up during a drag with `Window::refresh` (`window.rs`, "cancel the active drag and
        // redraw the window"), so the final position is always painted.
        let now = Instant::now();
        if self
            .resize_notify_at
            .is_none_or(|last| now.duration_since(last) >= RESIZE_NOTIFY_MIN_INTERVAL)
        {
            self.resize_notify_at = Some(now);
            cx.notify();
        }
    }

    /// End an armed resize gesture and paint where it finished.
    ///
    /// The final notify is unconditional and pays whatever the pacer dropped: the last move of a
    /// gesture usually lands inside the interval, and nothing else is coming to redraw it.
    /// Idempotent, because the release arrives at both this and any other listener.
    fn end_resize(&mut self, cx: &mut Context<Self>) {
        if self.resize_active.take().is_none() {
            return;
        }
        self.resize_notify_at = None;
        cx.emit(DockEvent::LayoutChanged);
        cx.notify();
    }

    /// Render one topology node with edit affordances controlled by `layout_editable`.
    fn render_item(
        &self,
        id: SharedString,
        root: DockRoot,
        path: Vec<usize>,
        item: &DockItem,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        match item {
            DockItem::Empty => {
                let p = MoonPalette::active(cx);
                div()
                    .size_full()
                    .bg(rgba_from(p.shell, 1.0))
                    .into_any_element()
            }
            DockItem::Panel(panel) => {
                let id_text = id.to_string();
                let want_header = panel.show_dock_header(cx);
                let host = div().relative().size_full().child(
                    TabPanel::new(id, vec![panel.clone()])
                        .dock_context(cx.entity().downgrade(), root, path.clone())
                        .show_header(want_header)
                        .show_panel_controls(want_header)
                        .layout_editable(self.layout_editable)
                        .detach_allowed(self.detach_allowed)
                        .close_allowed(self.close_allowed)
                        .pinned_leading_panels(self.pinned_leading_panels.clone())
                        .background_policy(self.tab_background_policy)
                        .content_background_policy(
                            self.content_background_policy
                                .unwrap_or_else(|| panel.background_policy(cx)),
                        ),
                );
                self.add_split_drop_zones(host, &id_text, root, path, want_header, cx)
                    .into_any_element()
            }
            DockItem::Tabs { items, active_ix } => {
                let target_splittable = items.iter().any(|p| p.show_dock_header(cx));
                let id_text = id.to_string();
                let host = div().relative().size_full().child(
                    TabPanel::new(id, items.clone())
                        .dock_context(cx.entity().downgrade(), root, path.clone())
                        .active_index(*active_ix)
                        .layout_editable(self.layout_editable)
                        .detach_allowed(self.detach_allowed)
                        .close_allowed(self.close_allowed)
                        .pinned_leading_panels(self.pinned_leading_panels.clone())
                        .background_policy(self.tab_background_policy)
                        .when_some(self.content_background_policy, |this, policy| {
                            this.content_background_policy(policy)
                        }),
                );
                self.add_split_drop_zones(host, &id_text, root, path, target_splittable, cx)
                    .into_any_element()
            }
            DockItem::Tiles { items, metas } => {
                let p = MoonPalette::active(cx);
                let id_text = id.to_string();
                let tile_key = Self::tile_key(root, &path);
                let dock = cx.entity();
                let mut tiles = div()
                    .id(ElementId::from(id.clone()))
                    .relative()
                    .size_full()
                    .overflow_hidden();
                tiles = item.background_policy(cx).apply(tiles, p.shell, 1.0);
                tiles = tiles.child(
                    canvas(
                        {
                            let dock = dock.clone();
                            let tile_key = tile_key.clone();
                            move |bounds, _, cx| {
                                dock.update(cx, |area, _| {
                                    area.tile_bounds.insert(tile_key.clone(), bounds);
                                });
                            }
                        },
                        |_, _, _, _| {},
                    )
                    .absolute()
                    .size_full(),
                );
                let mut ordered = items.iter().enumerate().collect::<Vec<_>>();
                ordered
                    .sort_by_key(|(ix, _)| metas.get(*ix).map(|meta| meta.z_index).unwrap_or(*ix));
                let tokens = MoonTheme::active_tokens(cx);
                for (ix, panel) in ordered {
                    let meta = metas.get(ix).copied().unwrap_or(TileMeta {
                        x: 12.0 + ix as f32 * 18.0,
                        y: 12.0 + ix as f32 * 18.0,
                        w: 320.0,
                        h: 200.0,
                        z_index: ix,
                    });
                    let dock_id = cx.entity_id();
                    let tile_path = path.clone();
                    let move_drag = DockTileDrag {
                        dock_id,
                        root,
                        path: tile_path.clone(),
                        ix,
                        kind: DockTileDragKind::Move,
                    };
                    let right_drag = DockTileDrag {
                        kind: DockTileDragKind::ResizeRight,
                        ..move_drag.clone()
                    };
                    let bottom_drag = DockTileDrag {
                        kind: DockTileDragKind::ResizeBottom,
                        ..move_drag.clone()
                    };
                    let corner_drag = DockTileDrag {
                        kind: DockTileDragKind::ResizeBottomRight,
                        ..move_drag.clone()
                    };

                    let start_tile_drag =
                        |dock: Entity<DockArea>,
                         root: DockRoot,
                         path: Vec<usize>,
                         ix: usize,
                         meta: TileMeta| {
                            move |event: &MouseDownEvent, _window: &mut Window, cx: &mut App| {
                                cx.stop_propagation();
                                dock.update(cx, |area, _| {
                                    area.start_tile_drag(
                                        root,
                                        path.clone(),
                                        ix,
                                        event.position,
                                        meta,
                                    );
                                });
                            }
                        };

                    let mut tile = div()
                        .id(ElementId::from(SharedString::from(format!(
                            "{id_text}:tile:{ix}"
                        ))))
                        .absolute()
                        .left(px(meta.x))
                        .top(px(meta.y))
                        .w(px(meta.w))
                        .h(px(meta.h))
                        .overflow_hidden()
                        .rounded(px(tokens.ui(5.0)))
                        .border(px(1.0))
                        .border_color(rgba_from(p.border, 1.0));
                    tile = panel.background_policy(cx).apply(tile, p.shell_high, 1.0);
                    tiles = tiles.child(
                        tile.child(
                            TabPanel::new(
                                SharedString::from(format!("{id_text}:tile-panel:{ix}")),
                                vec![panel.clone()],
                            )
                            .dock_context(cx.entity().downgrade(), root, {
                                let mut tile_path = path.clone();
                                tile_path.push(ix);
                                tile_path
                            })
                            .layout_editable(self.layout_editable)
                            .detach_allowed(self.detach_allowed)
                            .close_allowed(self.close_allowed)
                            .pinned_leading_panels(self.pinned_leading_panels.clone())
                            .background_policy(MoonBackgroundPolicy::NoFill)
                            .content_background_policy(panel.background_policy(cx)),
                        )
                        .child(
                            div()
                                .id(ElementId::from(SharedString::from(format!(
                                    "{id_text}:tile-move:{ix}"
                                ))))
                                .when(!self.layout_editable, |handle| handle.invisible())
                                .absolute()
                                .left(px(0.0))
                                .top(px(0.0))
                                .right(px(tokens.ui(42.0)))
                                .h(px(tokens.ui(23.0)))
                                .cursor(CursorStyle::OpenHand)
                                .hover(move |style| style.bg(rgba_from(p.overlay, 0.035)))
                                .on_mouse_down(
                                    MouseButton::Left,
                                    start_tile_drag(
                                        dock.clone(),
                                        root,
                                        tile_path.clone(),
                                        ix,
                                        meta,
                                    ),
                                )
                                .on_mouse_up(MouseButton::Left, cx.listener(Self::clear_tile_drag))
                                .on_mouse_up_out(
                                    MouseButton::Left,
                                    cx.listener(Self::clear_tile_drag),
                                )
                                .on_drag(move_drag, |drag, _, _, cx| {
                                    cx.stop_propagation();
                                    cx.new(|_| drag.clone())
                                })
                                .on_drag_move(cx.listener(Self::on_tile_drag_move)),
                        )
                        .child(
                            div()
                                .id(ElementId::from(SharedString::from(format!(
                                    "{id_text}:tile-resize-r:{ix}"
                                ))))
                                .when(!self.layout_editable, |handle| handle.invisible())
                                .absolute()
                                .right(px(0.0))
                                .top(px(22.0))
                                .bottom(px(10.0))
                                .w(px(tokens.ui(7.0)))
                                .cursor(CursorStyle::ResizeLeftRight)
                                .hover(|style| style.bg(rgba_from(p.accent, 0.16)))
                                .on_mouse_down(
                                    MouseButton::Left,
                                    start_tile_drag(
                                        dock.clone(),
                                        root,
                                        tile_path.clone(),
                                        ix,
                                        meta,
                                    ),
                                )
                                .on_mouse_up(MouseButton::Left, cx.listener(Self::clear_tile_drag))
                                .on_mouse_up_out(
                                    MouseButton::Left,
                                    cx.listener(Self::clear_tile_drag),
                                )
                                .on_drag(right_drag, |drag, _, _, cx| {
                                    cx.stop_propagation();
                                    cx.new(|_| drag.clone())
                                })
                                .on_drag_move(cx.listener(Self::on_tile_drag_move)),
                        )
                        .child(
                            div()
                                .id(ElementId::from(SharedString::from(format!(
                                    "{id_text}:tile-resize-b:{ix}"
                                ))))
                                .when(!self.layout_editable, |handle| handle.invisible())
                                .absolute()
                                .left(px(0.0))
                                .right(px(10.0))
                                .bottom(px(0.0))
                                .h(px(tokens.ui(7.0)))
                                .cursor(CursorStyle::ResizeUpDown)
                                .hover(|style| style.bg(rgba_from(p.accent, 0.16)))
                                .on_mouse_down(
                                    MouseButton::Left,
                                    start_tile_drag(
                                        dock.clone(),
                                        root,
                                        tile_path.clone(),
                                        ix,
                                        meta,
                                    ),
                                )
                                .on_mouse_up(MouseButton::Left, cx.listener(Self::clear_tile_drag))
                                .on_mouse_up_out(
                                    MouseButton::Left,
                                    cx.listener(Self::clear_tile_drag),
                                )
                                .on_drag(bottom_drag, |drag, _, _, cx| {
                                    cx.stop_propagation();
                                    cx.new(|_| drag.clone())
                                })
                                .on_drag_move(cx.listener(Self::on_tile_drag_move)),
                        )
                        .child(
                            div()
                                .id(ElementId::from(SharedString::from(format!(
                                    "{id_text}:tile-resize-corner:{ix}"
                                ))))
                                .when(!self.layout_editable, |handle| handle.invisible())
                                .absolute()
                                .right(px(0.0))
                                .bottom(px(0.0))
                                .size(px(12.0))
                                .cursor(CursorStyle::ResizeUpLeftDownRight)
                                .hover(|style| style.bg(rgba_from(p.accent, 0.22)))
                                .on_mouse_down(
                                    MouseButton::Left,
                                    start_tile_drag(
                                        dock.clone(),
                                        root,
                                        tile_path.clone(),
                                        ix,
                                        meta,
                                    ),
                                )
                                .on_mouse_up(MouseButton::Left, cx.listener(Self::clear_tile_drag))
                                .on_mouse_up_out(
                                    MouseButton::Left,
                                    cx.listener(Self::clear_tile_drag),
                                )
                                .on_drag(corner_drag, |drag, _, _, cx| {
                                    cx.stop_propagation();
                                    cx.new(|_| drag.clone())
                                })
                                .on_drag_move(cx.listener(Self::on_tile_drag_move)),
                        ),
                    );
                }
                tiles.into_any_element()
            }
            DockItem::Split {
                horizontal,
                items,
                sizes,
            } => {
                let p = MoonPalette::active(cx);
                let id_text = id.to_string();
                let split_key = Self::split_key(root, &path);
                let dock = cx.entity();
                let first_child_flexes = items.len() > 1
                    && sizes.len() >= items.len()
                    && sizes.iter().take(items.len()).all(Option::is_some);
                let mut split = div()
                    .id(ElementId::from(id.clone()))
                    .size_full()
                    .relative()
                    .flex()
                    .when(*horizontal, |this| this.flex_row())
                    .when(!*horizontal, |this| this.flex_col());
                split = split.child(
                    canvas(
                        {
                            let dock = dock.clone();
                            let split_key = split_key.clone();
                            move |bounds, _, cx| {
                                dock.update(cx, |area, _| {
                                    area.split_bounds.insert(split_key.clone(), bounds);
                                });
                            }
                        },
                        |_, _, _, _| {},
                    )
                    .absolute()
                    .size_full(),
                );
                for (ix, child) in items.iter().enumerate() {
                    if ix > 0 {
                        let separator = self.resize_handle(
                            SharedString::from(format!("{id_text}:resize:{ix}")),
                            *horizontal,
                            DockResizeTarget::Split {
                                root,
                                path: path.clone(),
                                after_ix: ix,
                            },
                            cx,
                        );
                        split = split.child(separator);
                    }
                    // Zero the minimum on BOTH axes (not just the main one). Otherwise a child
                    // of a vertical split (e.g. the bottom block with a dense row of side-by-side
                    // panels) keeps the min-WIDTH of its content, won't shrink to the window
                    // width, and its sibling (the top block with the chart) ends up a different
                    // width → an empty unfilled region appears to the right of the chart. With
                    // `overflow_hidden` the content is clipped instead.
                    let mut slot = div()
                        .relative()
                        .overflow_hidden()
                        .min_w(px(0.0))
                        .min_h(px(0.0))
                        .when(*horizontal, |this| this.h_full())
                        .when(!*horizontal, |this| this.w_full());
                    slot = child.background_policy(cx).apply(slot, p.shell, 1.0);
                    let slot_size = if first_child_flexes && ix == 0 {
                        None
                    } else {
                        sizes.get(ix).copied().flatten()
                    };
                    if let Some(size) = slot_size {
                        // A fixed panel PREFERS its size (`flex_basis`) but YIELDS
                        // (`flex_shrink`) when the container is narrower than the sum of sizes —
                        // otherwise the block doesn't fit the window, its sibling in the vertical
                        // split stays wider, and an empty region appears to the right of the
                        // chart. On widening it returns to its size (basis preserved).
                        slot = slot.flex_basis(px(size)).flex_shrink_1();
                        slot = if *horizontal {
                            slot.h_full()
                        } else {
                            slot.w_full()
                        };
                    } else {
                        // Flexible slot (e.g. the chart): grows and shrinks. No minimum here —
                        // in a dense row (bottom) the sum of minimums would inflate the block's
                        // min-width and break width sync in the vertical split. The base slot
                        // already has min_w(0)/overflow_hidden, so content is clipped, not forced.
                        slot = slot.flex_1();
                    }

                    let mut child_path = path.clone();
                    child_path.push(ix);
                    split = split.child(slot.child(self.render_item(
                        SharedString::from(format!("{id_text}:split:{ix}")),
                        root,
                        child_path,
                        child,
                        cx,
                    )));
                }
                split.into_any_element()
            }
        }
    }
}

impl Render for DockArea {
    /// Render the live dock, retaining tab activation even when structural editing is locked.
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = MoonPalette::active(cx);
        let dock = cx.entity();
        let mut root = div()
            .id(ElementId::from(self.id.clone()))
            .relative()
            .size_full()
            .overflow_hidden()
            .flex()
            .flex_col();
        root = self.background_policy.apply(root, p.shell, 1.0);
        root = root.child(
            canvas(
                {
                    let dock = dock.clone();
                    move |bounds, _, cx| {
                        dock.update(cx, |area, _| area.root_bounds = bounds);
                    }
                },
                |_, _, _, _| {},
            )
            .absolute()
            .size_full(),
        );
        // Only while a handle is held: an idle dock installs no window listeners at all.
        if let Some((_, cursor)) = self.resize_active {
            root = root.child(self.resize_pointer_hook(cursor, cx));
        }

        if let Some(panel_name) = self.zoomed_panel.as_ref() {
            if let Some(panel) = self.find_panel_named(panel_name.as_ref(), cx) {
                return root.child(
                    TabPanel::new(
                        SharedString::from(format!("{}:zoom", self.id)),
                        vec![panel.clone()],
                    )
                    .dock_context(cx.entity().downgrade(), DockRoot::Center, Vec::new())
                    .layout_editable(self.layout_editable)
                    .detach_allowed(self.detach_allowed)
                    .close_allowed(self.close_allowed)
                    .pinned_leading_panels(self.pinned_leading_panels.clone())
                    .background_policy(self.tab_background_policy)
                    .content_background_policy(
                        self.content_background_policy
                            .unwrap_or_else(|| panel.background_policy(cx)),
                    ),
                );
            }
        }

        let mut row = div().relative().flex_1().flex().overflow_hidden();
        row = row.child(
            canvas(
                {
                    let dock = dock.clone();
                    move |bounds, _, cx| {
                        dock.update(cx, |area, _| area.row_bounds = bounds);
                    }
                },
                |_, _, _, _| {},
            )
            .absolute()
            .size_full(),
        );

        if let Some((item, size, true)) = &self.left {
            row = row.child(
                div()
                    .relative()
                    .w(px(*size))
                    .h_full()
                    .child(self.render_item(
                        SharedString::from(format!("{}:left", self.id)),
                        DockRoot::Left,
                        Vec::new(),
                        item,
                        cx,
                    )),
            );
            row = row.child(self.resize_handle(
                SharedString::from(format!("{}:left-resize", self.id)),
                true,
                DockResizeTarget::OuterLeft,
                cx,
            ));
        }

        // `min_w(0)`: a flex item's min-width defaults to auto (= min-content), so without it
        // the center wrapper could not shrink below its CONTENT min-width (set by a dense bottom
        // row of side-by-side panels). On window narrowing the center overflowed it and the top
        // block with the chart did not stretch to full width → an empty region on the right.
        // `min_w(0)` + the row's `overflow_hidden` let the center shrink to the window.
        row = row.child(
            div()
                .relative()
                .flex_1()
                .h_full()
                .min_w(px(0.))
                .child(self.render_item(
                    SharedString::from(format!("{}:center", self.id)),
                    DockRoot::Center,
                    Vec::new(),
                    &self.center,
                    cx,
                )),
        );

        if let Some((item, size, true)) = &self.right {
            row = row.child(self.resize_handle(
                SharedString::from(format!("{}:right-resize", self.id)),
                true,
                DockResizeTarget::OuterRight,
                cx,
            ));
            row = row.child(
                div()
                    .relative()
                    .w(px(*size))
                    .h_full()
                    .child(self.render_item(
                        SharedString::from(format!("{}:right", self.id)),
                        DockRoot::Right,
                        Vec::new(),
                        item,
                        cx,
                    )),
            );
        }

        root = root.child(row);

        if let Some((item, size, true)) = &self.bottom {
            root = root.child(self.resize_handle(
                SharedString::from(format!("{}:bottom-resize", self.id)),
                false,
                DockResizeTarget::OuterBottom,
                cx,
            ));
            root = root.child(
                div()
                    .relative()
                    .h(px(*size))
                    .w_full()
                    .child(self.render_item(
                        SharedString::from(format!("{}:bottom", self.id)),
                        DockRoot::Bottom,
                        Vec::new(),
                        item,
                        cx,
                    )),
            );
        }

        root
    }
}

#[cfg(test)]
mod tests;
