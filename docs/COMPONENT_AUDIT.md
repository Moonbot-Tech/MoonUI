# MoonUI Component Audit

`component-audit` is the first mechanical guardrail for the MoonUI component
refactor.

It is not a replacement for visual PNG snapshots. It is the baseline layer that
records the current component architecture and catches regressions before a
human opens the gallery.

## Commands

Create or refresh the current baseline:

```powershell
cargo xtask component-audit --update-baseline
```

Check current code against the baseline:

```powershell
cargo xtask component-audit --check-baseline
```

Print the current full report:

```powershell
cargo xtask component-audit --json
```

The baseline lives at:

```text
docs/component-audit-baseline.json
```

## What It Guards

The audit currently records:

- component manifest classification: `Mirror`, `Forged`, `Domain`, `Pending`,
  `Forbidden`;
- source debt counters such as `MoonSkinPalette`, `moon_color`, public facade
  slurp, raw runtime hex and no-op API markers;
- critical semantic contracts that must not regress;
- guardrail contracts that prove the checking machinery is wired;
- the verifier class for each contract: `StructuralSource`, `BehavioralTest`,
  or `VisualGolden`.

Example critical visual contract:

```text
checkbox.checked_glyph.asset
```

This is backed by committed gallery golden PNGs. If checked checkbox rendering
is replaced by a text glyph such as `x`, the snapshot diff must fail.

Example behavioral contract:

```text
checkbox.click_toggles
```

This does not grep implementation text. It requires a concrete Rust test name
to exist and not be `#[ignore]`; `cargo test` is the execution side of the
contract.

Example structural contract:

```text
legacy_dock.internal_only
```

This is intentionally source/manifest based because the invariant is an API
boundary: the retained Longbridge dock source must stay internal and must not be
exported through the public facade.

## Baseline Rule

The baseline is intentionally allowed to contain current debt.

Refactors are allowed to reduce debt. They are not allowed to increase it.

For example:

- `moon_skin_palette_usages: 29 -> 0` is good;
- `moon_skin_palette_usages: 29 -> 30` is a regression and must fail;
- `checkbox.checked_glyph.asset: Pass -> Fail` is always a hard failure.

## Relationship To Visual Snapshots

This audit catches architecture and semantic regressions. It does not prove
pixel-perfect visual identity.

Visual snapshots are handled by the gallery runner:

```powershell
powershell -ExecutionPolicy Bypass -File tools\capture-gallery-snapshots.ps1 -Compare
```

The gallery switches pages internally, waits for frames to settle, clears stale
PNG files, writes current screenshots, and the wrapper compares them against
the committed golden baseline in:

```text
crates/moon-ui-gallery/snapshots/baseline
```

`component-audit` verifies that all golden files exist and that the gallery
snapshot flow is still wired. The actual pixel comparison is the PowerShell
snapshot command above.

On Windows the current snapshot path falls back to Win32 GDI client capture
because the GPUI Windows backend still reports `render_to_image` as
unimplemented. This is good enough to catch visual regressions in normal local
developer runs, but backend `render_to_image` remains the cleaner final target.

## Public API Snapshot

The second guardrail records the Moon-facing public API surface.

Create or refresh the API baseline:

```powershell
cargo xtask component-api --update-baseline
```

Check current API against the baseline:

```powershell
cargo xtask component-api --check-baseline
```

The baseline lives at:

```text
docs/component-api-baseline.json
```

This guard catches accidental removals or signature changes in public component
builders, events, public structs, enums, types, constants and facade exports.
Adding new API is allowed. Removing or changing existing API must be an approved
migration, not an accidental side effect of refactoring.

## Mirror Source Guard

The third guardrail tracks components that are intentionally classified as
`Mirror` in `crates/moon-ui-components/component-manifest.json`.

Run it against the pinned Longbridge donor tree:

```powershell
cargo xtask component-mirror --donor-root vendor\longbridge-gpui-component-ui-src --check-baseline
```

Refresh the mirror baseline only after reviewing the diff:

```powershell
cargo xtask component-mirror --donor-root vendor\longbridge-gpui-component-ui-src --update-baseline
```

The baseline lives at:

```text
docs/component-mirror-baseline.json
```

The donor source is vendored at:

```text
vendor/longbridge-gpui-component-ui-src
```

The manifest pins each upstream reference as `Longbridge::<component>@<sha>`.
The current donor SHA is recorded in the vendored `UPSTREAM.md`. A donor-based
baseline must not be checked without `--donor-root`; that is a false green.

This guard does not prove Moon visual quality. It proves that every `Mirror`
component is still tied to a known Longbridge source path and that any drift
from that source is explicit. If a component needs Moon-specific rendering,
styling, or behavior, reclassify it as `Forged` or record the intentional donor
diff in the reviewed baseline update.

## Required Local Gate

Run the full local non-visual gate with:

```powershell
powershell -ExecutionPolicy Bypass -File tools\run-component-guardrails.ps1
```

Run the local gate with visual checks:

```powershell
powershell -ExecutionPolicy Bypass -File tools\run-component-guardrails.ps1 `
  -WithSnapshots
```

The local gate uses the vendored Longbridge donor by default. Passing
`-DonorRoot` is only for intentional donor refresh work.

The snapshot gate captures both Dark and Light themes. A component that only
works in the terminal dark theme is not considered verified.

The repository also contains `.github/workflows/moonui-guardrails.yml`, which
runs the non-visual guardrails on push and pull request, including the vendored
donor mirror check.

## Refactor Workflow

1. Before a refactor, run:

   ```powershell
   powershell -ExecutionPolicy Bypass -File tools\run-component-guardrails.ps1
   ```

2. Make the refactor.

3. Run the audit again.

4. If the audit fails:

   - fix the regression; or
   - if the change is intentional, update the manifest/contract first and then
     refresh the baseline in the same reviewed change.

5. Never refresh the baseline just to hide a regression.
