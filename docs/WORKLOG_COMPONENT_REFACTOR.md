# MoonUI Component Refactor Worklog

## Goal

Finish the MoonUI palette refactor as a verifiable system, not as a one-off
visual patch. The result must keep Moon applications on the Moon-facing API,
make the gallery exercise the actual component set, and provide guardrails that
catch accidental visual, behavioral, and facade regressions.

## Working Rules

- Keep the journal current while working.
- Treat "looks done" as a trigger for a second audit.
- Do not hide regressions by refreshing baselines before understanding them.
- Prefer mechanical checks over promises in documentation.
- Public applications should depend on `moon_ui::*`. Raw Longbridge modules are
  not a public application escape hatch; useful raw components must become
  Moon-facing APIs first.

## Plan And Status

| Item | Status | Notes |
| --- | --- | --- |
| Snapshot current repo state | done | Git status and existing audit/API outputs inspected. |
| Component audit baseline | done | `component-audit` exists and currently passes, but needed tightening. |
| Public API baseline | done | `component-api` exists and catches removed/changed public signatures. |
| Visual gallery snapshots | done | Gallery drives page switching and writes PNGs; PowerShell wrapper compares PNGs. |
| Worklog | done | This file is the active journal. |
| Remove uncontrolled Longbridge facade slurp | done | `moon_ui::components` was removed from the public facade; apps must use `moon_ui::*`. |
| Expand component manifest to match gallery coverage | done | Public gallery-covered components now have manifest ownership. |
| Fix obvious no-op Moon API | done | `MoonButton::mono()`, `MoonCheckbox::mono()/tone()`, `MoonSelect::trigger_variant()`, `MoonSlider::id()`, and uncontrolled `MoonInput` keyed state were fixed. |
| Final validation | done | fmt/check/tests/audits/snapshot compare passed after fixing a Windows capture artifact. |
| Final self-audit | done | Remaining debt is listed below with current mechanical counters. |

## Self-Audit Findings

### Found: uncontrolled public facade escape hatch

`crates/moon-ui/src/lib.rs` exported all of `gpui_component::*` through
`moon_ui::components::*`. That makes it too easy for terminal code or future
apps to bypass Moon wrappers and accidentally mix raw Longbridge controls into a
Moon screen.

Decision: this intermediate state was later superseded. The final facade removes
the public `components` escape hatch entirely; application code imports
Moon-facing APIs from `moon_ui::*`.

### Found: manifest was weaker than gallery coverage

The gallery listed 30+ visual names, while
`crates/moon-ui-components/component-manifest.json` owned only a smaller core
set. That means a visual component could exist in the palette without a
manifest classification.

Decision: expand the manifest and add an audit contract that every public
manifest entry is present in gallery coverage.

### Found: "Debt" severity was used for passing guardrails

Human audit output printed `Pass Debt ...`, which reads like the check itself is
still debt. The intended meaning was "non-critical guardrail".

Decision: rename the severity to `Guardrail` and keep real debt in metrics/docs.

### Found: one public Moon API was a no-op

`MoonButton::mono()` accepted a value and returned `self` without using it.

Decision: wire it into text rendering so button labels/segments can switch to
the mono font.

### Found: `MoonInput` id was not part of state lifecycle

`MoonInput` accepted an id, but uncontrolled inputs created a fresh `InputState`
from render instead of using keyed window state. Parent rerender could therefore
recreate the input state.

Decision: uncontrolled `MoonInput` now uses `window.use_keyed_state` keyed by
the public id, matching the `MoonTextArea` pattern.

### Found: checkbox tone and mono builders were partial

`MoonCheckbox::tone()` and `MoonCheckbox::mono()` existed but did not affect the
underlying checkbox. This was the same bug class as `MoonButton::mono()`.

Decision: the base checkbox now accepts a Moon tone and mono flag; the wrapper
passes both through.

### Found: API snapshot parser mishandled multiline `pub use`

`component-api` stopped at `{` and produced a broken signature for multiline
`pub use gpui_component::{ ... };`.

Decision: multiline `pub use` now waits for the semicolon, and `xtask` has a
unit test covering this exact case.

### Found: Windows visual snapshots could capture shell overlays

The first final `-Compare` run failed because the Windows GDI fallback captured
a taskbar thumbnail preview over the gallery window. That was not a component
regression, but it made the guardrail nondeterministic.

Decision: the Windows fallback now keeps the gallery window topmost for the
whole capture, moves the cursor away from the taskbar before `BitBlt`, and the
PowerShell wrapper reports the failing page name instead of a bare pixel count.
The baseline was not refreshed to hide this failure; the same baseline passed
after the runner fix.

