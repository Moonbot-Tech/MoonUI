//! Private drag payloads and interaction policy for dock editing.

use gpui::*;

use super::{DockRoot, TileMeta};

/// Identifies the dock edge or split boundary targeted by a resize drag.
#[derive(Clone, Debug)]
pub(super) enum DockResizeTarget {
    OuterLeft,
    OuterRight,
    OuterBottom,
    Split {
        root: DockRoot,
        path: Vec<usize>,
        after_ix: usize,
    },
}

/// Carries a dock tab and its structural location through a tab drag.
#[derive(Clone, Debug)]
pub(super) struct DockTabDrag {
    pub(super) dock_id: EntityId,
    pub(super) root: DockRoot,
    pub(super) path: Vec<usize>,
    pub(super) panel_name: SharedString,
    /// Whether this panel may participate in split drops (true only for panels with
    /// `show_dock_header`). A split drop is accepted only when BOTH the dragged panel and
    /// the target slot are splittable — so e.g. a bottom dock panel can split the bottom
    /// strip but cannot be dropped into the chart slot.
    pub(super) splittable: bool,
}

/// Interaction capabilities for one dock tab under the current edit policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct TabInteractionPolicy {
    pub(super) draggable: bool,
    pub(super) accepts_drop: bool,
    pub(super) detachable: bool,
}

/// Keep pinned tabs fixed while leaving them available as reorder destinations.
///
/// Args:
///     layout_editable: Whether structural dock edits are enabled.
///     pinned: Whether this tab belongs to the leading pinned prefix.
///     detach_allowed: Whether the host permits native-window detachment.
///
/// Returns:
///     The independent drag-source, drop-target, and detach capabilities for the tab.
pub(super) fn tab_interaction_policy(
    layout_editable: bool,
    pinned: bool,
    detach_allowed: bool,
) -> TabInteractionPolicy {
    TabInteractionPolicy {
        draggable: layout_editable && !pinned,
        accepts_drop: layout_editable,
        detachable: layout_editable && !pinned && detach_allowed,
    }
}

impl Render for DockTabDrag {
    /// Render no visual content because the value is only a drag payload.
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

/// Describes whether a tile drag moves or resizes the selected tile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DockTileDragKind {
    Move,
    ResizeRight,
    ResizeBottom,
    ResizeBottomRight,
}

/// Carries one tile's structural location and drag operation through GPUI routing.
#[derive(Clone, Debug)]
pub(super) struct DockTileDrag {
    pub(super) dock_id: EntityId,
    pub(super) root: DockRoot,
    pub(super) path: Vec<usize>,
    pub(super) ix: usize,
    pub(super) kind: DockTileDragKind,
}

impl Render for DockTileDrag {
    /// Render no visual content because the value is only a drag payload.
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

/// Captures the pointer and tile geometry recorded when a tile drag begins.
#[derive(Clone, Debug)]
pub(super) struct DockTileDragStart {
    pub(super) root: DockRoot,
    pub(super) path: Vec<usize>,
    pub(super) ix: usize,
    pub(super) cursor: Point<Pixels>,
    pub(super) meta: TileMeta,
}
