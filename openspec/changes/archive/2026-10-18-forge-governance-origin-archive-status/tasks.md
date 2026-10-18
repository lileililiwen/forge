# Tasks: record the archived governance origin and Forge maintenance owner

## 1. BFS — Baseline and impact coverage

- [x] 1.1 Identify current Forge records that state the origin is unarchived
  or make archival depend on conformance/consumer migration.
- [x] 1.2 Confirm the user-provided state: original source repository archived;
  Forge's in-tree module is the maintenance target.
- [x] 1.3 Bound changes to Forge docs/specs; do not edit the archived origin or
  the imported module implementation.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Correct HANDOFF, roadmap and import guide ownership/status language.
- [x] 2.2 Update the canonical in-tree Governance maintenance requirement.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Confirm source-size conformance remains tracked but is independent of
  origin archival.
- [x] 3.2 Confirm sibling portability remains a separate Forge task and no
  current record directs changes to the archived origin.
- [x] 3.3 Review the full diff and confirm no module/source-origin files changed.

## 4. Verification

- [x] 4.1 Run name preflight, strict OpenSpec validation, wording audit and
  `git diff --check`.
- [x] 4.2 Run Forge Gate dry-run and bounded full Gate; record the verdict.
  Dry-run resolved eight checks. Two full runs were BLOCKED only at
  `declared-verification` (exit 101); `tests` and all other checks passed.
  The exact declared command passed when run alone. Gate runs the `tests` and
  `declared-verification` Cargo suites concurrently; the truncated Gate
  diagnostic omits the failing test name. This docs-only change cannot affect
  either suite. Remediation is to retain full check output and resolve the
  concurrent Cargo test failure before treating a future Gate as green.
- [x] 4.3 Archive without `--skip-specs`, promote the canonical spec, commit
  the related docs/archive, then make the HANDOFF-only commit and stop.