### Found: final no-op audit still had two public builders

After the first "final" pass, `component-audit` still reported
`noop_public_api_markers = 2`: `MoonSelect::trigger_variant()` and
`MoonSlider::id()`. Treating that as remaining debt would leave exactly the
kind of half-finished API this refactor is meant to prevent.

Decision: `MoonSlider::id()` now overrides the base slider root element id while
preserving the old public signature. `MoonSelect::trigger_variant()` now uses a
new core `Select::trigger_variant()` hook. The default select appearance remains
unchanged unless the builder is called; the snapshot diff caught the accidental
default visual change and it was reverted before updating status.

### Found: local theme colors were still embedded in Moon runtime code

`component-audit` still reported `raw_hex_in_moon_runtime = 18`. Those were
mostly white overlay hovers, black shadows, and one close-button foreground.
Leaving them inline would keep theme behavior split between `MoonPalette` and
individual components.

Decision: add palette tokens for `border_hover`, `shadow`, `overlay`, and
`on_accent`, then move those usages out of component bodies. The terminal
integration check caught the one struct literal in `chartdx`; it now uses
`..moon_ui::MoonPalette::TERMINAL` so future palette fields do not break that
adapter again.

## Remaining Debt Register

- `raw_hex_in_moon_runtime` is now `0`. Runtime Moon components should not add
  new raw theme colors outside `moon/tokens.rs`.
- `noop_public_api_markers` is now `0`. The audit baseline was updated so any
  new no-op public builder becomes a regression.
- `Radio` still uses old `cx.theme()` styling in places. It is not currently a
  Moon public surface in the gallery manifest, but it is a real future cleanup
  item if radio enters the Moon-facing palette.
- Windows gallery snapshots use a GDI client-capture fallback because the
  Windows backend still reports `render_to_image` as unimplemented. This is a
  useful visual guardrail, but backend render capture is the cleaner final
  target.

## 2026-06-21 Finalization Audit

The final plan in `R:\test\newfork\MOONUI_COMPONENT_FINAL_PLAN.md` changes the
bar from "the current terminal still works" to "the component library cannot
hide unfinished Longbridge surfaces." The first live audit after that plan found
these then-current non-ideal areas:

- The public `moon_ui::components::*` escape hatch was explicit, but it still
  exposed engine controls that were not MoonReady. Terminal code also still used
  `Accordion`, `Notification`, `Progress`, `Tag`, `Theme`, and `WindowExt`
  through that hatch at this point in the refactor.
- The manifest previously listed `dialog`, `notification`, `tree`, and `list`
  as pending, but did not classify other public escape-hatch controls that are
  already visible to applications.
- `MoonRoot` is still a type alias to the base `Root`, so the root contract is
  documented but not represented by a distinct Moon-owned API type.
- The theme bridge is still a manual sync method rather than a compile-checked
  typed conversion from Moon tokens to base theme color/metrics structs.
- The dock API is usable, but it still contains terminal-shaped helpers. Domain
  ordering must stay in applications; MoonUI should own only generic dock
  behavior.

Intermediate decision: add a gallery page named `NewControls`. It contains live components
from the public escape hatch that are not yet admitted as MoonReady. These
controls are also classified as `Pending` in the manifest. Pending controls use
`escape_path`, not `public_path`: `public_path` is reserved for the official
Moon-facing API. A control leaves `NewControls` only after it has a Moon-facing
wrapper/admission decision, a theme contract, behavior coverage, and normal
gallery coverage.

This intermediate decision was later superseded. The final state has no
`Pending` entries and no public `moon_ui::components::*` escape hatch; the
`NewControls` page now shows ready Moon-facing controls only.

Progress after this audit:

- Added `MoonProgress` and moved the terminal header risk meter from raw
  `moon_ui::components::progress::Progress` to `moon_ui::MoonProgress`.
- Added `MoonTag` and moved the terminal exchange pill from raw
  `moon_ui::components::tag::Tag` to `moon_ui::MoonTag`.
- Added `MoonAccordion` / `MoonAccordionItem` and moved the terminal line
  settings accordion from raw `moon_ui::components::accordion::Accordion` to
  `moon_ui::MoonAccordion`.
- Added `MoonNotification` and `MoonWindowExt`; terminal FireTest/assets/
  strategies now push notifications through the Moon-facing API instead of raw
  `moon_ui::components::notification::Notification` and `WindowExt`.
- Added `MoonAlert`. The normal gallery uses `MoonAlert`; raw
  `moon_ui::components::alert::Alert` remains visible only as
  `longbridge_alert` in `NewControls`.
