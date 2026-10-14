use crate::{
    ActiveTheme, Collapsible, h_flex,
    sidebar::{SidebarItem, SidebarMenu, SidebarMenuItem},
    v_flex,
};
use gpui::{
    App, ElementId, IntoElement, ParentElement, SharedString, Styled as _, Window, div,
    prelude::FluentBuilder as _, px,
};
use std::any::Any;

/// A group of items in the [`super::Sidebar`].
#[derive(Clone)]
pub struct SidebarGroup<E: SidebarItem + 'static> {
    label: SharedString,
    collapsed: bool,
    children: Vec<E>,
}

impl<E: SidebarItem> SidebarGroup<E> {
    /// Create a new [`SidebarGroup`] with the given label.
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            collapsed: false,
            children: Vec::new(),
        }
    }

    /// Add a child to the sidebar group, the child should implement [`SidebarItem`].
    pub fn child(mut self, child: E) -> Self {
        self.children.push(child);
        self
    }

    /// Add multiple children to the sidebar group.
    ///
    /// See also [`SidebarGroup::child`].
    pub fn children(mut self, children: impl IntoIterator<Item = E>) -> Self {
        self.children.extend(children);
        self
    }
}

impl<E: SidebarItem> Collapsible for SidebarGroup<E> {
    fn is_collapsed(&self) -> bool {
        self.collapsed
    }

    fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }
}

impl<E: SidebarItem> SidebarItem for SidebarGroup<E> {
    /// Render children without adding gaps outside their padded hit rows.
    fn render(
        self,
        id: impl Into<ElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> impl IntoElement {
        let id = id.into();
        let count = self.children.len();
        let half_gap = window.rem_size() * 0.25;

        v_flex()
            .relative()
            .when(!self.collapsed, |this| {
                this.child(
                    h_flex()
                        .flex_shrink_0()
                        .px_2()
                        .rounded(cx.theme().radius)
                        .text_xs()
                        .text_color(cx.theme().sidebar_foreground.opacity(0.7))
                        .h_8()
                        .child(self.label),
                )
            })
            .child(
                v_flex()
                    .gap_0()
                    .children(self.children.into_iter().enumerate().map(|(ix, child)| {
                        let top = if ix == 0 { px(0.0) } else { half_gap };
                        let bottom = if ix + 1 == count { px(0.0) } else { half_gap };
                        // Direct menu items need the same edge trimming as a SidebarMenu.
                        // Other SidebarItem implementations retain their own rendering contract.
                        if let Some(item) = (&child as &dyn Any).downcast_ref::<SidebarMenuItem>() {
                            return item
                                .clone()
                                .collapsed(self.collapsed)
                                .render_padded(format!("{}-{}", id, ix), top, bottom, window, cx)
                                .into_any_element();
                        }
                        if let Some(menu) = (&child as &dyn Any).downcast_ref::<SidebarMenu>() {
                            return menu
                                .clone()
                                .collapsed(self.collapsed)
                                .render_padded(format!("{}-{}", id, ix), top, bottom, window, cx)
                                .into_any_element();
                        }
                        div()
                            .when(ix > 0, |this| this.mt(half_gap * 2.0))
                            .child(child.collapsed(self.collapsed).render(
                                format!("{}-{}", id, ix),
                                window,
                                cx,
                            ))
                            .into_any_element()
                    })),
            )
    }
}
