/// Bundled SVG icons for Moon UI components.
///
/// The Rust library name is `gpui_component_assets`. `moon-ui` re-exports
/// [`Assets`] as `MoonAssets`. `moon-ui-components` generates `IconName` from
/// `assets/icons` using the directory this crate's build script publishes.
///
/// ## Usage
///
/// Native builds pass the unit struct straight through:
///
/// ```rust,no_run
/// use gpui_component_assets::Assets;
///
/// let app = gpui_platform::application().with_assets(Assets);
/// ```
///
/// ## Platform differences
///
/// - **Native**: `assets/icons/**/*.svg` is embedded with RustEmbed.
///   [`Assets::new`] ignores its endpoint.
/// - **WASM**: icons are not embedded. [`Assets::new`] stores an endpoint, and
///   `load` fetches a path with `reqwest` from `{endpoint}/assets/{path}` only
///   when that path starts with `icons/` and ends with `.svg`. The bytes are
///   cached in memory. Until the download finishes, `load` returns an error.
///   The sprite atlas does not keep that failure, so a later paint calls
///   `load` again.
#[cfg(not(target_family = "wasm"))]
mod native_assets;

#[cfg(target_family = "wasm")]
mod wasm_assets;

#[cfg(not(target_family = "wasm"))]
pub use native_assets::Assets;

#[cfg(target_family = "wasm")]
pub use wasm_assets::Assets;
