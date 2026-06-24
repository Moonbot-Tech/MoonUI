#![recursion_limit = "512"]

//! Moon UI public facade.
//!
//! The component implementation currently lives in `moon-ui-components`, a
//! source-compatible fork of Longbridge `gpui-component`. The public API for
//! Moon applications is exported from this crate so app code can depend on
//! `moon_ui::*` instead of the fork-internal crate name.

pub mod foundation {
    pub use gpui_component::moon::foundation::*;
}

pub use gpui_component::moon::*;
pub use gpui_component_assets::Assets as MoonAssets;
