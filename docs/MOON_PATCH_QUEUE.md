# MoonUI Patch Queue

MoonUI is maintained as a Git patch stack, not as a repeatedly overwritten
generated tree.

## Branches

- `upstream-clean`: clean standalone GPUI extraction from a fixed Zed revision.
- `master`: MoonUI product branch, rebased on top of `upstream-clean`.

## Current Upstream Base

- Zed commit: `84b753cb51441f104fc35b540b9fe77a409f4529`
- Local source used for the extraction: `R:\test\_zed_gpui_base_84b753`

Regeneration command:

```powershell
cargo xtask transform --zed-tag v0.0.0 --zed-path R:\test\_zed_gpui_base_84b753 --output crates --local
```

## Patch Layers On `master`

0. Standalone extraction hygiene on `upstream-clean`:
   - `xtask` materializes Apache license files instead of preserving symlinks
     or Windows text pointers such as `../../LICENSE`.
   - Root `LICENSE` is canonical Apache-2.0 text.
   - Every Apache crate has a real `LICENSE-APACHE` file. No pointer files,
     no symlink-only licenses, no BOM-only Windows artifacts.

1. Moon GPUI runtime patches:
   - `gpu_canvas`
   - `GpuCanvasDriver::frame() -> Skip | RequestPresent`
   - raw GPU hooks for DirectX, Metal, and wgpu
   - retained GPU canvas prepare/draw integration
   - visible-canvas pacing hooks where the platform needs them

2. Zed bugfix candidates kept separate from `gpu_canvas` when possible:
   - Windows DPI/restore-bounds behavior
   - Linux/X11 borderless decoration fallback
   - These are useful upstream fixes on their own. Do not hide them inside
     feature work when preparing Zed pull requests.

3. Moon UI components:
   - `moon-ui`
   - `moon-ui-components`
   - component assets and macros

4. MoonUI-only integration:
   - GPL helper crates removed from the extraction.
   - Runtime shader/font fallbacks and component integration needed by Moonbot.

Do not mix terminal application logic into this repository.

## Update Rule

When Zed moves:

1. Update `upstream-clean` by regenerating the clean extraction from the new Zed
   revision and committing that generated result.
2. Rebase `master` onto the updated `upstream-clean`.
3. Resolve only real conflicts.
4. Run platform checks before pushing.

This is the key rule. `xtask` creates the clean base; Git carries the MoonUI
patches forward.
