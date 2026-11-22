# Moon UI Gallery

Runnable component gallery for MoonUI.

The gallery is a visual/API coverage app: every Moon visual component that is
already part of the public `moon_ui` facade should appear here through that
facade, not through terminal code and not through private implementation paths.

Run:

```powershell
cargo run -p moon-ui-gallery
```

Options (both `--flag value` and `--flag=value` forms):

- `--page <name>` opens one of the tabs (`Controls`, `Inputs`, `Data`,
  `Overlays`, `Layout`, `NewControls`, `Composites`, `Stateful`, `Checkboxes`,
  `Toggles`; any letter case). An unknown name opens `Controls`.
- `--theme <name>`: `dark` (default), `light`, `system`, `obsidian` or
  `light-facet`. An unknown name falls back to `dark`.
- `--snapshot-dir <dir>` walks every tab from the first, saves `<Tab>.png`
  into `<dir>` and quits. It deletes the `.png` files already in `<dir>` first.
- `--snapshot-case-dir <dir>` opens the design-handoff case window instead and
  saves `<case id>.png` for every case into `<dir>/<theme>/`, deleting the `.png`
  files already there first. `--snapshot-cases <id,id,...>` captures only
  those cases and leaves the other PNGs in place; unknown ids are ignored, and
  if none is known every case runs, still without deleting anything.

Both snapshot flags need `--features snapshot`; without it the gallery prints
an error and quits. `tools\capture-gallery-snapshots.ps1` drives the page
snapshots for the visual goldens in `snapshots/baseline`.

Check without opening a window:

```powershell
cargo check -p moon-ui-gallery
cargo test -p moon-ui-gallery
```

Coverage rule:

- if a visual component is public and Moon-ready, it must be visible in the
  gallery;
- if it is a token/config/helper rather than a visual element, it must be used
  by a visible component section;
- if it cannot be shown through `moon_ui`, that is an API problem to fix in
  MoonUI, not a reason to import a private crate from the gallery.

`cargo test -p moon-ui-gallery` checks the coverage list behind the first rule:
every `public_path` and `escape_path` in
`crates/moon-ui-components/component-manifest.json` must have its final `::`
segment listed in `COMPONENT_COVERAGE` (`src/main.rs`). It does not check that
the component is actually rendered.