- Added `MoonDialog`, `MoonDialogContent`, and `MoonWindowExt` dialog openers.
  Terminal strategy/assets dialogs now use `open_unique_moon_dialog`; raw
  `moon_ui::components::dialog` was removed from the public facade.
- Removed the remaining terminal-side `moon_ui::components::*` import. The old
  caret-color workaround in MoonTerminal was deleted because `MoonTheme`
  already syncs `BaseTheme.colors.caret` from `MoonPalette.text`.
- Removed raw adapted modules (`accordion`, `alert`, `notification`, `popover`,
  `progress`, `tag`) from the `moon_ui::components` escape hatch. The approved
  removals are recorded in `docs/component-api-removals.json` so future
  accidental public API removal still fails `component-api`.
- Removed raw `menu` from the public escape hatch as well. Menu/AppMenuBar now
  stays a pending design decision instead of a public application API.
- Added `docs/component-class-migrations.json`. `component-audit` now requires
  explicit approved class migrations before an existing component may move from
  `Pending` to `Mirror`/`Forged`.
- At this intermediate point, `NewControls` tracked the remaining queue: legacy
  Root dialog API, AppMenuBar/menu, List, Tree, plus engine helper policy.
  Adapted raw Longbridge controls were removed from that tab and from the public
  facade.

## Validation Log

- `cargo fmt --all -- --check` passed.
- `cargo check -p moon-ui-gallery` passed.
- `cargo check -p moon-ui-gallery --features snapshot` passed.
- `cargo test -p xtask` passed.
- `cargo test -p moon-ui-components` passed: 252 tests.
- `cargo test -p moon-ui-gallery` passed: 3 tests.
- `cargo xtask component-audit --check-baseline` passed.
- `cargo xtask component-api --check-baseline` passed: 675 public signatures.
- `powershell -ExecutionPolicy Bypass -File tools\capture-gallery-snapshots.ps1 -Compare` first caught a Windows
  taskbar-preview artifact in `Controls`; after fixing the capture fallback, the same baseline passed.
  A later run also caught an accidental default `MoonSelect` visual change; after making trigger variants opt-in
  and moving raw colors into palette tokens, the same baseline passed:
  Controls 0.0063%, Inputs 0.0063%, Data 0.0063%, Overlays 0.0063%, Layout 0.0092%.
- MoonTerminal integration check passed:
  `cargo check -p moon-ui-gpui --target x86_64-pc-windows-msvc`.

## 2026-06-21 Follow-up Validation

- `cargo fmt --all -- --check` passed.
- `cargo check -p moon-ui-gallery` passed.
- `cargo test -p moon-ui-gallery` passed: 3 tests.
- `cargo xtask component-audit --check-baseline` passed. Current manifest:
  Mirror 13, Forged 19, Domain 2, Internal 6, Forbidden 2, Pending 4.
- `cargo xtask component-api --check-baseline` passed with approved removals
  for raw adapted escape-hatch modules.
- MoonTerminal integration check passed:
  `cargo check -p moon-ui-gpui --target x86_64-pc-windows-msvc`.
- Gallery visual snapshots were refreshed for six pages and then compared
  after the facade cleanup: Controls 0.0023%, Inputs 0.0000%, Data 0.0000%,
  Overlays 0.0000%, Layout 0.0000%, NewControls 0.0000%.
- `rg -n "moon_ui::components" R:\test\MoonTerminal\crates\moon-ui-gpui\src`
  returned no matches.

## 2026-06-21 NewControls Correction

Correction: I briefly treated a thin `Moon*` wrapper as a MoonReady component
and put raw-looking Avatar/Switch/Radio/Kbd/Link/Separator/Skeleton/Spinner
wrappers into `NewControls`. That was wrong. A wrapper alone preserves behavior
but does not prove Moon visual quality.

Decision: `NewControls` is a showcase of ready adapted Moon-facing controls,
not a dump of raw Longbridge surfaces and not a pending backlog. A component may
enter `NewControls` only when all of these are true:

- the public API is `moon_ui::Moon*`, not `moon_ui::components::*`;
- the rendered control uses Moon theme/tokens and visually fits the gallery;
- the gallery example is live and snapshot-covered;
- the manifest class is truthful (`Mirror`/`Forged` only after the visual pass).

The premature wrappers were removed from the public Moon facade and from the
manifest. `NewControls` now shows already adapted surfaces: `MoonAlert`,
`MoonTag`, `MoonProgress`, `MoonDialog`, `MoonNotification`, and
`MoonAccordion`.

