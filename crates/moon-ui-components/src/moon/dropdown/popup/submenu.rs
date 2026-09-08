//! Place deferred submenu content beside its measured parent row within the viewport.

use gpui::*;
use std::{cell::Cell, rc::Rc};

/// A deferred portal whose parent-row bounds are captured before its prepaint pass.
pub(super) struct SubmenuPlacement {
    pub row_bounds: Rc<Cell<Bounds<Pixels>>>,
    pub gap: f32,
    pub top_overlap: f32,
    pub child: AnyElement,
}

impl IntoElement for SubmenuPlacement {
    type Element = Self;

    /// Preserve the portal as a layout element.
    fn into_element(self) -> Self {
        self
    }
}

impl Element for SubmenuPlacement {
    type RequestLayoutState = LayoutId;
    type PrepaintState = ();

    /// The surrounding menu owns retained identity.
    fn id(&self) -> Option<ElementId> {
        None
    }

    /// Internal geometry has no separate inspector source.
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    /// Measure the menu outside normal row flow before choosing its opening side.
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, LayoutId) {
        let child = self.child.request_layout(window, cx);
        let layout = window.request_layout(
            Style {
                position: Position::Absolute,
                ..Style::default()
            },
            [child],
            cx,
        );
        (layout, child)
    }

    /// Prefer the row's right side, flip to its left, then clamp both axes to the viewport.
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        child: &mut LayoutId,
        window: &mut Window,
        cx: &mut App,
    ) {
        let row = self.row_bounds.get();
        let size = window.layout_bounds(*child).size;
        let viewport = window.viewport_size();
        let margin = px(6.0);
        let right = row.right() + px(self.gap);
        let left = row.left() - px(self.gap) - size.width;
        let x = if right + size.width <= viewport.width - margin {
            right
        } else {
            left
        };
        let x = x.min(viewport.width - size.width - margin).max(margin);
        let y = (row.top() - px(self.top_overlap))
            .min(viewport.height - size.height - margin)
            .max(px(0.0));
        window.with_element_offset(point(x, y) - bounds.origin, |window| {
            self.child.prepaint(window, cx)
        });
    }

    /// Paint at the position retained by the child's prepaint pass.
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut LayoutId,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.child.paint(window, cx);
    }
}
