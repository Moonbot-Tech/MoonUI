//! Persisted dock state and name-based topology projections.

use std::{
    borrow::BorrowMut,
    collections::{HashMap, HashSet},
};

use gpui::{App, Window};
use serde::{Deserialize, Serialize};

use super::{DOCK_TILE_MIN_H, DOCK_TILE_MIN_W, DockItem, DockPlacement, MoonPanelRegistry};

/// Persisted state for the center area and optional side docks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DockAreaState {
    #[serde(default)]
    pub version: Option<usize>,
    pub center: PanelState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left_dock: Option<DockState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub right_dock: Option<DockState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bottom_dock: Option<DockState>,
}

impl Default for DockAreaState {
    fn default() -> Self {
        Self {
            version: None,
            center: PanelState::empty(),
            left_dock: None,
            right_dock: None,
            bottom_dock: None,
        }
    }
}

/// Persisted state for one side dock and its placement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DockState {
    pub panel: PanelState,
    pub placement: DockPlacement,
    pub size: f32,
    pub open: bool,
}

/// Recursive serialized state for one panel, tab group, tile group, or split.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PanelState {
    pub panel_name: String,
    #[serde(default)]
    pub children: Vec<PanelState>,
    pub info: PanelInfo,
}

/// Serialized payload describing the structural kind of a [`PanelState`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PanelInfo {
    #[serde(rename = "stack")]
    Stack { sizes: Vec<f32>, axis: usize },
    #[serde(rename = "tabs")]
    Tabs { active_index: usize },
    #[serde(rename = "panel")]
    Panel(serde_json::Value),
    #[serde(rename = "tiles")]
    Tiles { metas: Vec<TileMeta> },
}

/// Persisted freeform position, size, and stacking order for one tile.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TileMeta {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub z_index: usize,
}

/// Serializable panel-name topology for sharing one layout across independent docks.
///
/// The projection contains geometry, side placement, tab order, and sizes. It deliberately
/// excludes panel payloads, active tabs, zoom state, group identifiers, and live panel instances.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DockTopologyByName {
    pub center: DockTopologyNode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left: Option<DockTopologySide>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub right: Option<DockTopologySide>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bottom: Option<DockTopologySide>,
}

/// One optional side root in a name-based dock topology.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DockTopologySide {
    pub item: DockTopologyNode,
    pub size: f32,
    pub open: bool,
}

/// One serializable node in a name-based dock topology.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DockTopologyNode {
    Empty,
    Panel {
        name: String,
    },
    Tabs {
        names: Vec<String>,
    },
    Tiles {
        names: Vec<String>,
        metas: Vec<TileMeta>,
    },
    Split {
        horizontal: bool,
        items: Vec<DockTopologyNode>,
        sizes: Vec<Option<f32>>,
    },
}

impl Default for DockTopologyByName {
    /// Create an empty center topology with no side roots.
    fn default() -> Self {
        Self {
            center: DockTopologyNode::Empty,
            left: None,
            right: None,
            bottom: None,
        }
    }
}

impl PartialEq for DockTopologyByName {
    /// Compare canonical topology while ignoring representational empty, duplicate, and size noise.
    fn eq(&self, other: &Self) -> bool {
        let left = self.normalized();
        let right = other.normalized();
        left.center == right.center
            && left.left == right.left
            && left.right == right.right
            && left.bottom == right.bottom
    }
}

impl DockTopologyByName {
    /// Build a deterministic single-strip topology from the supplied preferred panel names.
    pub fn tab_preset<I, S>(names: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let names = names.into_iter().map(Into::into).collect::<Vec<_>>();
        Self {
            center: DockTopologyNode::Tabs { names },
            left: None,
            right: None,
            bottom: None,
        }
        .normalized()
    }

    /// Return every unique panel name in normalized topology traversal order.
    pub fn panel_names(&self) -> Vec<String> {
        let topology = self.normalized();
        let mut names = Vec::new();
        topology.center.append_names(&mut names);
        for side in [&topology.left, &topology.right, &topology.bottom]
            .into_iter()
            .flatten()
        {
            side.item.append_names(&mut names);
        }
        names
    }

