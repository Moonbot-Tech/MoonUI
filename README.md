<p align="center">
  <a href="https://moonbot.pro">
    <img src="assets/moonbot-logo-full.svg" alt="Moonbot" width="220">
  </a>
</p>

# MoonUI

MoonUI is Moonbot's standalone GPUI runtime and component workspace.

It is extracted from a fixed Zed commit, then maintained as a small patch stack:
clean standalone GPUI first, Moon runtime patches next, Moon UI components last.
This keeps updates from upstream Zed reviewable instead of turning the repository
into an opaque generated snapshot.

## Repository Layout

- `upstream-clean` is the clean standalone GPUI extraction from Zed.
- `master` is `upstream-clean` plus Moon runtime patches and Moon components.
- `crates/moon-gpui` is the GPUI facade used by applications.
- `crates/moon-gpui-platform` selects Windows, macOS, Linux, web, and headless backends.
- `crates/moon-ui` is the public Moon UI facade for application code.
- `crates/moon-ui-components` is the Moon-maintained component port. Its Rust crate
  name remains `gpui_component` to keep port diffs readable.

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
paths = [
    "../MoonUI/crates/moon-gpui",
    "../MoonUI/crates/moon-gpui-platform",
    "../MoonUI/crates/moon-ui",
]
```

Do not commit that override. It is only for local editing.

## Build

Windows MSVC:

```powershell
cmd.exe /d /s /c 'call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat" >nul && "C:\files\utils\rust\cargo\bin\cargo.exe" check -p moon-gpui --target x86_64-pc-windows-msvc'
cmd.exe /d /s /c 'call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat" >nul && "C:\files\utils\rust\cargo\bin\cargo.exe" check -p moon-ui --target x86_64-pc-windows-msvc'
```

macOS requires the full Xcode Metal toolchain. Linux requires the GPUI Linux
backend dependencies used by Zed.

## Updating From Zed

Regenerate a clean extraction from the new Zed revision on `upstream-clean`,
commit it there, then rebase `master` onto the updated `upstream-clean`.

Use path dependencies for the repository image. That is the default:

```powershell
cargo run -p xtask -- transform --zed-tag v0.0.0 --zed-path R:\path\to\zed --output crates
```

`--versioned-deps` is only for a publish-style experiment where MoonUI crates
exist as versioned packages. Do not use it for the Git repository image.

That workflow lets Git do a real three-way merge. Most upstream movement merges
without noise, and real conflicts appear only where MoonUI's patches touch the
same code.

---

<p align="center">
  Moonbot<br>
  Advanced terminal for cryptocurrency trading<br>
  <a href="https://moonbot.pro">moonbot.pro</a>
</p>
