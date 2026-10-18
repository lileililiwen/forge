# Tasks: Restore the governance verification baseline

## 1. BFS — Baseline and impact coverage

- [x] Reproduce both failures with `python3 -m unittest discover -s tests -v`:
  missing Mnemora registry row and three unaccounted tracked root JSON files.
- [x] Read Mnemora's declaration and confirm `id/path=mnemora`,
  `profile=rust-product`, `role=product`, `stack=rust`, and `lifecycle=active`.
- [x] Confirm the three omitted JSON files are tracked and are governance-root
  records; no sibling source write is needed.
- [x] Map both failures to this repository's registry and fixture owners; no
  other project or runtime is changed.

## 2. DFS — Requirement-by-requirement implementation

- [x] Add Mnemora to `projects.json` using the declaration's exact identity and
  axes; add a regression assertion while preserving the original migration
  baseline as historical evidence.
- [x] Add the three tracked data records to `GOVERNANCE_ROOT_DATA_FILES` so the
  sandbox copies them into its temporary governance root.

## 3. BFS — Cross-surface regression and completeness

- [x] Verify the live migration check has no outstanding rows and the registry
  reports no `DISCOVERED_UNREGISTERED` or taxonomy-axis finding for Mnemora.
  The portfolio audit still reports 23 `SPEC_PURPOSE_PLACEHOLDER` findings in
  Mnemora; those sibling-owned findings are recorded and outside this change.
- [x] Verify the sandbox coverage test accounts for every top-level governance
  JSON file and promotion tests still use only temporary roots.
- [x] Confirm Mnemora's Git tree is clean and no other sibling repository was
  changed; preserve unrelated changes in this repository.

## 4. Verification

- [x] Focused identity/taxonomy, promotion, and new baseline test modules pass:
  94 tests, 0 failures.
- [x] Full unittest suite passes: 937 tests, 0 failures, 2 skipped.
- [x] Workspace self-check (0 errors, 0 warnings), unattended auto-gate
  (PASS), axis migration check (no outstanding rows), strict validation
  (31/31), change-name check, and `git diff --check` pass.
- [x] Archive without `--skip-specs`; commit implementation/tests/archive as
  commit 1, then `HANDOFF.md` only as commit 2.