Follow-up: `MoonToggle` and `MoonRadio` were added the correct way: ported from
the designer-stand Moon implementation, scaled through `MoonTheme` tokens, kept
free of raw runtime colors, covered by unit metrics tests, added to the
manifest as `Forged`, and shown in `NewControls` only after visual inspection.

Validation after correction:

- `cargo fmt --all -- --check` passed.
- `cargo check -p moon-ui-gallery` passed.
- `cargo test -p moon-ui-gallery` passed.
- `cargo test -p moon-ui-components` passed: 254 tests.
- `cargo xtask component-audit --check-baseline` passed. Current manifest:
  Mirror 13, Forged 21, Domain 2, Internal 6, Forbidden 2, Pending 3.
- `cargo xtask component-api --check-baseline` passed: 758 public signatures.
- `cargo check -p moon-ui-gpui --target x86_64-pc-windows-msvc` passed.
- `powershell -ExecutionPolicy Bypass -File tools\capture-gallery-snapshots.ps1 -Compare`
  passed after refreshing the intentionally changed `NewControls` baseline:
  Controls 0.0010%, Inputs 0.0000%, Data 0.0000%, Overlays 0.0000%,
  Layout 0.0000%, NewControls 0.0000%.

## 2026-06-21 Batch MoonReady Expansion

The next batch deliberately moved several designer-stand Moon primitives at
once instead of validating one control at a time. The admission rule stayed the
same: no raw Longbridge visual wrappers in `NewControls`; only Moon-facing
components with MoonTheme-backed rendering and manifest coverage.

Added Moon-facing public components:

- `MoonCollapsible`
- `MoonFormRow`
- `MoonGroupBox`
- `MoonLabel`
- `MoonPresetStrip` / `MoonPresetItem`
- `MoonSelectorPill` / `MoonSelectorSegment`
- `MoonStepper`
- `MoonSurface` / `MoonSurfaceVariant`

These were ported from the designer-stand Moon implementation and changed to
use `MoonTheme::active_tokens(cx)` instead of `MoonPalette::TERMINAL` runtime
constants. White hover fills and old stand-specific gray literals were replaced
with palette tokens such as `overlay`, `panel_high`, `border_hover`, and
`shadow`.

`NewControls` now contains two real scenario blocks for this batch:

- form/settings composition: surface, label, group box, form row, selector pill,
  and stepper;
- toolbar/header composition: collapsible section and preset strip.

The batch also closed an application facade hole found by the source audit:
MoonTerminal no longer imports `moon_ui::components::*` or `gpui_component::*`
from `crates/moon-ui-gpui/src`. Terminal usages of raw `Notification`,
`WindowExt`, `Progress`, `Tag`, and `Accordion` were moved to
`MoonNotification`, `MoonWindowExt`, `MoonProgress`, `MoonTag`, and
`MoonAccordion`. The base `Theme` type is now re-exported from top-level
`moon_ui::Theme` for the remaining caret bridge.

Validation after the batch:

- `cargo fmt --all` passed in MoonUI and MoonTerminal.
- `cargo check -p moon-ui-gallery` passed.
- `cargo test -p moon-ui-components` passed: 256 tests.
- `cargo test -p moon-ui-gallery` passed: 3 tests.
- `cargo xtask component-audit --check-baseline` passed. Current manifest:
  Mirror 13, Forged 34, Domain 2, Internal 6, Forbidden 2, Pending 3.
- `cargo xtask component-api --check-baseline` passed: 914 public signatures.
- `powershell -ExecutionPolicy Bypass -File tools\capture-gallery-snapshots.ps1 -Compare`
  passed after refreshing the intentionally changed local gallery baseline:
  Controls 0.0014%, Inputs 0.0000%, Data 0.0000%, Overlays 0.0000%,
  Layout 0.0000%, NewControls 0.0000%.
- MoonTerminal integration check passed:
  `cargo check -p moon-ui-gpui --target x86_64-pc-windows-msvc`.
- `rg -n "moon_ui::components|moon-ui-components|gpui_component::" R:\test\MoonTerminal\crates\moon-ui-gpui\src`
  returned no matches.

## 2026-06-21 Second Batch MoonReady Expansion

The next useful/simple batch admitted components that either have small enough
behavior to own directly or can safely preserve Longbridge behavior behind a
Moon-facing wrapper:

- `MoonAvatar`, `MoonAvatarGroup`, `MoonAvatarSize` — forged Moon renderer
  because the raw Longbridge avatar skin did not match the terminal palette.
