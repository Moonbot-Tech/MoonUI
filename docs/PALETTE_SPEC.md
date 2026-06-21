# MoonUI Palette Specification

## Purpose

MoonUI is the canonical UI runtime and component palette for MoonBot products.
The palette is not a screenshot archive, a type-coverage checklist, or a thin
re-export layer. It must provide real, reusable, MoonBot-themed components with
working behavior, stable APIs, and clear ownership boundaries.

The main consumer today is MoonTerminal, but MoonUI must remain a reusable
library. Terminal-specific business concepts, market data, strategy logic,
exchange state, and chart rendering must not leak into generic palette
components.

## Source Of Truth

The visual source of truth is the MoonBot terminal design reference:

```text
R:\test\chart-test\for_dis\MoonBot Terminal Design.html
```

The HTML reference should be viewed through a local HTTP server, not through
`file://`.

Component behavior may reuse and adapt proven Longbridge/Zed patterns when they
already solve lifecycle, focus, input, docking, table, overlay, or virtualization
problems. Longbridge visual style is not the design reference.

## Architecture Boundary

MoonUI owns:

- reusable components;
- component state models;
- MoonBot theme tokens, metrics, typography, and scaling;
- reusable window chrome and overlay infrastructure;
- GPUI runtime integration required by these components;
- gallery/showcase applications that prove component behavior.

MoonUI does not own:

- terminal business panels as product-specific entities;
- chart rendering internals;
- exchange or strategy domain logic;
- app-specific persistence policies except generic serialization hooks;
- one-off widgets that cannot be reused outside the current application.

Applications must depend on the public `moon-ui` crate and consume components
through `moon_ui::*` or documented public submodules. Applications must not
reach into internal component crates unless explicitly documented as a temporary
migration exception.

## Component Adaptation Rule

When a Longbridge component has useful behavior but the wrong visual style or
missing MoonBot hooks, the correct action is:

1. keep or port the behavior engine into MoonUI;
2. expose the missing theme, metric, state, or builder API in MoonUI;
3. apply MoonBot design tokens inside the component or its Moon wrapper;
4. use the adapted Moon component from the application.

Do not recreate complex behavior by hand in application screens only to get the
right colors. Do not use raw Longbridge components directly when their visuals
or public API bypass the MoonUI theme contract.

## Theme And Metrics

Design-relevant numbers belong in MoonUI theme, metrics, or typography systems
when they affect reusable component appearance:

- font sizes and line heights;
- row heights;
- toolbar heights;
- input heights;
- button paddings;
- border radius;
- icon sizes;
- component gaps;
- hover, focus, active, disabled, and selected colors;
- overlay shadows and borders;
- table and list density;
- dock, tab, and chrome dimensions.

Hard-coded local numbers are allowed only when they are truly component-local
geometry and not a design token. If the same number or relationship appears in
more than one component, it should usually become a named token or be derived
from an existing token.

Font scaling and global UI scaling must be architecture-level capabilities, not
per-screen hacks. Increasing UI font size should not require hunting through
application panels.

## Real Component Requirement

A component is not considered implemented in the palette merely because:

- its type exists;
- it is listed in a coverage manifest;
- it compiles;
- a static mock of it is visible;
- a screenshot happens to contain something visually similar.

A component is considered implemented only when its primary user-facing behavior
works in the gallery and in at least one realistic usage path, or when the
component is explicitly documented as visual-only.

Examples:

- A button must respond to click, hover, disabled, loading, icon, and selected
  states where applicable.
- An input must support focus, editing, selection, validation, masks, and
  disabled/read-only states where applicable.
- A table must prove scrolling, selection, sorting, resizing, clipping, and large
  data behavior where applicable.
- A menu or popover must prove open/close lifecycle, placement, edge behavior,
  selection, and dismissal.
- A dock must prove tab selection, tab reorder, split/drop, resize, zoom, detach
  events, and host integration where applicable.
- A window frame must prove the intended visual type and the hit-test/control
  behavior that belongs to that type.

## Gallery Contract

`moon-ui-gallery` is the canonical human-facing component showcase.

The gallery must be organized as a small number of meaningful horizontal pages,
not as a long list of Rust type names. The current intended grouping is:

- Controls;
- Inputs;
- Data;
- Overlays;
- Layout / Windows.

Each page must show realistic compositions of real components. The gallery may
keep an internal coverage manifest for tests, but that manifest must not become
the user interface.

For every visual component included in the palette, the gallery must either:

- demonstrate its real behavior; or
- explicitly mark it as intentionally visual-only; or
- link it to a documented temporary gap.

The gallery is not complete while any interactive component is represented only
as a decorative static picture.

## Gallery Acceptance Checklist

Before calling the gallery ready, verify:

- every showcased interactive component changes state in response to user
  actions;
- overlays open and close through real component APIs;
- context menus, popovers, dropdowns, and tooltips are not permanently forced
  open unless the page is specifically demonstrating that state;
- tables and lists clip text correctly and do not steal unrelated scroll axes;
- dock panels can perform the behaviors the gallery claims to showcase;
- detached or secondary windows use the shared Moon window APIs;
- no page is a raw Rust type checklist;
- no component silently falls back to Longbridge visual defaults;
- the gallery builds and runs on Windows/MSVC;
- platform-specific behavior is either tested on the platform or marked as
  unverified.

## Verification

Minimum verification for component or gallery changes:

```powershell
cargo fmt --all -- --check
cargo check -p moon-ui-gallery
cargo test -p moon-ui-gallery
```

For Windows visual work, also build the runnable binary:

```powershell
$vcvars = 'C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat'
cmd.exe /d /s /c "`"$vcvars`" && cargo build -p moon-ui-gallery --target x86_64-pc-windows-msvc"
```

Compile success is not enough. Any component whose purpose is visual or
interactive must also be opened and checked in the gallery or in a realistic app
screen.

## Temporary Exceptions

Temporary exceptions are allowed only when they are explicit and actionable.
They must say:

- what is incomplete;
- why it is incomplete;
- what component/API should replace it;
- where the follow-up is tracked.

An exception must not be hidden behind a passing coverage test.

## Definition Of Done

A MoonUI palette change is done only when:

- the public API is reusable outside MoonTerminal;
- behavior is real, not decorative;
- visuals follow MoonBot design tokens;
- component state and event lifecycles are wired correctly;
- the gallery demonstrates the relevant behavior;
- application code can use the component without ad-hoc visual recreation;
- build and tests pass;
- known gaps are documented instead of silently shipped.
