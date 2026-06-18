<p align="center">
  <a href="https://moonbot.pro">
    <img src="assets/moonbot-logo-full.svg" alt="Moonbot" width="220">
  </a>
</p>

# MoonUI

MoonUI is Moonbot's standalone GPUI runtime and component workspace.

Applications use this repository directly from Cargo. Developers who need to
edit the runtime or components can clone MoonUI next to the application and add
a local `[patch]` override without changing public manifests.

## Repository Layout

- `crates/moon-gpui` is the GPUI facade used by applications.
- `crates/moon-gpui-platform` selects Windows, macOS, Linux, web, and headless backends.
- `crates/moon-ui` is the public Moon UI facade for application code.
- `crates/moon-ui-components` is the Moon-maintained component port. Its Rust crate
  name remains `gpui_component` to keep port diffs readable.
- `docs/MOON_PATCH_QUEUE.md` describes the maintainer workflow for updating the
  standalone GPUI extraction from upstream Zed.

## Use From Cargo

Applications should depend on the public Git repository:

```toml
gpui = { package = "moon-gpui", git = "https://github.com/Moonbot-Tech/MoonUI", branch = "master" }
gpui_platform = { package = "moon-gpui-platform", git = "https://github.com/Moonbot-Tech/MoonUI", branch = "master", features = ["font-kit", "wayland", "x11"] }
moon-ui = { package = "moon-ui", git = "https://github.com/Moonbot-Tech/MoonUI", branch = "master" }
```

For local development, keep the public `Cargo.toml` unchanged and add a private
path override in the consuming application's `.cargo/config.toml`:

```toml
[patch."https://github.com/Moonbot-Tech/MoonUI"]
moon-gpui = { path = "../MoonUI/crates/moon-gpui" }
moon-gpui-platform = { path = "../MoonUI/crates/moon-gpui-platform" }
moon-ui = { path = "../MoonUI/crates/moon-ui" }
```

Do not commit that override. It is only for local editing. Prefer `[patch]`
over top-level `paths`: it replaces the same Git source without changing the
shape of the dependency graph.

## Build

Windows MSVC:

```powershell
cmd.exe /d /s /c 'call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat" >nul && "C:\files\utils\rust\cargo\bin\cargo.exe" check -p moon-gpui --target x86_64-pc-windows-msvc'
cmd.exe /d /s /c 'call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat" >nul && "C:\files\utils\rust\cargo\bin\cargo.exe" check -p moon-ui --target x86_64-pc-windows-msvc'
```

macOS requires the full Xcode Metal toolchain. Linux requires the GPUI Linux
backend dependencies used by Zed.

## Licensing

MoonUI preserves upstream licensing from the projects it is built from.

- GPUI-derived crates are based on Zed and carry their original Zed license
  metadata. See the root `LICENSE` and crate-level `LICENSE-APACHE` files.
- Zed's GPL `zlog` / `ztracing` helper crates are intentionally not extracted.
  The remaining extracted GPUI crates in this repository are Apache-2.0.
- Moon UI component crates are an Apache-2.0 port of Longbridge
  `gpui-component`. The component crate also keeps its upstream copyright
  notice in `crates/moon-ui-components/LICENSE-APACHE`.
- Moonbot-specific additions are distributed under Apache-2.0 unless a file or
  crate manifest says otherwise.

---

<p align="center">
  Moonbot<br>
  Advanced terminal for cryptocurrency trading<br>
  <a href="https://moonbot.pro">moonbot.pro</a>
</p>
