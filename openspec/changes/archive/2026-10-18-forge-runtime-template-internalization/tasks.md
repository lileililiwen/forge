## 1. BFS — Baseline and impact coverage

- [x] Record current runtime renderer source, registry IDs, file inventory, declaration fields, and sibling Git state.
- [x] Add compatibility tests for the version-1 registry and safe failure boundaries.
- [x] Add Forge CLI/resource-resolution and release-layout tests before implementation.

## 2. DFS — Requirement-by-requirement implementation

- [x] Copy renderer and assets with provenance into Forge; preserve the version-1 contract.
- [x] Implement `forge runtime-template` CLI dispatch and bounded resource lookup.
- [x] Extend archive and installer scripts to bundle resources with the binary.

## 3. BFS — Cross-surface regression and completeness

- [x] Verify list, inspect, render and check through the Forge command and installed layout.
- [x] Verify invalid override, missing bundle, missing Python, renderer failure, and unchanged sibling checkout behavior.
- [x] Review generated declarations, file ownership, docs, CLI help and all callers.

## 4. Verification

- [x] Run focused Rust and Python contract checks, `python3 -m py_compile`, packaging shell syntax, and release package/install/smoke rehearsal.
- [x] Run build, applicable workspace tests, OpenSpec name preflight, strict validation and `git diff --check`.
- [x] Run `forge gate --dry-run` and full Forge Gate with bounded timeout; record exact verdicts.
- [x] Archive without `--skip-specs`, promote canonical specs, stage only related paths, and commit implementation/archive.
- [x] Record the verification evidence and no-active-change pointer state in HANDOFF for its required second commit; do not push.
