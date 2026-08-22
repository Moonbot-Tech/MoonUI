//! Private tree mutations and persistence projections for DockItem.

use std::rc::Rc;

use gpui::{App, SharedString, WeakEntity, Window};

use super::{
    DockArea, DockItem, DockPanelSlot, DockTopologyNode, MoonBackgroundPolicy, PanelInfo,
    PanelState, PanelView, TileMeta, default_tile_meta,
};

impl DockItem {
    pub(super) fn with_panel_added(self, panel: Rc<dyn PanelView>) -> Self {
        match self {
            DockItem::Empty => DockItem::Panel(panel),
            DockItem::Panel(existing) => DockItem::Tabs {
                items: vec![existing, panel],
                active_ix: 1,
            },
            DockItem::Tabs {
                mut items,
                active_ix: _,
            } => {
                items.push(panel);
                DockItem::Tabs {
                    active_ix: items.len() - 1,
                    items,
                }
            }
            DockItem::Split { .. } | DockItem::Tiles { .. } => DockItem::Tabs {
                items: vec![panel],
                active_ix: 0,
            },
        }
    }

    /// Insert `panel` into the first `Tabs` node that already contains a panel whose
    /// name is in `sibling_names`, at `ix` (clamped to the tab count), making it active.
    /// Used to restore a detached/closed panel back to its "home" tab strip WITHOUT
    /// collapsing the surrounding split (unlike `with_panel_added`, which replaces a
    /// whole `Split` with a single `Tabs`). Returns true if it was inserted.
    pub(super) fn insert_into_named_tabs(
        &mut self,
        panel: Rc<dyn PanelView>,
        ix: usize,
        sibling_names: &[&str],
        dock_area: &WeakEntity<DockArea>,
        window: &mut Window,
        cx: &mut App,
    ) -> bool {
        match self {
            DockItem::Tabs { items, active_ix } => {
                if items.iter().any(|p| {
                    sibling_names
                        .iter()
                        .any(|n| p.panel_name(cx).as_ref() == *n)
                }) {
                    let ix = ix.min(items.len());
                    panel.on_added_to(dock_area.clone(), window, cx);
                    items.insert(ix, panel);
                    *active_ix = ix;
                    return true;
                }
                false
            }
            DockItem::Panel(existing) => {
                // Slot holds a lone sibling (panels were split apart): merge the returning
                // panel with it into a Tabs so it rejoins as a tab — instead of failing and
                // letting the caller fall back to a destructive full-window add.
                if sibling_names
                    .iter()
                    .any(|n| existing.panel_name(cx).as_ref() == *n)
                {
                    panel.on_added_to(dock_area.clone(), window, cx);
                    let existing = existing.clone();
                    let (items, active_ix) = if ix == 0 {
                        (vec![panel, existing], 0)
                    } else {
                        (vec![existing, panel], 1)
                    };
                    *self = DockItem::Tabs { items, active_ix };
                    return true;
                }
                false
            }
            DockItem::Split { items, .. } => {
                for it in items.iter_mut() {
                    if it.insert_into_named_tabs(
                        panel.clone(),
                        ix,
                        sibling_names,
                        dock_area,
                        window,
                        cx,
                    ) {
                        return true;
                    }
                }
                false
            }
            _ => false,
        }
    }

    pub(super) fn notify_added(
        &self,
        dock_area: &WeakEntity<DockArea>,
        window: &mut Window,
        cx: &mut App,
    ) {
        match self {
            DockItem::Empty => {}
            DockItem::Panel(panel) => panel.on_added_to(dock_area.clone(), window, cx),
            DockItem::Tabs { items, .. } => {
                for panel in items {
                    panel.on_added_to(dock_area.clone(), window, cx);
                }
            }
            DockItem::Tiles { items, .. } => {
                for panel in items {
                    panel.on_added_to(dock_area.clone(), window, cx);
                }
            }
            DockItem::Split { items, .. } => {
                for item in items {
                    item.notify_added(dock_area, window, cx);
                }
            }
        }
    }

    pub(super) fn is_empty(&self) -> bool {
        matches!(self, DockItem::Empty)
    }

