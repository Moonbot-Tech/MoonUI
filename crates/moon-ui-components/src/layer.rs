//! Named deferred-draw priority bands for MoonUI overlay surfaces.
//!
//! GPUI deferred draws are sorted into one global ascending order by `priority` and painted in
//! that order (`moon-gpui` `window.rs`, `deferred_draw_traversal_order`). This file is the only
//! place these bands are numbered. A higher number paints later and therefore on top.
//!
//! The band is absolute, not relative to the hosting surface: a child overlay inside a
//! higher-band parent must opt into a band above that parent or it is painted underneath.
//! `MoonSelect::in_popover()` (`moon/select.rs:378`) is the existing opt-in; the managed
//! tooltip avoids the problem by sitting above every band unconditionally.

/// Longbridge default overlay band: plain popovers, select menus, comboboxes, context menus,
/// native-menu fallback, and dropdown submenus.
pub(crate) const LAYER_OVERLAY: usize = 1;

/// Moon popover and Moon dropdown surfaces.
pub(crate) const LAYER_MOON_POPOVER: usize = 30_000;

/// A menu opened from inside a Moon popover (`MoonSelect::in_popover`).
pub(crate) const LAYER_MOON_POPOVER_MENU: usize = 31_000;

/// The managed tooltip overlay. The tooltip is the topmost transient surface and must outrank
/// every other deferred band.
pub(crate) const LAYER_TOOLTIP: usize = 100_000;

#[cfg(test)]
mod tests;
