# Tasks: Forge as an external DriftWatch checker

## 1. BFS — Baseline and impact coverage

- [x] Enumerate every finding source (doctor, policy, governance, readiness)
  and its severity vocabulary; record the mapping table.
- [x] Read the sibling checker protocol spec (config-checker-protocol) and
  copy its tolerance rules into the contract fixture notes.
- [x] Identify all state the read-only rule must protect (manifest bytes,
  registry, `.forge/` trees, journal, Git HEAD) and wire before/after
  assertions in the test skeleton.
- [x] Add fixtures: empty-alerts document, mixed severities, truncation
  overflow, unregistered target.

## 2. DFS — Requirement-by-requirement implementation

- [x] Implement the `forge check [TARGET]` CLI with bounded options and
  machine-pure stdout.
- [x] Implement the envelope projection over existing assessment results,
  including unverified/unavailable → warning conversion.
- [x] Implement source/symbol construction with project-relative path
  confinement.
- [x] Implement truncation with an explicit summary alert and credential
  redaction through `policy::redact_credentials`.
- [x] Enforce the no-mutation rule: reuse read-only assessment calls, no
  journal writes.
- [x] Add the docs registration snippet (`[[checkers]]` block) to
  README/docs.

## 3. BFS — Cross-surface regression and completeness

- [x] Verify doctor/inspect/governance/readiness surfaces are unchanged by a
  checker run (byte equality + journal row count equality).
- [x] Verify the emitted document parses under the sibling protocol rules
  (alerts key presence, unknown-field tolerance, severity vocabulary).
- [x] Verify CLI help, completions and `--format json` interplay; confirm no
  MCP/API/portal leakage of the new command.
- [x] Remove any placeholder severities/symbols; confirm mapping table
  completeness against every finding category in §24.

## 4. Verification

- [x] `cargo fmt --all -- --check`; `cargo build`; `cargo test --all-targets`
  (native long test exclusion recorded if used).
- [x] `cargo clippy --all-targets -- -D warnings`.
- [x] `node scripts/check-openspec-change-names.mjs`; `openspec validate
  --all --strict --no-interactive`; `git diff --check`.
- [x] Optional integration on a host with driftwatchdog: register the
  checker in a scratch project, run `driftwatch check`, and record real
  evidence or the exact blocker.