    pub(super) fn background_policy(&self, cx: &App) -> MoonBackgroundPolicy {
        match self {
            DockItem::Empty => MoonBackgroundPolicy::Opaque,
            DockItem::Panel(panel) => panel.background_policy(cx),
            DockItem::Tabs { items, active_ix } => items
                .get((*active_ix).min(items.len().saturating_sub(1)))
                .map(|panel| panel.background_policy(cx))
                .unwrap_or(MoonBackgroundPolicy::Opaque),
            // Composite containers can mix transparent chart hosts and opaque UI
            // panels. Their own surface must stay unpainted; each child slot
            // applies its own policy.
            DockItem::Split { .. } | DockItem::Tiles { .. } => MoonBackgroundPolicy::NoFill,
        }
    }

    pub(super) fn remove_panel_named(
        self,
        panel_name: &str,
        window: &mut Window,
        cx: &mut App,
    ) -> (Self, bool) {
        let (item, removed) = self.extract_panels_named(panel_name, cx, |ix| TileMeta {
            x: 0.0,
            y: 0.0,
            w: 240.0,
            h: 160.0,
            z_index: ix,
        });
        for panel in &removed {
            panel.on_removed(window, cx);
        }
        (item, !removed.is_empty())
    }

    pub(super) fn into_panels(self) -> Vec<Rc<dyn PanelView>> {
        match self {
            DockItem::Empty => Vec::new(),
            DockItem::Panel(panel) => vec![panel],
            DockItem::Tabs { items, .. } => items,
            DockItem::Tiles { items, .. } => items,
            DockItem::Split { items, .. } => items
                .into_iter()
                .flat_map(|item| item.into_panels())
                .collect(),
        }
    }

    pub(super) fn find_panel_named(&self, panel_name: &str, cx: &App) -> Option<Rc<dyn PanelView>> {
        match self {
            DockItem::Empty => None,
            DockItem::Panel(panel) => {
                (panel.panel_name(cx).to_string() == panel_name).then_some(panel.clone())
            }
            DockItem::Tabs { items, .. } | DockItem::Tiles { items, .. } => items
                .iter()
                .find(|panel| panel.panel_name(cx).to_string() == panel_name)
                .cloned(),
            DockItem::Split { items, .. } => items
                .iter()
                .find_map(|item| item.find_panel_named(panel_name, cx)),
        }
    }

    /// Find a named panel and record the live node that owns its activation state.
    pub(super) fn panel_location(
        &self,
        panel_name: &str,
        cx: &App,
        path: &mut Vec<usize>,
    ) -> Option<(Vec<usize>, DockPanelSlot)> {
        match self {
            DockItem::Empty => None,
            DockItem::Panel(panel) => (panel.panel_name(cx).as_ref() == panel_name)
                .then(|| (path.clone(), DockPanelSlot::Panel)),
            DockItem::Tabs { items, .. } => items
                .iter()
                .position(|panel| panel.panel_name(cx).as_ref() == panel_name)
                .map(|ix| (path.clone(), DockPanelSlot::Tab(ix))),
            DockItem::Tiles { items, .. } => items
                .iter()
                .position(|panel| panel.panel_name(cx).as_ref() == panel_name)
                .map(|ix| (path.clone(), DockPanelSlot::Tile(ix))),
            DockItem::Split { items, .. } => {
                for (ix, item) in items.iter().enumerate() {
                    path.push(ix);
                    let found = item.panel_location(panel_name, cx, path);
                    path.pop();
                    if found.is_some() {
                        return found;
                    }
                }
                None
            }
        }
    }

    /// Append every live panel identity once while retaining topology traversal order.
    pub(super) fn append_unique_panels(&self, panels: &mut Vec<Rc<dyn PanelView>>) {
        let mut append = |panel: &Rc<dyn PanelView>| {
            if !panels.iter().any(|known| Rc::ptr_eq(known, panel)) {
                panels.push(panel.clone());
            }
        };
        match self {
            DockItem::Empty => {}
            DockItem::Panel(panel) => append(panel),
            DockItem::Tabs { items, .. } | DockItem::Tiles { items, .. } => {
                for panel in items {
                    append(panel);
                }
            }
            DockItem::Split { items, .. } => {
                for item in items {
                    item.append_unique_panels(panels);
                }
            }
        }
    }

