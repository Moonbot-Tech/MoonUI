# Moon UI Gallery

Runnable component gallery for MoonUI.

The gallery is a visual/API coverage app: every Moon visual component that is
already part of the public `moon_ui` facade should appear here through that
facade, not through terminal code and not through private implementation paths.

Run:

```powershell
cargo run -p moon-ui-gallery
```

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
