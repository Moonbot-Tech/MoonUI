//! Moon-facing facade for native/system menu primitives.
//!
//! Root-owned `MoonContextMenu` and forged `MoonPopupMenu` remain the default
//! in-window overlay APIs. `MoonNativeMenu` is the explicit escape hatch when a
//! menu must be rendered by the operating system instead of GPUI.

pub type MoonNativeMenu = crate::native_menu::NativeMenu;