- `MoonBreadcrumb`, `MoonBreadcrumbItem` — forged compact Moon breadcrumb.
- `MoonDescriptionList` — mirror wrapper over Longbridge DescriptionList.
- `MoonPagination` — mirror wrapper over Longbridge Pagination.
- `MoonProgressCircle`, `MoonProgressCircleSize` — mirror wrapper over
  Longbridge ProgressCircle.

The `NewControls` page now has an additional live
`Identity / navigation` scenario and a `Description data` scenario. The page
shows avatar grouping, circular progress, clickable breadcrumb, pagination
state changes, and description list layout in context.

Intentionally deferred after inventory:

- `list`, `tree`, `combobox`, `date_picker`, `number_input`, `sidebar`,
  `sheet`, `hover_card`, and native/menu surfaces need focused behavior passes.
  They are useful, but they should not be admitted by a thin visual wrapper
  because their state, focus, keyboard, popup, or lifecycle behavior is the
  important part.

Validation after the second batch:

- `cargo fmt --all -- --check` passed.
- `cargo check -p moon-ui-gallery` passed.
- `cargo test -p moon-ui-components` passed: 257 tests.
- `cargo test -p moon-ui-gallery` passed: 3 tests.
- `cargo xtask component-audit --check-baseline` passed. Current manifest:
  Mirror 16, Forged 37, Domain 2, Internal 6, Forbidden 2, Pending 3.
- `cargo xtask component-api --check-baseline` passed: 974 public signatures.
- `powershell -ExecutionPolicy Bypass -File tools\capture-gallery-snapshots.ps1 -Compare`
  passed after refreshing the intentionally changed local gallery baseline:
  Controls 0.0018%, Inputs 0.0000%, Data 0.0000%, Overlays 0.0000%,
  Layout 0.0000%, NewControls 0.0000%.

## 2026-06-21 Facade Completion For Terminal Handoff

The facade pass finished the useful Longbridge-backed controls that were
blocking terminal-side cleanup. These controls are admitted as `Mirror`: the
Longbridge behavior/state engines stay intact, while the public application path
is now `moon_ui::*` and the visual layer goes through the synchronized Moon
theme bridge.

New Moon-facing facade entries:

- `MoonCombobox`, `MoonComboboxState`, `MoonSearchableVec`,
  `MoonSearchableListDelegate`, `MoonSearchableListItem`,
  `MoonSearchableListState`, `MoonSearchableListChange`;
- `MoonCalendar`, `MoonCalendarState`, `MoonDate`, `MoonDateMatcher`;
- `MoonDatePicker`, `MoonDatePickerState`, `MoonDateRangePreset`;
- `MoonHoverCard`, `MoonHoverCardState`;
- `MoonList`, `MoonListState`, `MoonListDelegate`, `MoonListItem`;
- `MoonTree`, `MoonTreeState`, `MoonTreeItem`, `MoonTreeEntry`;
- `MoonSidebar`, `MoonSidebarGroup`, `MoonSidebarMenu`,
  `MoonSidebarMenuItem`, `MoonSidebarToggleButton`;
- `MoonSheet`, `MoonSheetSettings`, `MoonPlacement`;
- `MoonScrollableElement` as the narrow facade for scroll extension methods.

`MoonWindowExt` now owns the sheet path too:

- `open_moon_sheet`;
- `open_moon_sheet_at`;
- `close_moon_sheet`.

This prevents terminal code from importing raw `moon_ui::components::WindowExt`
just to open a root-owned sheet. Existing dialog and notification methods
remain unchanged.

`list` and `tree` were moved from `Pending` to `Mirror` with explicit approved
class migrations in `docs/component-class-migrations.json`. The reason is not
"they compile"; the reason is that Longbridge already provides the real value:
focus, keyboard navigation, virtual rendering, selection, expand/collapse
events and context-menu lifecycle.

`NewControls` now includes live stateful scenarios for:

- `MoonCombobox`;
- `MoonDatePicker`;
- `MoonCalendar`;
- `MoonHoverCard`;
- `MoonList`;
- `MoonTree`;
- `MoonSidebar`;
- `MoonSheet`.

Known guardrail gap: the current gallery snapshot harness captures one viewport
per page. The new heavy controls are lower on the scrollable `NewControls`
page, so the local visual comparison proves that page rendering is stable, but
does not yet crop every lower component state. The next gallery harness pass
should either capture scroll positions or split advanced controls into another
page before treating snapshots as full visual coverage for the lower blocks.

Terminal handoff rule after this pass:

- import from `moon_ui::*`;
- do not import `moon_ui::components::*`, `moon-ui-components`, or
  `gpui_component::*`;
