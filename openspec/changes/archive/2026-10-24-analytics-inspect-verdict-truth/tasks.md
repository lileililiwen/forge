# Tasks: analytics-inspect-verdict-truth

## 1. BFS baseline

- [x] 1.1 Read the mandatory context: `AGENTS.md`, `.ai-rules/workflow.md`, `.ai-rules/completion.md`.
- [x] 1.2 Read the report: `src/analytics/model.rs` (`ExternalPlaneReport`, `healthy`).
- [x] 1.3 Read the inspection: `src/analytics/projections.rs` (`inspect_external_planes`, `observe_one`, the report construction).
- [x] 1.4 Read the CLI: `src/cli/analytics.rs` (`cmd_analytics_inspect`, the journal row).
- [x] 1.5 Read the statuses: `src/analytics/constants.rs`.
- [x] 1.6 Read the canonical capability: `openspec/specs/external-planes-analytics/spec.md` ("External system identity and health").
- [x] 1.7 Read the existing tests: `tests/analytics_contract.rs`, `tests/analytics_cross_surface.rs`, `src/analytics/projections_tests.rs`.
- [x] 1.8 Reproduce the defect on the current tree and record the observation status, the report's `available` field and the journal row.

## 2. DFS requirement implementation

- [x] 2.1 Decide the rule: an inspection that invoked no adapter planned, an inspection whose providers were all healthy delivered, anything else failed.
- [x] 2.2 Decide where the rule lives: in the analytics model, next to the report it judges, read from the observations.
- [x] 2.3 Decide the verdict's names, its wire shape and its journal state.
- [x] 2.4 Decide that `healthy` is derived from the verdict and that `available` is derived from the observations.
- [x] 2.5 Decide the status a dry run's observations carry: `planned`.
- [x] 2.6 Decide the untouched surfaces: the adapter invocation, the evidence, the notes, the redaction, the metrics lane, the assurance browser, the MCP surface, the contract version.

## 3. Implement

- [x] 3.1 `src/analytics/constants.rs`: the `planned` status.
- [x] 3.2 `src/analytics/model.rs`: `AnalyticsVerdict`, `verdict_for_observations`, `ExternalPlaneReport::from_observations`, the derived `healthy` and `available`.
- [x] 3.3 `src/analytics/projections.rs`: the dry run's observation status, and both report constructions go through the constructor.
- [x] 3.4 `src/cli/analytics.rs`: the journal state and the journal detail follow the verdict.
- [x] 3.5 `src/analytics/mod.rs`: the persistence paragraph names the journal states.
- [x] 3.6 `src/analytics/projections_tests.rs`: rewrite the unit test that enshrined the old dry-run status; add the verdict's unit coverage.
- [x] 3.7 `tests/analytics_inspect_verdict_truth_contract.rs`: the CLI contract for a plan, a delivery and a failure.

## 4. BFS regression and completeness

- [x] 4.1 `cargo build` succeeds and the binary is produced.
- [x] 4.2 The repository test suite passes.
- [x] 4.3 A dry run's observations are `planned`, the verdict is `planned`, the report is not healthy, and the journal records `planned`.
- [x] 4.4 A dry run exits 0.
- [x] 4.5 A healthy inspection's verdict is `delivered`, the report is healthy, the availability agrees with the observations, and the journal records `done`.
- [x] 4.6 A failed provider's verdict is `failed`, the report is not healthy, and the journal records `failed`.
- [x] 4.7 A disabled block still reports `disabled` observations and stays healthy.
- [x] 4.8 The metrics lane, the assurance browser, the MCP surface, the adapter invocation, the evidence and the redaction are unchanged.
- [x] 4.9 Every touched Rust file is rustfmt-clean, and no pre-existing rustfmt-dirty file was touched.
- [x] 4.10 `cargo clippy --workspace --all-targets --all-features` reports no new warning.
- [x] 4.11 Every touched file stays under the 1000 physical line limit.

## 5. Verification

- [x] 5.1 `forge gate --dry-run` resolves every required check.
- [x] 5.2 `forge gate --timeout-secs 2400` passes locally, before the archive.
- [x] 5.3 `openspec validate --all --strict --no-interactive` passes.
- [x] 5.4 `node scripts/check-openspec-change-names.mjs` passes.
- [x] 5.5 `git diff --check` reports no finding.

## 6. Archive

- [ ] 6.1 Archive without `--skip-specs`, so the new capability is promoted.
- [ ] 6.2 Replace the archive's `TBD` Purpose with the real one, and update the modified capability's purpose.
- [ ] 6.3 Commit the implementation with its specs, then update the handoff pointer and commit that separately.

## 7. Evidence

All commands were run from the repository root with
`source /tmp/forge-env.sh` (so `CARGO_TARGET_DIR=/home/paul/code/forge/target`).