    /// Count raw panel occurrences so duplicate identities cannot hide behind normalized topology.
    pub(super) fn panel_occurrence_count(&self) -> usize {
        match self {
            DockItem::Empty => 0,
            DockItem::Panel(_) => 1,
            DockItem::Tabs { items, .. } | DockItem::Tiles { items, .. } => items.len(),
            DockItem::Split { items, .. } => {
                items.iter().map(DockItem::panel_occurrence_count).sum()
            }
        }
    }

    /// Project one live node to payload-free panel names and normalized geometry.
    pub(super) fn topology_by_name(&self, cx: &App) -> DockTopologyNode {
        match self {
            DockItem::Empty => DockTopologyNode::Empty,
            DockItem::Panel(panel) => DockTopologyNode::Panel {
                name: panel.panel_name(cx).to_string(),
            },
            DockItem::Tabs { items, .. } => DockTopologyNode::Tabs {
                names: items
                    .iter()
                    .map(|panel| panel.panel_name(cx).to_string())
                    .collect(),
            },
            DockItem::Tiles { items, metas } => DockTopologyNode::Tiles {
                names: items
                    .iter()
                    .map(|panel| panel.panel_name(cx).to_string())
                    .collect(),
                metas: metas.clone(),
            },
            DockItem::Split {
                horizontal,
                items,
                sizes,
            } => DockTopologyNode::Split {
                horizontal: *horizontal,
                items: items.iter().map(|item| item.topology_by_name(cx)).collect(),
                sizes: (0..items.len())
                    .map(|ix| sizes.get(ix).copied().flatten())
                    .collect(),
            },
        }
    }

    /// Add repaired missing panels to the first compatible center node without discarding it.
    pub(super) fn append_repaired_panels(&mut self, panels: Vec<Rc<dyn PanelView>>) {
        if panels.is_empty() {
            return;
        }
        match self {
            DockItem::Empty => {
                *self = if panels.len() == 1 {
                    DockItem::Panel(panels.into_iter().next().expect("one panel"))
                } else {
                    DockItem::Tabs {
                        items: panels,
                        active_ix: 0,
                    }
                };
            }
            DockItem::Panel(existing) => {
                let mut items = vec![existing.clone()];
                items.extend(panels);
                *self = DockItem::Tabs {
                    items,
                    active_ix: 0,
                };
            }
            DockItem::Tabs { items, .. } => items.extend(panels),
            DockItem::Tiles { items, metas } => {
                for panel in panels {
                    let ix = items.len();
                    items.push(panel);
                    metas.push(default_tile_meta(ix));
                }
            }
            DockItem::Split { items, .. } => {
                if let Some(first) = items.first_mut() {
                    first.append_repaired_panels(panels);
                }
            }
        }
    }

    /// Reorder pinned panels within tabs and before every sibling at horizontal split ancestors.
    ///
    /// Args:
    ///     pinned: Stable panel names that must lead their tab/split topology.
    ///     cx: Application context used to resolve live panel names.
    ///
    /// Returns:
    ///     Whether any tab order or horizontal split order changed.
    pub(super) fn enforce_pinned_leading(&mut self, pinned: &[SharedString], cx: &App) -> bool {
        match self {
            DockItem::Tabs { items, active_ix } => {
                let previous_active = items.get(*active_ix).cloned();
                let previous = items
                    .iter()
                    .map(|panel| Rc::as_ptr(panel) as *const () as usize)
                    .collect::<Vec<_>>();
                items.sort_by_key(|panel| {
                    pinned
                        .iter()
                        .position(|name| name.as_ref() == panel.panel_name(cx).as_ref())
                        .map(|ix| (0, ix))
                        .unwrap_or((1, usize::MAX))
                });
                if let Some(active) = previous_active {
                    *active_ix = items
                        .iter()
                        .position(|panel| Rc::ptr_eq(panel, &active))
                        .unwrap_or(0);
                }
                previous
                    != items
                        .iter()
                        .map(|panel| Rc::as_ptr(panel) as *const () as usize)
                        .collect::<Vec<_>>()
            }
            DockItem::Split {
                horizontal,
                items,
                sizes,
            } => {
                let mut changed = items.iter_mut().fold(false, |changed, item| {
                    item.enforce_pinned_leading(pinned, cx) || changed
                });
                if *horizontal {
                    let previous = items
                        .iter()
                        .map(|item| item.contains_any_panel(pinned, cx))
                        .collect::<Vec<_>>();
                    if previous
                        .iter()
                        .skip_while(|pinned| **pinned)
                        .any(|pinned| *pinned)
                    {
                        let previous_sizes = std::mem::take(sizes);
                        let mut paired = std::mem::take(items)
                            .into_iter()
                            .enumerate()
                            .map(|(ix, item)| {
                                let size = previous_sizes.get(ix).copied().flatten();
                                (item, size, ix)
                            })
                            .collect::<Vec<_>>();
                        paired.sort_by_key(|(item, _, ix)| {
                            (!item.contains_any_panel(pinned, cx), *ix)
                        });
                        for (item, size, _) in paired {
                            items.push(item);
                            sizes.push(size);
                        }
                        changed = true;
                    }
                }
                changed
            }
            DockItem::Empty | DockItem::Panel(_) | DockItem::Tiles { .. } => false,
        }
    }

