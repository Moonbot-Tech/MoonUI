//! Private drag payload elements for data-table column movement and resizing.

use gpui::{Context, Empty, EntityId, IntoElement, Render, SharedString, Window};

/// Identifies the retained table state and column participating in a resize drag.
#[derive(Clone, Debug)]
pub(super) struct MoonDataColumnResizeDrag {
    pub(super) state_id: EntityId,
    pub(super) key: String,
}

impl Render for MoonDataColumnResizeDrag {
    /// Render no visual content because the payload exists only for GPUI drag routing.
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

/// Identifies the retained table state and column participating in a move drag.
#[derive(Clone, Debug)]
pub(super) struct MoonDataColumnDrag {
    pub(super) state_id: EntityId,
    pub(super) key: SharedString,
}

impl Render for MoonDataColumnDrag {
    /// Render no visual content because the payload exists only for GPUI drag routing.
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}
