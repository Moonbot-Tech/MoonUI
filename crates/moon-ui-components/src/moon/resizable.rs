//! Moon-facing facade for Longbridge resizable panel primitives.

use gpui::ElementId;

pub type MoonResizablePanel = crate::resizable::ResizablePanel;
pub type MoonResizablePanelEvent = crate::resizable::ResizablePanelEvent;
pub type MoonResizablePanelGroup = crate::resizable::ResizablePanelGroup;
pub type MoonResizableState = crate::resizable::ResizableState;

pub fn moon_h_resizable(id: impl Into<ElementId>) -> MoonResizablePanelGroup {
    crate::resizable::h_resizable(id)
}

pub fn moon_resizable_panel() -> MoonResizablePanel {
    crate::resizable::resizable_panel()
}

pub fn moon_v_resizable(id: impl Into<ElementId>) -> MoonResizablePanelGroup {
    crate::resizable::v_resizable(id)
}