    /// Return a deterministic topology suitable for equality checks and persistence.
    pub fn normalized(&self) -> Self {
        let mut seen = HashSet::new();
        let center = self.center.normalized(&mut seen);
        let left = Self::normalized_side(self.left.as_ref(), &mut seen);
        let right = Self::normalized_side(self.right.as_ref(), &mut seen);
        let bottom = Self::normalized_side(self.bottom.as_ref(), &mut seen);
        Self {
            center,
            left,
            right,
            bottom,
        }
    }

    /// Normalize a side root and discard it when no named panel survives.
    fn normalized_side(
        side: Option<&DockTopologySide>,
        seen: &mut HashSet<String>,
    ) -> Option<DockTopologySide> {
        let side = side?;
        let item = side.item.normalized(seen);
        if matches!(item, DockTopologyNode::Empty) {
            return None;
        }
        Some(DockTopologySide {
            item,
            size: normalized_positive(side.size).unwrap_or(0.0),
            open: side.open,
        })
    }
}

impl DockTopologyNode {
    /// Append the node's panel names in topology traversal order.
    fn append_names(&self, names: &mut Vec<String>) {
        match self {
            Self::Empty => {}
            Self::Panel { name } => names.push(name.clone()),
            Self::Tabs { names: node_names }
            | Self::Tiles {
                names: node_names, ..
            } => {
                names.extend(node_names.iter().cloned());
            }
            Self::Split { items, .. } => {
                for item in items {
                    item.append_names(names);
                }
            }
        }
    }

    /// Normalize one node while retaining the first global occurrence of every panel name.
    fn normalized(&self, seen: &mut HashSet<String>) -> Self {
        match self {
            Self::Empty => Self::Empty,
            Self::Panel { name } => {
                if name.is_empty() || !seen.insert(name.clone()) {
                    Self::Empty
                } else {
                    Self::Panel { name: name.clone() }
                }
            }
            Self::Tabs { names } => {
                let names = unique_names(names, seen);
                match names.as_slice() {
                    [] => Self::Empty,
                    [name] => Self::Panel { name: name.clone() },
                    _ => Self::Tabs { names },
                }
            }
            Self::Tiles { names, metas } => {
                let mut kept_names = Vec::new();
                let mut kept_metas = Vec::new();
                for (ix, name) in names.iter().enumerate() {
                    if !name.is_empty() && seen.insert(name.clone()) {
                        kept_names.push(name.clone());
                        kept_metas.push(normalized_tile_meta(
                            metas
                                .get(ix)
                                .copied()
                                .unwrap_or_else(|| default_tile_meta(ix)),
                        ));
                    }
                }
                match kept_names.as_slice() {
                    [] => Self::Empty,
                    [name] => Self::Panel { name: name.clone() },
                    _ => Self::Tiles {
                        names: kept_names,
                        metas: kept_metas,
                    },
                }
            }
            Self::Split {
                horizontal,
                items,
                sizes,
            } => {
                let mut kept_items = Vec::new();
                let mut kept_sizes = Vec::new();
                for (ix, item) in items.iter().enumerate() {
                    let item = item.normalized(seen);
                    if !matches!(item, Self::Empty) {
                        kept_items.push(item);
                        kept_sizes.push(
                            sizes
                                .get(ix)
                                .copied()
                                .flatten()
                                .and_then(normalized_positive),
                        );
                    }
                }
                match kept_items.len() {
                    0 => Self::Empty,
                    1 => kept_items.remove(0),
                    _ => Self::Split {
                        horizontal: *horizontal,
                        items: kept_items,
                        sizes: kept_sizes,
                    },
                }
            }
        }
    }
}

/// In-memory name-based layout that also retains local active-tab and zoom state.
///
/// Unlike [`DockTopologyByName`], this type is intentionally not serializable or owner-bound. It
/// is used to restore one dock's local runtime state without retaining opaque panel instances.
#[derive(Clone, Debug, PartialEq)]
pub struct DockNamedLayout {
    pub(super) topology: DockTopologyByName,
    pub(super) active_tabs: HashMap<String, String>,
    pub(super) zoomed_panel: Option<String>,
}

impl DockNamedLayout {
    /// Return every panel name captured by this exact local runtime layout.
    pub fn panel_names(&self) -> Vec<String> {
        self.topology.panel_names()
    }
}

/// Retain unique non-empty names in traversal order.
fn unique_names(names: &[String], seen: &mut HashSet<String>) -> Vec<String> {
    names
        .iter()
        .filter(|name| !name.is_empty() && seen.insert((*name).clone()))
        .cloned()
        .collect()
}

