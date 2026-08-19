# Contributing to MoonUI

Keep changes focused and preserve the separation between the extracted GPUI runtime, the inherited
Longbridge component base, and Moon-owned widgets. Detailed component policy lives in
[`docs/COMPONENT_AUDIT.md`](docs/COMPONENT_AUDIT.md), palette policy in
[`docs/PALETTE_SPEC.md`](docs/PALETTE_SPEC.md), and visual verification in
[`docs/VISUAL_GUARDRAILS.md`](docs/VISUAL_GUARDRAILS.md).

## Tests

Three kinds of Rust test have three homes:

| kind | sees | lives in |
|---|---|---|
| unit | private items | the module's **sibling `tests.rs` file** |
| integration | public API only | `<crate>/tests/*.rs` |
| doc-test | public API | inline in the `///` example |

- **Do not create `#[cfg(test)] mod tests { ... }` inside a file that carries logic.** Keep only
  the declaration beside the implementation and put the bodies in the sibling file:

  ```rust
  // src/moon/popover.rs
  #[cfg(test)]
  mod tests;
  ```

  ```rust
  // src/moon/popover/tests.rs
  use super::{MoonPopoverPlacement, anchor_for};
  use gpui::Anchor;

  /// Catches changing `popover.rs:anchor_for` so a top-end popover opens from the wrong corner.
  #[test]
  fn top_end_uses_the_bottom_right_anchor() {
      assert_eq!(
          anchor_for(MoonPopoverPlacement::TopEnd),
          Anchor::BottomRight
      );
  }
  ```

- A declaration in `src/parser.rs` or `src/parser/mod.rs` resolves to
  `src/parser/tests.rs`.
- **Crate roots carry no unit tests**, whether the root is the conventional `src/lib.rs` /
  `src/main.rs` or a custom `[lib].path` / `[[bin]].path`. Keep roots thin and move testable logic
  into modules; crate-root unit-test modules otherwise have ambiguous or collision-prone homes.
- Rust tests are committed: both `src/**/tests.rs` and `<crate>/tests/*.rs` belong in Git.
- A `BehavioralTest` manifest contract names a bare Rust test function in its crate. The body may
  move to a sibling file, but the function must remain present, enabled, and named exactly as the
  contract records it.
- When a parent imports `gpui::*`, use selective imports in its test module instead of
  `use super::*`. GPUI exports its own `test` attribute, which can shadow Rust's built-in
  `#[test]` and cause recursive macro expansion.

### What makes a test worth keeping

- Name the plausible production edit the test catches and its user-visible consequence in the
  test's doc comment. A test that cannot fail on a named regression does not belong.
- Derive the expected value independently of the code under test. Reading back a field just set by
  the test, comparing a constant with its own literal, or asserting only `> 0` / `is_some()` is not
  a useful oracle.
- Prove a new regression test: run it green, apply the named plausible mutation, confirm the named
  assertion turns red for the expected reason, then restore the tree exactly and run it green
  again. If the behavior cannot be reproduced locally, report it as unverified instead of claiming
  proof.
- Visual and interactive components must be exercised in both Dark and Light themes. Gallery
  screenshots are useful for stable in-flow visuals but do not prove an open deferred overlay;
  use a `gpui::test` geometry probe with a debug selector for popovers, dropdowns, and tooltips.

## Comments and documentation

- **Comments and doc comments are in English:** `//!`, `///`, and `//`.
- Every new or changed module, function, method, and type gets an accurate doc comment, public or
  private. Describe arguments, return values, and relevant errors or panics; explain why
  non-obvious logic exists.
- Never leave documentation describing behavior that the code no longer has. Existing non-English
  comments outside the edited block are not a reason for a repository-wide translation, but
  rewrite them in English when changing the code they describe.

## Components and UI

- Prefer the Moon wrapper in `crates/moon-ui-components/src/moon/`. Before editing an inherited
  base component, check its class and drift budget in
  `crates/moon-ui-components/component-manifest.json`.
- `Mirror` components permit no donor drift. `TrackedFork` changes must fit their reviewed
  `donor_drift_budget`. Never hand-edit `vendor/`; it is the frozen Longbridge donor.
- Use `MoonTheme::active_tokens`, `MoonPalette`, `MoonTone`, and scaled design-reference values.
  Geometry uses `tokens.ui(...)`; text uses the font/line-height helpers. Do not add raw palette
  hex values or double-scale already resolved pixels.
- Widgets normally implement `RenderOnce`. Use `Render` with an `Entity` only for state objects.
  Open overlays through the shared Moon window APIs rather than rendering them as ordinary
  children.
- A new public widget needs all of the following:

  1. Its Moon module and sibling unit-test file.
  2. The alphabetical module declaration and public re-export in `moon/mod.rs`.
  3. Startup wiring in the appropriate shared `init` function, when required.
  4. A `component-manifest.json` entry and meaningful contracts.
  5. An interactive gallery example and `COMPONENT_COVERAGE` entry.

- Intentional API removals, component-class changes, or donor drift are declared before baselines
  are refreshed. Never update a baseline merely to make a regression pass, and always inspect the
  baseline diff before keeping it.
- A change that ADDS public API refreshes `docs/component-api-baseline.json` in the same pull
  request; the snapshot check fails on an addition it does not know. That keeps the file a
  description of the surface rather than a record of the part somebody once wrote down, and keeps
  the next signature change from arriving with a backlog of other people's additions attached.

## Commits and pull requests

- Commit format: `<type>(<scope>): <subject>` with `feat`, `fix`, `refactor`, `docs`, `test`, or
  `chore`. Example: `fix(popover): preserve fitted content width`.
- Changes to code, APIs, assets, manifests, baselines, tests, CI, or build configuration go through
  a pull request. Maintainers may update the root `README.md` and ordinary prose under `docs/`
  directly; baseline and escape-hatch JSON files are not prose. A mixed change follows the pull
  request route as a whole.
- Keep `master` linear and start from a fresh `origin/master`. Never force-push or reset shared
  history; fix forward or revert.

## Build and checks

The non-visual local gate matches CI:

```powershell
powershell -ExecutionPolicy Bypass -File tools\run-component-guardrails.ps1
```

It runs formatting, the gallery check, component and gallery tests, and the component audit, API,
and donor-mirror checks. Run the script rather than isolated `xtask` commands so the mirror check
receives the correct donor path.

- For visual work, add the manual snapshot comparison:

  ```powershell
  powershell -ExecutionPolicy Bypass -File tools\run-component-guardrails.ps1 -WithSnapshots
  ```

  Snapshot comparison is opt-in and does **not** run in CI. Update a visual baseline only for an
  intentional, reviewed visual change.
- Touching the GPUI runtime also requires `cargo test -p moon-gpui`; that suite is not part of the
  component guardrail workflow.
- `cargo check` does not compile `#[cfg(test)]` modules. Use `cargo check --all-targets` when the
  relevant test code needs a compile check.
- A library change is not verified only because its own crate is green. Exercise reusable widget
  behavior in the gallery and validate consumer-facing changes in MoonTerminal when they affect
  the terminal integration.

## Files

Use LF, UTF-8, four-space indentation, and a trailing newline. A CRLF rewrite appears as a
whole-file diff.