    /// Return whether this subtree contains any configured pinned panel name.
    ///
    /// Args:
    ///     names: Stable panel names considered pinned.
    ///     cx: Application context used to resolve live panel names.
    ///
    /// Returns:
    ///     `true` when at least one requested panel exists anywhere below this item.
    pub(super) fn contains_any_panel(&self, names: &[SharedString], cx: &App) -> bool {
        names
            .iter()
            .any(|name| self.find_panel_named(name.as_ref(), cx).is_some())
    }

    /// Return the path to the smallest subtree containing every requested panel that is present.
    ///
    /// Missing names are ignored because their panels may have been detached or closed. Returning
    /// the whole matching subtree preserves nested sibling splits during split restoration.
    pub(super) fn smallest_subtree_with_all(&self, names: &[&str], cx: &App) -> Option<Vec<usize>> {
        let present: Vec<&str> = names
            .iter()
            .copied()
            .filter(|n| self.find_panel_named(n, cx).is_some())
            .collect();
        if present.is_empty() {
            return None;
        }
        self.smallest_node_with(&present, cx)
    }

    /// Return the path to the smallest node containing every supplied panel name.
    ///
    /// Descend while one child still contains the complete set; an empty path identifies this
    /// node when no child can contain the set alone.
    fn smallest_node_with(&self, names: &[&str], cx: &App) -> Option<Vec<usize>> {
        if !names.iter().all(|n| self.find_panel_named(n, cx).is_some()) {
            return None;
        }
        if let DockItem::Split { items, .. } = self {
            for (i, child) in items.iter().enumerate() {
                if let Some(mut sub) = child.smallest_node_with(names, cx) {
                    let mut path = vec![i];
                    path.append(&mut sub);
                    return Some(path);
                }
            }
        }
        Some(Vec::new())
    }

    /// Return any panel name in this node except `exclude`.
    ///
    /// The name acts as a stable split-slot anchor after taking a panel collapses the old node.
    pub(super) fn first_panel_name_excluding(&self, exclude: &str, cx: &App) -> Option<String> {
        match self {
            DockItem::Empty => None,
            DockItem::Panel(p) => {
                let n = p.panel_name(cx).to_string();
                (n != exclude).then_some(n)
            }
            DockItem::Tabs { items, .. } | DockItem::Tiles { items, .. } => items
                .iter()
                .map(|p| p.panel_name(cx).to_string())
                .find(|n| n != exclude),
            DockItem::Split { items, .. } => items
                .iter()
                .find_map(|it| it.first_panel_name_excluding(exclude, cx)),
        }
    }

    /// Return the path from this item to the panel, tabs, or tiles node containing `name`.
    pub(super) fn find_panel_path(&self, name: &str, cx: &App) -> Option<Vec<usize>> {
        match self {
            DockItem::Empty => None,
            DockItem::Panel(p) => (p.panel_name(cx).as_ref() == name).then(Vec::new),
            DockItem::Tabs { items, .. } | DockItem::Tiles { items, .. } => items
                .iter()
                .any(|p| p.panel_name(cx).as_ref() == name)
                .then(Vec::new),
            DockItem::Split { items, .. } => items.iter().enumerate().find_map(|(i, it)| {
                it.find_panel_path(name, cx).map(|mut sub| {
                    let mut path = vec![i];
                    path.append(&mut sub);
                    path
                })
            }),
        }
    }

