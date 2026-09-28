# Tasks: forge-publish-queue-status

## 1. BFS — Baseline and impact coverage

- [ ] B1. Reconcile the existing provider progress commits (`fe13db1` and
  `62d2da2`) against this package and capture their behavior in fixtures.
- [ ] B2. Map queue identity, project operation identity, journal rows, CLI
  output, provider events, and failure transitions to the requirements.
- [ ] B3. Add fixture skeletons for two projects, one active provider, one
  failed provider, malformed events, and an interrupted queue.
- [ ] B4. Confirm implementation-handoff boundary: Forge owns state/status;
  Jenkins owns Mac execution; no Mac source/script persistence.

## 2. DFS — Requirement-by-requirement implementation

- [ ] D1. Add explicit queue/run and project state models with one-running
  invariant and deterministic ordering.
- [ ] D2. Extend the provider request/event transport with optional queue ID,
  event validation, bounded detail, and credential redaction.
- [ ] D3. Persist active and terminal project states through the existing
  operations journal without adding a second database.
- [ ] D4. Implement `forge deploy status` fleet/project/queue queries and
  bounded `--watch` behavior.
- [ ] D5. Add Forge and Jenkins provider contract tests for event ordering,
  protocol failure, timeout, and terminal response compatibility.

## 3. BFS — Cross-surface regression and completeness

- [ ] R1. Prove sequential execution across manual publish, fleet publish, and
  GitHub-triggered publish paths.
- [ ] R2. Prove JSON/human status parity and read-only status queries.
- [ ] R3. Verify partial success, fail-fast, disabled provider, stale journal,
  unknown project, and watch timeout behavior.
- [ ] R4. Verify Mac runtime-only invariants, shared Docker cache behavior,
  route refresh, and no permanent source/script on Mac.
- [ ] R5. Review Forge/OpenSpec/provider docs for stale claims that all fleet
  projects are already deployed.

## 4. Verification

- [ ] V1. Run `python3 -m unittest discover -s tests -v` in `jenkins-local`.
- [ ] V2. Run `cargo fmt --all -- --check`, `cargo build`, and targeted plus
  all applicable Forge tests.
- [ ] V3. Run `node scripts/check-openspec-change-names.mjs`.
- [ ] V4. Run `openspec validate --all --strict --no-interactive`.
- [ ] V5. Capture real Mac evidence for one project and one fleet status query;
  do not claim full fleet deployment from dry-run or build-only evidence.
