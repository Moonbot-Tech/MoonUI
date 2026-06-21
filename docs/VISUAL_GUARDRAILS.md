# MoonUI Visual Guardrails

MoonUI refactors must preserve two things independently:

1. Component behavior/API, checked by Rust tests and `xtask` contracts.
2. Component appearance, checked by the gallery screenshot runner.

The visual runner compares against committed golden PNGs. Pixel output still
depends on OS, DPI, font rasterization and GPU backend, so the threshold is
small but non-zero. The baseline lives under
`crates/moon-ui-gallery/snapshots/baseline` and is part of the repo.

The gallery owns the snapshot flow. It switches pages itself and writes PNG
files through `moon-ui-gallery --features snapshot -- --snapshot-dir ...`.
The PowerShell wrapper only starts that internal mode and compares the resulting
PNGs.

## Workflow

Before a risky UI refactor:

```powershell
powershell -ExecutionPolicy Bypass -File tools\capture-gallery-snapshots.ps1 -UpdateBaseline
```

Only run `-UpdateBaseline` when the visual change is intentional and reviewed.

After the refactor:

```powershell
powershell -ExecutionPolicy Bypass -File tools\capture-gallery-snapshots.ps1 -Compare
```

The runner opens `moon-ui-gallery` once per theme, then the gallery switches
through all pages itself:

- Controls
- Inputs
- Data
- Overlays
- Layout
- NewControls
- Composites
- Stateful

Each page is captured in both Dark and Light mode as a PNG and compared against
the committed baseline (`Dark-*` and `Light-*`). The gallery clears its snapshot
directory before writing new PNGs and waits before capturing so stale frames do
not masquerade as valid snapshots. A component refactor is not visually clean if
the diff exceeds the configured threshold and the change was not intentional.

On Windows, if `Window::render_to_image()` is not available, the gallery uses a
Win32 client-area capture fallback. That fallback brings the gallery window to
the front, keeps it topmost for the capture, and moves the cursor away from the
taskbar before `BitBlt` so Windows shell previews are not baked into component
snapshots. The long-term target is still a real backend `render_to_image`
implementation.

## What This Catches

- Missing checkbox glyph assets.
- White or transparent fill regressions.
- Broken button/input/table colors.
- Light-theme regressions hidden by the terminal dark palette.
- Accidental size, radius, padding, and hover-state drift visible in the gallery.
- Components added to the manifest but forgotten in the gallery coverage test.

## Hard Rule

Do not call a MoonUI visual refactor done from code review alone. At minimum run:

```powershell
cargo test -p moon-ui-components
cargo test -p moon-ui-gallery
cargo xtask component-audit --check-baseline
cargo xtask component-api --check-baseline
tools\run-component-guardrails.ps1
powershell -ExecutionPolicy Bypass -File tools\capture-gallery-snapshots.ps1 -Compare
```

If there is no local visual baseline yet, create it first with `-UpdateBaseline`
before editing, then review and commit the generated baseline files deliberately.
