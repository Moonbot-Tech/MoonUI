//! Moon-facing sheet exports.
//!
//! Sheets are owned by `Root` through `MoonWindowExt`; callers should not draw a
//! local fake side panel when a root-owned overlay/sheet is what they mean.

pub use crate::sheet::{Sheet as MoonSheet, SheetSettings as MoonSheetSettings};
