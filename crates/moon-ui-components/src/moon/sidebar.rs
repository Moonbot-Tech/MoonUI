//! Moon-facing sidebar exports.
//!
//! Sidebar is a Mirror component: it keeps Longbridge's collapsed layout,
//! animated clip-width, menu hierarchy and virtualized content path. Styling is
//! provided by synchronized `sidebar_*` Moon theme colors.

pub use crate::sidebar::{
    Sidebar as MoonSidebar, SidebarCollapsible as MoonSidebarCollapsible,
    SidebarFooter as MoonSidebarFooter, SidebarGroup as MoonSidebarGroup,
    SidebarHeader as MoonSidebarHeader, SidebarItem as MoonSidebarItem,
    SidebarMenu as MoonSidebarMenu, SidebarMenuItem as MoonSidebarMenuItem,
    SidebarToggleButton as MoonSidebarToggleButton,
};
