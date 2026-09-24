# Tasks: DriftWatch policy adapter alignment

## 1. BFS — Baseline and impact coverage

- [x] Inventory the current invocation, PolicyReport schema, redaction path,
  doctor detection list, provider matrix row and every caller.
- [x] Confirm the real Driftwatchdog CLI surface against the sibling repo
  (`check`, `gate --format json`) and record the accepted envelope fixtures.
  Live probes against the sibling release build at commit `25811ed`
  corrected one design assumption: `gate --dry-run` prints the human plan
  and never reaches its JSON writer, so the gate surface is the real
  `gate --format json` (design.md carries the correction).
- [x] Map each new requirement/scenario to parser, adapter, doctor and
  provider-matrix layers plus fixture files.
- [x] Add checker-report and gate-status JSON fixtures (passing, blocked,
  unknown-contract, malformed) before implementation.
  `tests/fixtures/driftwatch/` holds verbatim sibling captures plus
  `NOTES.md` with the full probed surface matrix.

## 2. DFS — Requirement-by-requirement implementation

- [x] Implement ordered binary resolution (override → driftwatchdog →
  driftwatch) with tests.
- [x] Rework `run_driftwatch` to the real invocation: cwd confinement, no
  `--project`; add gate fallback for gate-manifest projects.
- [x] Implement envelope parsing and PolicyReport normalization for both
  document shapes; blocked gate → fail findings, unknown contract →
  `unavailable` with version named.
- [x] Extend `detect_driftwatch` to `driftwatch.toml`, `gate.toml`,
  `.ai-gate/gate.yaml`.
- [x] Update `docs/provider-evidence.md` command surface and probe wording.

## 3. BFS — Cross-surface regression and completeness

- [x] Re-verify release-check and provider-runner paths consume the same
  normalized report; MCP/API/portal surfaces unchanged (transport parity).
- [x] Re-verify stale, timeout, non-zero-exit and credential-redaction
  boundaries; no path yields PASS from absent evidence.
- [x] Re-verify fixture-only hosts (no real binary) keep every prior test
  green; empty-PATH behavior unchanged. (The pre-alignment
  `two_projects_each_get_their_own_policy_observation` fixture read the
  project from the removed `--project` argv position; it now derives the
  scope from its working directory, the real confinement guarantee.)
- [x] On a host with driftwatchdog installed, run one live `forge provider
  driftwatch-policy --live` round trip and record evidence or the exact
  blocking failure. Recorded 2026-09-24: PATH-resolved ordered probe to a
  release build of the sibling → row `supported` (`sandbox: live`,
  `source: live:driftwatchdog`) with `contract=driftwatch-checker/0.1.0`
  and `checkers=3` evidence; gate-managed project → real `gate --format
  json`, blocked document recorded as `gate_blocked=true` evidence with
  the exit-1 note, and the only state written was one `gate_runs` row in
  the project's own `.driftwatch/state.db`; the stale `~/.cargo/bin`
  install (pre-envelope) is honestly classified `unavailable` naming its
  exit-2 `--format` rejection; credential-shaped alert text reached the
  row only as `[REDACTED]`.

## 4. Verification

- [x] `cargo fmt --all -- --check`; `cargo build`; `cargo test --all-targets`
  (native-toolchain long test may be excluded with the recorded reason).
- [x] `cargo clippy --all-targets -- -D warnings`.
- [x] `node scripts/check-openspec-change-names.mjs`; `openspec validate
  --all --strict --no-interactive`; `git diff --check`.
- [x] Record actual command output as evidence; environment-blocked checks
  (live driftwatchdog run) name the exact next action, never a pass.
