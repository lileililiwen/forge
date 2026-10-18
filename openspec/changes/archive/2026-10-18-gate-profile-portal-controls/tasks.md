# Tasks: Gate profile workbench controls

## BFS baseline

- [x] Trace the current API, evidence envelope, workbench rendering and browser test flow; capture existing compatibility expectations.
- [x] Confirm the bounded package maps one selector/request/evidence/render lifecycle and leaves siblings unchanged.

## DFS requirement implementation

- [x] Add strict typed profile request parsing with an omitted-field `full` default and no side effects on invalid requests.
- [x] Assign the selected profile to `GateConfig` and retain existing evidence/journal behavior.
- [x] Add a labelled profile selector, clear help, profile facts for local and hosted evidence, and honest missing-profile text.
- [x] Preserve loading, success, error, accessibility and responsive behavior in the existing Quality card.
- [x] Ensure the Quality card and profile control remain within the viewport at narrow widths, with lifecycle facts wrapping.

## BFS regression and completeness

- [x] Cover API default, explicit fast/full, invalid JSON/profile, unchanged prior evidence and authorization.
- [x] Cover browser request payload, displayed local/hosted profile, pending/error/success and the documented legacy full-profile default.
- [x] Verify keyboard/focus/label behavior and 320 CSS px reflow in the browser.
- [x] Review all Gate status/rerun callers and confirm CLI, hosted acceptance, persistence and siblings remain compatible.

## Verification

- [x] Run targeted Rust API/Gate tests and browser checks; record exact commands/results.
- [x] Run formatting, `node scripts/check-openspec-change-names.mjs`, `openspec validate --all --strict --no-interactive`, and `git diff --check`.
- [x] Run `forge gate --dry-run`, then full `forge gate` with bounded timeout; record verdict and attribution in HANDOFF.
- [x] Archive without `--skip-specs` and review the promoted canonical spec and related diff.
