# Tasks: enforce-openspec-seriality

## 1. BFS — Baseline and impact coverage

- [x] 1.1 Confirm the repository currently has no active change and
  `current_spec: none`; record the existing `unarchived_changes()` and
  `validate_change_names()` paths and the Gate command that invokes the local
  project audit.
- [x] 1.2 Bound the policy to project id `workspace-governance`; sibling
  repositories and the `auto_gate.py`/Driftwatchdog contracts remain
  unchanged.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Add focused tests first for zero, one, and multiple active changes,
  current pointer match/mismatch/duplication/staleness, and non-governance
  project scope; run them to demonstrate the new cases fail before the fix.
  The first red run had 5 failing policy cases (multiple changes, missing,
  duplicate, mismatched, and stale pointers); the expanded nine-case suite
  passes after implementation.
- [x] 2.2 Implement `OPENSPEC_CHANGE_SERIALITY` using the existing active
  directory enumerator and an anchored `current_spec:` parser; fail closed
  when the existing handoff file cannot be read.
- [x] 2.3 Wire the check into the `workspace-governance` project audit and
  add the current-repository assertion to the already-required unit test Gate.
  `validate_project()` reports the ERROR, while the unit test blocks this Gate
  without changing the sibling Driftwatchdog finding-code registry.
- [x] 2.4 Document the one-active-change rule, pointer behavior, and recovery
  steps in README and the workspace-check delta spec.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Prove archive directories and dot-prefixed OpenSpec state are not
  counted, and that multiple changes produce one diagnostic listing sorted
  names. The focused tests cover both boundaries and deterministic ordering.
- [x] 3.2 Prove sibling project audits are unaffected and existing finding
  JSON, summary, and Forge adapter observation shapes are unchanged. The
  non-governance fixture emits no seriality finding, and the full suite passes
  the existing report and Forge adapter regressions.
- [x] 3.3 Review Gate output for an active, correctly pointed change and for a
  fixture with two active changes; confirm the repository-state test fails
  closed for the second case and the audit reports its ERROR. The fixture
  audit emits one `OPENSPEC_CHANGE_SERIALITY` ERROR; the live repository-state
  assertion is part of the required tests Gate. The real Driftwatchdog Gate
  passes with the current single active change and its matching pointer.

## 4. Verification

- [x] 4.1 Run focused tests, the full governance unit suite, workspace
  self-check, and auto-gate; record exact results.
  Focused tests: 10 passed; full suite: 948 passed, 2 skipped;
  self-check: 0 errors, 0 warnings; auto-gate: PASS; Driftwatchdog Gate: PASS
  after the sandboxed attempt's pip-audit network failure was rerun with
  network access.
- [x] 4.2 Run strict OpenSpec validation, change-name validation, and
  `git diff --check`; archive and confirm the active list is empty.
  Strict all-spec validation: 27 passed; change names valid; diff check clean;
  archive is `2026-10-12-enforce-openspec-seriality`; `openspec list` reports
  no active changes.
- [x] 4.3 Preserve the two-commit boundary: code, tests, docs, and archive in
  commit 1, followed by a HANDOFF-only pointer/evidence commit; preserve all
  pre-existing dirty files and untracked user files. Explicit staging is used
  for both commits.