- use `MoonWindowExt as _` for dialogs, sheets, context menus and notifications;
- use `MoonScrollableElement` if a screen needs scroll extension methods;
- use `MoonComponentIndexPath` for Longbridge-backed list/combobox state APIs
  until the index-path seam is unified.

Validation:

- `cargo check -p moon-ui-gallery` passed.
- `cargo test -p moon-ui-components` passed: 257 tests.
- `cargo test -p moon-ui-gallery` passed: 3 tests.
- `cargo xtask component-audit --check-baseline` passed. Current manifest:
  Mirror 24, Forged 37, Domain 2, Internal 6, Forbidden 2, Pending 1.
- `cargo xtask component-api --check-baseline` passed: 994 public signatures.
- `powershell -ExecutionPolicy Bypass -File tools\capture-gallery-snapshots.ps1 -Compare`
  passed after refreshing the intentionally changed local gallery baseline:
  Controls 0.0014%, Inputs 0.0000%, Data 0.0000%, Overlays 0.0000%,
  Layout 0.0000%, NewControls 0.0000%.

## 2026-06-21 Useful Donor Completion Batch

The remaining useful donor controls that were still outside the public
Moon-facing API were admitted or resolved:

- `MoonSwitch` exposes the Longbridge switch behavior through `moon_ui::*`.
- `MoonResizablePanelGroup`, `MoonResizablePanel`, `MoonResizableState`, and
  `moon_h_resizable` / `moon_v_resizable` / `moon_resizable_panel` expose the
  real Longbridge resizable panel engine without forcing applications to import
  `moon_ui::components::*`.
- `MoonSettings`, `MoonSettingPage`, `MoonSettingGroup`, `MoonSettingItem`,
  `MoonSettingField`, and field option aliases expose the reusable Longbridge
  settings layout through the public facade.
- `MoonNativeMenu` replaces the previous raw `menu` pending escape hatch. The
  in-window menu APIs remain `MoonContextMenu` / `MoonPopupMenu`; native menu is
  now explicit and OS-rendered by design.
- `MoonRating` was first tried as a thin Longbridge mirror, but visual review
  showed the raw icon rendering was not readable enough in the Moon terminal
  palette. It was therefore admitted as a forged Moon renderer instead of
  pretending a wrapper was sufficient.

`component-manifest.json` now has no `Pending` entries. Current classification:

- Mirror: 28;
- Forged: 38;
- Domain: 2;
- Internal: 6;
- Forbidden: 2.

The `NewControls` page now shows the new ready controls:

- `MoonSwitch` and forged `MoonRating` in the choice-control card;
- `MoonNativeMenu` through an explicit OS-native menu trigger;
- `MoonSettings` with real typed fields;
- `MoonResizablePanelGroup` with real draggable panels.

Snapshot stability was tightened by moving the OS cursor away from the gallery
window before each screenshot run in `tools/capture-gallery-snapshots.ps1`.
This fixed random hover-state diffs in the Layout page. A taller snapshot
window was tested and rejected because it created black/offscreen capture
artifacts on the current Windows desktop; keep the normal 1280x900 snapshot
size until scroll-position captures or split advanced pages are implemented.

Validation after this batch:

- `cargo fmt --all -- --check` passed.
- `cargo check -p moon-ui-gallery` passed.
- `cargo test -p moon-ui-components` passed: 257 tests.
- `cargo test -p moon-ui-gallery` passed: 3 tests.
- `cargo xtask component-audit --check-baseline` passed with 0 Pending.
- `cargo xtask component-api --check-baseline` passed: 1026 public signatures.
- `powershell -ExecutionPolicy Bypass -File tools\capture-gallery-snapshots.ps1 -Compare`
  passed after refreshing the intentionally changed local gallery baseline:
  Controls 0.0010%, Inputs 0.0000%, Data 0.0000%, Overlays 0.0000%,
  Layout 0.0000%, NewControls 0.0000%.

## 2026-06-21 Facade / Theme Bridge Tightening

Two remaining architectural gaps from the final component plan were closed:

- `MoonThemeTokens` now maps into the fork base `ThemeColor` through one complete
  struct literal. `MoonTheme::install_config` assigns `base.colors` from that
  typed bridge instead of mutating `BaseTheme.colors` field-by-field. If the
  underlying Longbridge theme color struct gains a field, Rust now fails the
  build instead of silently leaving a default/foreign color in the UI.
- The terminal no longer imports top-level `moon_ui::Theme` to patch the input
  caret color. The caret now comes from the Moon theme bridge
  (`caret = MoonPalette.text`). The raw top-level `pub use gpui_component::Theme`
  was removed from `moon-ui`; the approved API removal is recorded in
  `docs/component-api-removals.json`.