    /// Push a panel into the first tabs node without collapsing the center through
    /// `with_panel_added`; return whether a tabs node was found.
    pub(super) fn try_push_into_first_tabs(&mut self, panel: Rc<dyn PanelView>) -> bool {
        match self {
            DockItem::Tabs { items, active_ix } => {
                items.push(panel);
                *active_ix = items.len() - 1;
                true
            }
            DockItem::Split { items, .. } => {
                for it in items.iter_mut() {
                    if it.try_push_into_first_tabs(panel.clone()) {
                        return true;
                    }
                }
                false
            }
            _ => false,
        }
    }

    pub(super) fn take_panel_named(
        self,
        panel_name: &str,
        cx: &App,
    ) -> (Self, Option<Rc<dyn PanelView>>) {
        match self {
            DockItem::Empty => (DockItem::Empty, None),
            DockItem::Panel(panel) => {
                if panel.panel_name(cx).to_string() == panel_name {
                    (DockItem::Empty, Some(panel))
                } else {
                    (DockItem::Panel(panel), None)
                }
            }
            DockItem::Tabs { items, active_ix } => {
                let mut taken = None;
                let mut kept = Vec::with_capacity(items.len());
                for panel in items {
                    if panel.panel_name(cx).to_string() == panel_name {
                        taken = Some(panel);
                    } else {
                        kept.push(panel);
                    }
                }
                let item = match kept.len() {
                    0 => DockItem::Empty,
                    1 => DockItem::Panel(kept.remove(0)),
                    len => DockItem::Tabs {
                        items: kept,
                        active_ix: active_ix.min(len.saturating_sub(1)),
                    },
                };
                (item, taken)
            }
            DockItem::Tiles { items, metas } => {
                let mut taken = None;
                let mut kept_items = Vec::with_capacity(items.len());
                let mut kept_metas = Vec::with_capacity(items.len());
                for (ix, panel) in items.into_iter().enumerate() {
                    if panel.panel_name(cx).to_string() == panel_name {
                        taken = Some(panel);
                    } else {
                        kept_items.push(panel);
                        kept_metas.push(metas.get(ix).copied().unwrap_or(TileMeta {
                            x: 12.0 + ix as f32 * 18.0,
                            y: 12.0 + ix as f32 * 18.0,
                            w: 320.0,
                            h: 200.0,
                            z_index: ix,
                        }));
                    }
                }
                let item = match kept_items.len() {
                    0 => DockItem::Empty,
                    1 => DockItem::Panel(kept_items.remove(0)),
                    _ => DockItem::Tiles {
                        items: kept_items,
                        metas: kept_metas,
                    },
                };
                (item, taken)
            }
            DockItem::Split {
                horizontal,
                items,
                sizes,
            } => {
                let mut taken = None;
                let mut kept_items = Vec::new();
                let mut kept_sizes = Vec::new();
                for (ix, item) in items.into_iter().enumerate() {
                    let (item, child_taken) = item.take_panel_named(panel_name, cx);
                    if taken.is_none() {
                        taken = child_taken;
                    }
                    if !item.is_empty() {
                        kept_items.push(item);
                        kept_sizes.push(sizes.get(ix).copied().unwrap_or(None));
                    }
                }
                let item = match kept_items.len() {
                    0 => DockItem::Empty,
                    1 => kept_items.remove(0),
                    _ => DockItem::Split {
                        horizontal,
                        items: kept_items,
                        sizes: kept_sizes,
                    },
                };
                (item, taken)
            }
        }
    }

