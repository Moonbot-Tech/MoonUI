# Moon UI component assets

Bundled SVG icons for Moon UI components.

The Cargo package is `moon-ui-components-assets`. The Rust library name stays
`gpui_component_assets`, and the workspace depends on it as
`gpui-component-assets`. `moon-ui` re-exports `Assets` as `MoonAssets`.

`build.rs` publishes the absolute path of `assets/icons` through the Cargo
`links` key `gpui-component-default-icons`. `moon-ui-components` reads that
path as `DEP_GPUI_COMPONENT_DEFAULT_ICONS_ICONS_DIR` and generates `IconName`
from the SVG files in that directory.

On native targets, `Assets` embeds `assets/icons/**/*.svg` with RustEmbed and
implements `gpui::AssetSource`. It is a unit struct, so callers pass `Assets`
by value. `Assets::new` ignores its endpoint argument.

On `wasm32`, the icons are not embedded. `Assets::new` stores the endpoint.
`load` downloads a path only when it starts with `icons/` and ends with
`.svg`, using `reqwest` against `{endpoint}/assets/{path}`, and keeps the
bytes in memory. Until that download finishes, `load` returns an error. The
sprite atlas does not store the failure, so a later paint calls `load` again.

## License

Apache-2.0