| Check | Command | Observed |
|---|---|---|
| Build | `cargo build --workspace` | `Finished dev profile` — no error; the binary is produced. |
| Analytics unit tests | `cargo test -p forge --lib analytics::` | 30 passed; 0 failed. |
| New contract test | `cargo test --test analytics_inspect_verdict_truth_contract` | 5 passed; 0 failed. |
| Analytics contracts | `cargo test --test analytics_contract --test analytics_cross_surface` | 17 passed (contract) + 6 passed (cross-surface); 0 failed. |
| Full suite | `bash scripts/check-tests.sh` | The owning agent's run: exit 0; the `232` `test result:` lines sum to **4220 passed; 0 failed; 1 ignored**. The pre-change baseline, measured the same way, was 231 lines and 4211 passed, so this change adds exactly the 9 tests it declares (4 unit cases and 5 contract cases) and one new test binary. |
| Repository Gate | `forge gate --timeout-secs 2400` | The owning agent's run on the final tree: exit 0; `.forge/gate/forge/evidence.json` records `"aggregate": "passed"` at `"observed_at": "2026-10-24T06:48:39Z"` on revision `c713179432b54283f9d2575c887dcf0760f95abc` (the parent commit — the implementation is uncommitted while the Gate runs), with `build`, `declared-verification`, `placeholder-threshold`, `product-code-boundary`, `repository`, `security`, `source-file-size` and `tests` all in state `pass`. The rehearsal before it (`forge gate --dry-run`) resolved the same 8 required checks, `unbound required checks: none`, `exemption budget: cap=2 declared=0 headroom=2 required=8 verdict=within`. The advisory database's fetch was resolved to the local mirror of the same cached database (`/home/paul/.cache/advisory-db-mirror.git`, commit `640bf5a9`) through `GIT_CONFIG_COUNT` / `GIT_CONFIG_KEY_0=url.file:///home/paul/.cache/advisory-db-mirror.git.insteadOf` / `GIT_CONFIG_VALUE_0=https://github.com/RustSec/advisory-db.git`, and `CARGO_HTTP_PROXY=http://127.0.0.1:1` made the unreachable crates.io index update fail fast instead of waiting for the TCP timeout. Both settings applied to that one Gate process only: no file in this repository and no global cargo or git configuration was changed. |
| Lint | `cargo clippy --workspace --all-targets --all-features` | The owning agent's run: exit 0 with `39` lines starting with `warning`, the same count as the pre-change baseline, and no warning names a touched file. The remaining warnings are pre-existing in untouched files The remaining warnings are pre-existing in untouched files (`src/auth/setup.rs`, `src/contract/mod.rs`, `src/deploy/model.rs`, `src/hosted_ci/store.rs`, `src/registry/model.rs`, `src/cli/auth.rs`, `crates/cargo-audit-boost/*`, `crates/cargo-sonar/*`, `tests/deploy_ssh_adapter_contract.rs`, `tests/origin_repository_retirement_contract.rs`). |
| Format | `rustfmt --edition 2021 --check <file>` on each touched file | 8 files clean: `src/analytics/constants.rs`, `src/analytics/model.rs`, `src/analytics/projections.rs`, `src/analytics/mod.rs`, `src/analytics/projections_tests.rs`, `src/cli/analytics.rs`, `tests/analytics_contract.rs`, `tests/analytics_inspect_verdict_truth_contract.rs`. |
| File size | `wc -l` on each touched file | `src/analytics/constants.rs` 126, `src/analytics/model.rs` 354, `src/analytics/projections.rs` 868, `src/analytics/mod.rs` 96, `src/analytics/projections_tests.rs` 655, `src/cli/analytics.rs` 90, `tests/analytics_contract.rs` 580, `tests/analytics_inspect_verdict_truth_contract.rs` 360 — the largest is 868 lines, under the 1000 physical line limit. |
| Whitespace | `git diff --check` | no finding. |
| Spec validation | `openspec validate --all --strict --no-interactive` | 156 passed, 0 failed (156 items). |
| Change names | `node scripts/check-openspec-change-names.mjs` | `check-openspec-change-names: PASS`. |

### The defect, before and after

Before the change, `forge analytics inspect /tmp/ana-probe/proj --dry-run` printed
`status=available` for every provider and journalled a completed
inspection:

```
  plane=content provider=unified-content project_ref=slug status=available source=dry-run observed_at=2026-10-24T05:58:39.755821154+00:00 note=dry-run: adapter was not invoked
('analytics', 'ana-probe', 'done', 'inspect: project=ana-probe enabled=true observations=2 healthy=true dry_run=true')
```

After the change the same run prints the planned status and journals the
planned verdict:

```
  plane=content provider=unified-content project_ref=slug status=planned source=dry-run observed_at=2026-10-24T06:13:08.054827946+00:00 note=dry-run: adapter was not invoked
('analytics', 'ana-probe', 'planned', 'inspect: project=ana-probe enabled=true observations=2 verdict=planned healthy=false dry_run=true')
```

The report's JSON carries `verdict: planned` and `available: false`; the
exit code stays 0.

### Findings outside the package

- `tests/analytics_contract.rs` (`inspect_accepts_well_formed_block`)
  asserted that the dry run's stdout contains `available`. That
  assertion enshrined the same false claim as the rewritten unit
  test, so it now asserts `planned` and refuses `status=available`.
  The package's risk table named only the unit test.
- The report's `healthy` is a method, not a serialized field, so the
  JSON document exposes the verdict and the availability but no
  `healthy` boolean. The contract test reads the health from the
  journal detail (`healthy=false` / `healthy=true`), which is the
  one place the surface states it.

### The owning agent's additions

- The worker wrote the five source files, the unit tests and the contract
  test, then stopped reporting. The owning agent re-ran every check on the
  final tree — build, the analytics unit tests, the analytics contract
  tests, the full repository suite, clippy, rustfmt, the file-size limit,
  the strict spec validation, the change-name preflight and the Gate —
  and corrected the analytics module's persistence paragraph, which named
  the journal states without the `rejected` state a refused request still
  writes. The Gate run is the owning agent's, because the worker does not
  own the Gate.