    /// Remove every occurrence of one stable panel name and return identities in tree order.
    ///
    /// Args:
    ///     panel_name: Stable logical name to extract from this topology node.
    ///     cx: Application context used to resolve live panel names.
    ///     missing_tile_meta: Caller-specific fallback retained for malformed tile metadata.
    ///
    /// Returns:
    ///     The normalized topology node and every removed occurrence, including duplicate `Rc`s.
    pub(super) fn extract_panels_named<F>(
        self,
        panel_name: &str,
        cx: &App,
        missing_tile_meta: F,
    ) -> (Self, Vec<Rc<dyn PanelView>>)
    where
        F: Fn(usize) -> TileMeta + Copy,
    {
        match self {
            DockItem::Empty => (DockItem::Empty, Vec::new()),
            DockItem::Panel(panel) => {
                if panel.panel_name(cx).as_ref() == panel_name {
                    (DockItem::Empty, vec![panel])
                } else {
                    (DockItem::Panel(panel), Vec::new())
                }
            }
            DockItem::Tabs { items, active_ix } => {
                let mut kept = Vec::with_capacity(items.len());
                let mut removed = Vec::new();
                for panel in items {
                    if panel.panel_name(cx).as_ref() == panel_name {
                        removed.push(panel);
                    } else {
                        kept.push(panel);
                    }
                }
                let item = match kept.len() {
                    0 => DockItem::Empty,
                    1 => DockItem::Panel(kept.remove(0)),
                    len => DockItem::Tabs {
                        items: kept,
                        active_ix: active_ix.min(len.saturating_sub(1)),
                    },
                };
                (item, removed)
            }
            DockItem::Tiles { items, metas } => {
                let mut kept_items = Vec::with_capacity(items.len());
                let mut kept_metas = Vec::with_capacity(items.len());
                let mut removed = Vec::new();
                for (ix, panel) in items.into_iter().enumerate() {
                    if panel.panel_name(cx).as_ref() == panel_name {
                        removed.push(panel);
                    } else {
                        kept_items.push(panel);
                        kept_metas.push(
                            metas
                                .get(ix)
                                .copied()
                                .unwrap_or_else(|| missing_tile_meta(ix)),
                        );
                    }
                }
                let item = match kept_items.len() {
                    0 => DockItem::Empty,
                    1 => DockItem::Panel(kept_items.remove(0)),
                    _ => DockItem::Tiles {
                        items: kept_items,
                        metas: kept_metas,
                    },
                };
                (item, removed)
            }
            DockItem::Split {
                horizontal,
                items,
                sizes,
            } => {
                let mut kept_items = Vec::new();
                let mut kept_sizes = Vec::new();
                let mut removed = Vec::new();
                for (ix, item) in items.into_iter().enumerate() {
                    let (item, child_removed) =
                        item.extract_panels_named(panel_name, cx, missing_tile_meta);
                    removed.extend(child_removed);
                    if !item.is_empty() {
                        kept_items.push(item);
                        kept_sizes.push(sizes.get(ix).copied().unwrap_or(None));
                    }
                }
                let item = match kept_items.len() {
                    0 => DockItem::Empty,
                    1 => kept_items.remove(0),
                    _ => DockItem::Split {
                        horizontal,
                        items: kept_items,
                        sizes: kept_sizes,
                    },
                };
                (item, removed)
            }
        }
    }

    pub(super) fn dump(&self, cx: &App) -> PanelState {
        match self {
            DockItem::Empty => PanelState::empty(),
            DockItem::Panel(panel) => panel.dump(cx),
            DockItem::Tabs { items, active_ix } => {
                let mut state = PanelState::new("tabs").info(PanelInfo::tabs(*active_ix));
                state.children = items.iter().map(|panel| panel.dump(cx)).collect();
                state
            }
            DockItem::Tiles { items, metas } => {
                let mut state = PanelState::new("tiles").info(PanelInfo::Tiles {
                    metas: metas.clone(),
                });
                state.children = items.iter().map(|panel| panel.dump(cx)).collect();
                state
            }
            DockItem::Split {
                horizontal,
                items,
                sizes,
            } => {
                // Keep one size entry per panel so `sizes` stays aligned with `items`. A flexible
                // panel is serialized as the 0.0 sentinel that `to_item` restores to `None`.
                // Flattening the options would shift later sizes onto the wrong panels after load.
                let sizes: Vec<f32> = (0..items.len())
                    .map(|i| sizes.get(i).copied().flatten().unwrap_or(0.0))
                    .collect();
                let mut state = PanelState::new("stack").info(PanelInfo::stack(sizes, *horizontal));
                state.children = items.iter().map(|item| item.dump(cx)).collect();
                state
            }
        }
    }
}
