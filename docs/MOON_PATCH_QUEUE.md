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
   - macOS Control-click policy: Zed rewrites a ctrl-left press into a right
     click unconditionally, dropping the Control flag and the click count with
     it, which makes a ctrl-left application gesture unreachable. MoonUI passes
     the press through by default and keeps the editor convention as an opt-in
     (`gpui::set_macos_control_click_as_secondary`). A re-sync will bring the
     unconditional rewrite back in `moon-gpui-macos/src/window.rs` — keep the
     call to `macos_control_click_rewrite` instead.
   - `Window::is_text_input_active`: reports whether the drawn frame installed a
     text-input handler. An application binding a bare key (Caps Lock, a lone
     modifier) needs it, because those arrive whatever has focus.
   - regular pointer tooltips require 800 ms of continuous hover, expire after
     five visible seconds, and stay suppressed until pointer re-entry; hoverable
     tooltips remain persistent while either their trigger or content is hovered.
   - `UniformList::on_visible_range`: a channel for observing the item range the
     list draws. The renderer closure cannot serve as one — `measure_item` runs
     it on a single item, twice per frame, before the real range exists — so a
     consumer wired through the renderer sees a phantom one-item range. A
     re-sync drops the field, the builder and its call in `prepaint`; restore
     all three, and with them both halves of the reporting rule: an empty list
     reports `0..0`, while a list that holds rows but renders none of them
     stays silent.
   - `KeystrokeEvent::is_held`: keystroke observers and interceptors receive the
     platform's auto-repeat flag, so an app-level interceptor can spend a repeat
     before actions and element listeners see it.
   - Input modality gates auto-repeat on the pointer (`InputModalityState` in
     `window.rs`): once the mouse has moved after a key press, that key's
     repeats no longer flip the modality back to `Keyboard`. Upstream counts
     every `KeyDown`, and with the pointer moving under a held key that
     alternated a whole-window `refresh` at the repeat rate. Only where the
     backend reports `is_held` (Windows, macOS, Wayland, web) — X11 always sends
     `false`, so there every repeat still counts as a press. A re-sync restores
     the bare `last_input_modality` field and its `KeyDown(_)` arm; keep the
     state struct and its unit tests.


   - `Window::content_zoom` / `set_content_zoom`: browser-style page zoom per
     window. The `scale_factor` field holds the platform factor times the zoom,
     `viewport_size` the platform content size divided by it, both recomputed
     by `sync_platform_geometry` from `bounds_changed` and from a zoom change.
     Pointer input is divided on entry (`unzoom_input`, at the top of
     `dispatch_event` before anything reads the event: positions,
     `ScrollDelta::Pixels` and file-drop positions), and `set_client_inset`,
     `show_window_menu`, `set_traffic_light_position` and the accessibility
     click fallback multiply on the way back, as do
     `PlatformInputHandler::{bounds_for_range, selected_bounds}` while
     `character_index_for_point` divides, so every platform's IME geometry
     converts in one place. `resize`, like `bounds()`, stays in the platform's
     pixels so a saved size restores unchanged. A zoom requested while drawing
     is parked in `pending_content_zoom` and applied at the top of the next
     `draw`; an immediate apply cancels a parked one and rescales the tracked
     pointer. `GpuFrameInfo` and `GpuCanvasTextContext` carry `content_zoom`
     beside the combined `scale_factor` for a canvas that keeps device density,
     and the retained text cache key includes it. A re-sync drops the two
     fields and their init in `Window::new`, `sync_platform_geometry`,
     `content_zoom` / `set_content_zoom` / `apply_content_zoom`, `unzoom_input`
     and its call, the block in `draw`, the setters' and the a11y click's
     multiplications, the `PlatformInputHandler` conversions, the
     `content_zoom` field and parameter of `GpuFrameInfo` /
     `GpuCanvasTextContext` (forwarded by `draw_retained_text_layer`) and the
     `#[cfg(test)] mod tests;` line; restore all of them and keep
     `window/tests.rs`.

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