The visual coverage gap in `NewControls` was also corrected. The tall page hid
several ready controls below the first 1280x900 snapshot viewport, so the
gallery now has separate real scenario pages:

- `NewControls` for ready individual controls and root overlay triggers;
- `Composites` for settings, resizable panels, combobox/date/calendar/list;
- `Stateful` for tree/sidebar state and root-owned sheet behavior.

Validation after this tightening:

- `cargo fmt --all -- --check` passed.
- `cargo check -p moon-ui-gallery` passed.
- `cargo test -p moon-ui-gallery` passed: 3 tests.
- `cargo xtask component-audit --check-baseline` passed with 0 Pending.
- `cargo xtask component-api --check-baseline` passed: 1025 public signatures
  after the approved raw `Theme` removal.
- `powershell -ExecutionPolicy Bypass -File tools\capture-gallery-snapshots.ps1 -Compare`
  passed after refreshing the intentionally changed local gallery baseline:
  Controls 0.0023%, Inputs 0.0000%, Data 0.0000%, Overlays 0.0000%,
  Layout 0.0000%, NewControls 0.0000%, Composites 0.0000%,
  Stateful 0.0000%.
- `cargo check -p moon-ui-gpui --target x86_64-pc-windows-msvc` passed in the
  terminal checkout with local MoonUI.

## 2026-06-21 Facade Escape Hatch Removal / Mirror Diff Health

The previous tightening still left one architectural ambiguity in the docs: an
application could read the old notes and assume that `moon_ui::components::*`
remained a controlled public escape hatch. That is no longer the target state.

Current decision:

- application code imports Moon-facing APIs from `moon_ui::*`;
- the top-level `moon_ui` facade re-exports only `gpui_component::moon::*` and
  `gpui_component::moon::foundation::*`;
- raw Longbridge modules stay inside the implementation crate and are not a
  public application API;
- if a raw Longbridge component is useful, it must become a `Moon*` API first or
  stay internal until that work is done.

The raw facade was removed:

- removed top-level `pub use gpui_component::Theme`;
- removed top-level `pub mod components`;
- recorded the approved API removals in `docs/component-api-removals.json`.

The audit now has explicit source metrics for this rule:

- `facade_components_escape_hatch = 0`;
- `facade_raw_gpui_exports = 0`.

The theme bridge was also made structurally harder to regress:

- `MoonThemeTokens::theme_colors()` now returns a complete `ThemeColor` struct
  literal;
- if Longbridge adds a new `ThemeColor` field, the Moon bridge fails at compile
  time instead of silently inheriting a non-Moon default.

A new mirror guard was added:

- `cargo xtask component-mirror --check-baseline` verifies every manifest
  `Mirror` component against a recorded source hash;
- `cargo xtask component-mirror --donor-root <Longbridge ui src> --check-baseline`
  also compares those mirrors with a real Longbridge donor checkout;
- `docs/component-mirror-baseline.json` records the intentional donor drift.

This gives the palette two separate proofs:

- `component-audit` proves Moon architecture and critical semantic contracts;
- `component-mirror` proves that mirror components have not drifted from their
  declared donor source by accident.

Validation after this tightening:

- `cargo fmt --all -- --check` passed.
- `cargo check -p moon-ui-gallery` passed.
- `cargo test -p moon-ui-components` passed: 257 tests.
- `cargo test -p moon-ui-gallery` passed: 3 tests.
- `cargo xtask component-audit --check-baseline` passed with 0 Pending and all
  source debt counters at 0.
- `cargo xtask component-api --check-baseline` passed: 1019 public signatures
  after approved facade removals.
- `cargo xtask component-mirror --check-baseline` passed: 28 mirror components.
- `cargo xtask component-mirror --donor-root R:\test\chart-test\stand-gpui\vendor\gpui-component\crates\ui\src --check-baseline`
  passed against the Longbridge donor tree.
- `powershell -ExecutionPolicy Bypass -File tools\capture-gallery-snapshots.ps1 -Compare`
  passed:
  Controls 0.0010%, Inputs 0.0000%, Data 0.0000%, Overlays 0.0000%,
  Layout 0.0000%, NewControls 0.0000%, Composites 0.0000%,
  Stateful 0.0000%.

## 2026-06-21 Audit Closure Pass

The external audit after the previous "done" call was right about several live
gaps. This pass closed them as architecture, not as local patches.

Closed dock issue:

