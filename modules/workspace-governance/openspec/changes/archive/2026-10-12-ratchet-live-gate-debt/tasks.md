# Tasks: Block new work on live gate debt increases

## 1. BFS — Baseline and impact coverage

- [x] Reproduce the gap: a clean recorded state, one new unbound manifest check, and one new OpenSpec change currently produce zero findings.
- [x] Trace the checker call path and binder helpers; confirm the snapshot is loaded once and the new-change rule currently returns when the project has no snapshot entry.
- [x] Define behavior for unreadable manifests, unknown profiles, baseline pairs, exceptions, and debt-closing changes.
- [x] Bound implementation to workspace governance Python and record Driftwatchdog/binder ownership without editing sibling repositories.
- [x] Measure and close the governance repository's own newly mandatory governance-quality binding using binder-derived output so its self-check can exercise this rule.
- **Done when** required-check resolution, failure states, and acceptance oracles are explicit.

## 2. DFS — Requirement-by-requirement implementation

- [x] Add failing tests for live unrecorded debt, a bound-check negative control, and fail-closed manifest/profile failures; the pre-change run produced 3 expected failures and the bound control passed.
- [x] Reuse binder manifest/profile helpers to read actual required commands without writing or executing them.
- [x] Compare live unbound concerns with the record and include unrecorded increases in the blocking finding.
- [x] Preserve recorded debt and existing baseline, exception, and debt-closing behavior.
- [x] Run focused regression tests (44 passed) and verify the pre-change reproduction is now blocked in both a direct fixture and checker subprocess.
- **Done when** live increases block and all existing grandfathering controls retain their behavior.

## 3. BFS — Cross-surface regression and completeness

- [x] Confirm no debt-record or manifest writes and no gate-command execution occur in the checker path; integration tests compare the before/after digests.
- [x] Run the complete Python suite (847 passed, 1 skipped) and strict OpenSpec validation (13/13).
- [x] Run workspace self-check (0 errors, 0 warnings), auto-gate (PASS, 0 errors, 0 warnings), change-name check, and `git diff --check`.
- **Done when** tests, requirement scenarios, and the binder oracle agree.

## 4. Verification

- [x] Record exact results in `HANDOFF.md`.
- [x] Commit explicit spec, implementation, tests, and docs paths as commit 1; commit `HANDOFF.md` only as commit 2.
- [x] Keep the change active; do not archive without authorization.
