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

### Bundled themes

Four palettes ship in the crate, each a `MoonPalette` constant in
`moon/tokens.rs` paired with a theme file under `crates/moon-ui-components/themes/`:

| theme | file | palette | colour set | skin |
|---|---|---|---|---|
| Terminal | `moon-terminal.toml` | `MoonPalette::TERMINAL` | dark | flat |
| Light | `moon-light.toml` | `MoonPalette::LIGHT` | light | flat |
| Graphite | `moon-graphite.toml` | `MoonPalette::GRAPHITE` | dark | flat |
| Obsidian | `moon-obsidian.toml` | `MoonPalette::OBSIDIAN` | dark | facet |

The constant and the file hold the same values on purpose — the constant is what
code reads, the file is what a host installs — so a change to one is a change to
both, and a test pins the pair.

The skin is the manner in which a theme side draws depth, `MoonSkin` in
`moon/skin.rs`, and it is separate from the palette. `Flat` is a solid fill and a
hairline, and is what a theme file gets when it names no skin. `Facet` lights
each surface from above — a gradient fill, a one-pixel edge, a highlight and a
shadow — with every colour derived from the palette by the rules in that module,
so it needs no palette entries of its own. A component consults the skin only to
draw the faceted look; under `Flat` it must paint exactly what it painted before
skins existed. The skin is set per side: Obsidian's light side is the flat light
theme.

Three rules of the faceted look are easy to break from a component:

- **A blur is written as the design gives it.** The design's shadows and glows
  are CSS `box-shadow` values, where the gaussian's standard deviation is half
  the blur; the renderer takes the standard deviation itself as its blur radius.
  `facet`, `facet_tinted`, `facet_glow` and the backdrop lights convert on the
  way in, so a component passes the design's number to them. A shadow built
  directly with `box_shadow(..)` is not converted, and a design blur passed that
  way renders twice as wide and far fainter than designed.
- **A small socket is not a well.** A checkbox box, a radio circle and a toggle
  track take `MoonFacetKind::Socket`: the well's fill and shadows under an edge
  that still reads at a dozen pixels. The plain `Well` edge is right for an
  input and disappears on a control that small.
- **The ambient light belongs to the window, not to a panel.** `MoonRoot` paints
  `moon_backdrop` when it owns an opaque window fill. A host whose root paints
  nothing, because something shows through it, lays `moon_ambient_light` inside
  an opaque band of its own, clipped to that band, and never over what shows
  through.

The rules hold on a light palette too: the gallery's "Light (facet)" style
(`--theme light-facet`) draws `MoonPalette::LIGHT` with the Facet skin. It is a
demonstration, not a bundled theme.

Two rules bind a new bundled palette:

- **Which side it is on is MEASURED, never declared.** `MoonPalette::is_light`
  reads the luminance of `shell`, so a palette lands on the light or dark side by
  its own colours. That is why Graphite needs no `ThemeMode` of its own: it is a
  dark theme with softer surfaces, and every consumer that branches on `is_light`
  takes the dark arm with no new code. Adding a `ThemeMode` variant to carry a
  palette is the wrong instrument.
- **`text` and `text_muted` must reach 4.5:1 against `shell`, `window` and
  `panel`**, measured by `contrast_ratio` (WCAG 2.x). A palette may not borrow an
  ink from a neighbouring theme that fails this: Graphite lifts its own
  `text_muted` precisely because the dark palette's value measures 3.03:1 on a
  mid-tone surface. Fix legibility in the palette's own tokens, never by adding a
  third colour table for consumers to branch on.

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
not as a long list of Rust type names. The tabs are `GALLERY_PAGES` in
`crates/moon-ui-gallery/src/main.rs`, in this order. The heading each page
opens with is in `gallery.rs` (Checkboxes and Toggles open from their own
modules):

- Controls;
- Inputs;
- Data — "Tables / Lists / Dock";
- Overlays — "Menus / Overlays";
- Layout — "Navigation / Status / Tokens", which includes the window-frame
  variants;
- NewControls — "NewControls / Ready Moon adaptations";
- Composites — "Composites / Ready Moon adaptations";
- Stateful — "Stateful / Ready Moon adaptations";
- Checkboxes — checkboxes and radios;
- Toggles.

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