/// Keep only finite positive geometry values and canonicalize all invalid values to absent.
fn normalized_positive(value: f32) -> Option<f32> {
    (value.is_finite() && value > 0.0).then_some(value)
}

/// Build the deterministic fallback geometry for a tile without stored metadata.
pub(super) fn default_tile_meta(ix: usize) -> TileMeta {
    TileMeta {
        x: 12.0 + ix as f32 * 18.0,
        y: 12.0 + ix as f32 * 18.0,
        w: 320.0,
        h: 200.0,
        z_index: ix,
    }
}

/// Canonicalize invalid tile geometry without changing valid persisted coordinates.
fn normalized_tile_meta(meta: TileMeta) -> TileMeta {
    TileMeta {
        x: meta.x.is_finite().then_some(meta.x).unwrap_or(0.0),
        y: meta.y.is_finite().then_some(meta.y).unwrap_or(0.0),
        w: normalized_positive(meta.w).unwrap_or(DOCK_TILE_MIN_W),
        h: normalized_positive(meta.h).unwrap_or(DOCK_TILE_MIN_H),
        z_index: meta.z_index,
    }
}

impl Default for PanelInfo {
    fn default() -> Self {
        Self::Panel(serde_json::Value::Null)
    }
}

impl PanelInfo {
    /// Create a leaf-panel payload from panel-specific serialized data.
    pub fn panel(info: serde_json::Value) -> Self {
        Self::Panel(info)
    }

    /// Create a tab-group payload with the supplied active index.
    pub fn tabs(active_index: usize) -> Self {
        Self::Tabs { active_index }
    }

    /// Create a split payload with optional child sizes and the selected axis.
    pub fn stack(sizes: Vec<f32>, horizontal: bool) -> Self {
        Self::Stack {
            sizes,
            axis: usize::from(!horizontal),
        }
    }

    /// Return the active tab index when this payload represents a tab group.
    pub fn active_index(&self) -> Option<usize> {
        match self {
            Self::Tabs { active_index } => Some(*active_index),
            _ => None,
        }
    }
}

impl PanelState {
    /// Create an empty placeholder state with no panel identity or children.
    pub fn empty() -> Self {
        Self {
            panel_name: String::new(),
            children: Vec::new(),
            info: PanelInfo::default(),
        }
    }

    /// Create a leaf state for the supplied serialized panel name.
    pub fn new(panel_name: impl Into<String>) -> Self {
        Self {
            panel_name: panel_name.into(),
            children: Vec::new(),
            info: PanelInfo::default(),
        }
    }

    /// Append one child state to this recursive layout node.
    pub fn child(mut self, child: PanelState) -> Self {
        self.children.push(child);
        self
    }

    /// Replace the structural payload associated with this state node.
    pub fn info(mut self, info: PanelInfo) -> Self {
        self.info = info;
        self
    }

    /// Reconstruct this serialized node as a live dock item through the panel registry.
    pub fn to_item<C>(&self, window: &mut Window, cx: &mut C) -> DockItem
    where
        C: BorrowMut<App>,
    {
        match &self.info {
            PanelInfo::Tabs { active_index } => DockItem::Tabs {
                items: self
                    .children
                    .iter()
                    .flat_map(|child| child.to_item(window, cx).into_panels())
                    .collect(),
                active_ix: *active_index,
            },
            PanelInfo::Stack { axis, .. } => DockItem::Split {
                horizontal: *axis == 0,
                sizes: match &self.info {
                    // Map the 0.0 dump sentinel to None (flex); preserve positive fixed sizes.
                    PanelInfo::Stack { sizes, .. } => {
                        sizes.iter().map(|s| (*s > 0.0).then_some(*s)).collect()
                    }
                    _ => Vec::new(),
                },
                items: self
                    .children
                    .iter()
                    .map(|child| child.to_item(window, cx))
                    .collect(),
            },
            PanelInfo::Tiles { metas } => DockItem::Tiles {
                items: self
                    .children
                    .iter()
                    .flat_map(|child| child.to_item(window, cx).into_panels())
                    .collect(),
                metas: metas.clone(),
            },
            PanelInfo::Panel(_) => DockItem::Panel(MoonPanelRegistry::build_panel(
                &self.panel_name,
                self,
                &self.info,
                window,
                cx,
            )),
        }
    }
}