- `move_panel_to_tabs` now follows the same invariant as split-drop: capture a
  target anchor before `take`, cancel self-drop before mutating layout, re-find
  the target path after `take`, and use a non-destructive fallback.
- Added dock tests for target re-resolution and self-drop preservation.
- `component-audit` now requires those tests and source markers under
  `dock.behavior_contracts`.

Closed guardrail wiring issue:

- Added `tools/run-component-guardrails.ps1` as the one-command local gate.
- Added `.github/workflows/moonui-guardrails.yml` so non-visual component
  guardrails run on push and pull request.
- The local gate can also run donor mirror checks and visual snapshots.

Closed base raw-color issue:

- Replaced direct base-layer hover/track colors in input, slider and checkbox
  with Moon palette tokens.
- Added `raw_hex_in_moon_base_runtime = 0` to `component-audit` for the themed
  base files that Moon wrappers rely on.

Closed light-theme verification issue:

- The gallery now has a visible Dark/Light theme toggle.
- Snapshot CLI accepts `--theme`.
- `tools/capture-gallery-snapshots.ps1` captures and compares both Dark and
  Light themes, producing `Dark-*` and `Light-*` snapshots.
- Snapshot capture now moves the cursor before each theme run and waits more
  frames between pages so list hover/animation state does not produce false
  diffs.

Closed legacy dock ambiguity:

- The retained Longbridge `src/dock/*` implementation is now represented in the
  manifest as `longbridge_dock_legacy`, class `Internal`.
- `legacy_dock.internal_only` verifies that it stays internal and is not
  exported through the public `moon_ui` facade.

Closed MoonRoot alias ambiguity:

- The real root type is now `MoonRoot`.
- `Root` remains only as a backward-compatible alias.
- `root.moon_owned_type` verifies this direction cannot silently regress.

Current audit counters after this pass:

- `Mirror: 28`;
- `Forged: 38`;
- `Domain: 2`;
- `Internal: 7`;
- `Forbidden: 2`;
- all source debt counters are `0`, including `raw_hex_in_moon_base_runtime`.

## 2026-06-21 Audit Verifier Pass

The next audit found a deeper problem: several guardrails were self-confirming.
They checked local source markers or local hashes without a second side.

Closed contract verifier issue:

- `component-audit` now records a verifier class for every contract:
  `StructuralSource`, `BehavioralTest`, or `VisualGolden`.
- Behavioral contracts now require concrete Rust test names to exist and not be
  `#[ignore]`; `cargo test` remains the execution side.
- Visual contracts now require committed gallery golden PNGs for every page in
  both Dark and Light themes.
- `manifest.contracts_have_verifiers` fails if the component manifest names a
  contract that `component-audit` does not know how to verify.
- Added Rust tests for checkbox click toggling and input tone propagation where
  the previous audit had only source-shape checks.

Closed visual baseline issue:

- Gallery golden snapshots moved from local `target/.../baseline` to committed
  `crates/moon-ui-gallery/snapshots/baseline`.
- `tools/capture-gallery-snapshots.ps1` still writes current output under
  `target`, but compares against the committed baseline.

Closed mirror donor issue:

- Added `vendor/longbridge-gpui-component-ui-src` as a pinned Longbridge donor
  source snapshot.
- Manifest `upstream_ref` values now include the donor SHA.
- `component-mirror --check-baseline` fails if the baseline was recorded with a
  donor and the current run has no donor.
- `tools/run-component-guardrails.ps1` and GitHub Actions use the vendored donor
  by default.

## 2026-06-22 Mirror Truth Contract

The follow-up audit found one more blind spot: `component-mirror` knew which
`Mirror` components had donor drift, but `component-audit` did not consume that
fact. A component could therefore stay classified as a clean mirror with
`fork_reason: null` even when the donor baseline showed modified base files.

Closed mirror truth issue:

- Added `mirror.donor_drift_requires_reason` to `component-audit`.
- The audit now reads `docs/component-mirror-baseline.json` and fails if that
  baseline was not recorded with a donor.
- Any current manifest `Mirror` whose mirror baseline reports
  `donor_changed_files > 0` must either have a non-empty manifest
  `fork_reason` or be reclassified as `Forged`.
- Added reviewed `fork_reason` entries for the existing drifted mirrors:
  button, checkbox, dialog, hover_card, input, list, menu, popover, progress,
  progress_circle, resizable, select, settings, slider, text_area, and tree.

This preserves the intended distinction:

- `Mirror` can still mean Longbridge behavior/state is source-owned;
- reviewed Moon hooks/tests/style hardening inside base files are allowed, but
  must be explicit;
- unreviewed donor drift is now a critical audit failure, not a green report.
