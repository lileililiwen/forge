# Tasks: governance-adapter-request-write-race

Phase order is BFS baseline → DFS requirement implementation → BFS
regression/completeness → Verification, per `.ai-rules/workflow.md`. Checkbox
state reflects only work actually completed.

## 1. BFS — Baseline and impact coverage

- [x] 1.1 Reproduce the flake from the target itself: 20 consecutive
  `cargo test --test governance_contract` runs, recording which test failed
  each time. Result: **13 failures across 8 of 20 runs**, confined to exactly
  four tests — `valid_external_response_is_normalized_and_redacted` (5),
  `unknown_external_status_is_incompatible` (3),
  `malformed_external_response_is_incompatible` (3),
  `workspace_governance_evidence_is_redacted_and_bounded` (2). No other test
  in the target failed once.
- [x] 1.2 Read all four failing tests and find the property they share that no
  other test has: their adapter script is `#!/bin/sh` + `printf` only and
  **never reads stdin**. The six passing adapters all begin `req=$(cat)`,
  which blocks the writer until the request is consumed.
- [x] 1.3 Capture the actual failure detail instead of assuming one. A
  temporary instrumented harness over the real `save_provider_selection` +
  `check_project` path produced
  `detail=Some("cannot write adapter request: Broken pipe (os error 32)")` with
  `status=Unavailable`. This rules out the 5-second timeout (the whole target
  finishes in 0.05–0.12 s), a spawn failure, and a non-zero adapter exit.
- [x] 1.4 Test, and reject, the shared-mutable-state hypothesis. 20 runs with
  `--test-threads=1` produced **7 failures across 4 runs** — serialising does
  not fix it, so the defect is not cross-test shared state.
- [x] 1.5 Audit every cross-test global reachable from this target and record
  each as cleared: no `set_var`/`remove_var` in the test or in
  `src/governance.rs`/`src/core`; no `set_current_dir` in `src/`; no `static` /
  `OnceLock` / `lazy_static` on this path (`src/vocabulary.rs`'s lock and
  thread-local override are not reachable from it); per-test `TempDir::new()`;
  the pid-keyed `.providers.yaml.tmp-<pid>` and `.observations.json.tmp-<pid>`
  names cannot collide because each test owns a distinct directory; no socket;
  the `tests/fixtures/governance-audit/` fixtures are read-only and the one
  `remove_file` targets a staged copy in the test's own `TempDir`.
- [x] 1.6 Map the consumers of the write failure:
  `run_external_provider` (`src/governance.rs:475-487`) maps
  `GovernanceUnavailable` to `ProviderStatus::Unavailable`, which
  `Provider failure isolation` requires. The mapping is correct; the input was
  false. The wait/read path at `src/governance.rs:619-668` already holds the
  child's answer and is not the defect.
- [x] 1.7 Record three adjacent defects as explicit non-goals rather than
  widening this change: stdout is not drained while the child runs (a 256 KiB
  allowance against a 64 KiB pipe buffer deadlocks), `git_revision` has no
  deadline, and the wait loop polls at 10 ms.

## 2. DFS — Requirement implementation

- [x] 2.1 Extract the request write in `run_adapter` into
  `write_adapter_request(&mut impl Write, &[u8]) -> Result<(), ForgeError>`,
  documented as: a `BrokenPipe` is the adapter having closed its input, and is
  `Ok(())`; every other error keeps the typed `GovernanceUnavailable` refusal.
- [x] 2.2 Call it from `run_adapter` so that a `BrokenPipe` falls through to the
  existing wait/read path, and the adapter's real exit status, stdout and
  stderr decide the observation.
- [x] 2.3 On the genuine-error path only, kill and reap the child before
  returning the refusal, so no live adapter is left behind. The `BrokenPipe`
  path must not kill — that child is the one whose answer we want.

## 3. BFS — Regression and completeness

- [x] 3.1 Add the deterministic boundary guard
  `a_request_write_to_an_adapter_that_already_exited_is_not_a_failure`: spawn
  `/bin/sh -c 'exec 0<&-; exit 0'`, **reap it**, then write. Deterministic by
  construction, because the child is provably gone before the write.
- [x] 3.2 Confirm that guard actually fails against the pre-fix behaviour
  (bare `write_all(...)?`) and is not a vacuous test.
- [x] 3.3 Add the end-to-end guard
  `an_adapter_that_never_reads_its_request_still_answers` to
  `tests/governance_contract.rs`, whose adapter closes its own stdin and then
  answers, so the scenario is named in the suite instead of left to
  scheduling luck.
- [x] 3.4 Leave the four previously-flaky tests intact — no `#[ignore]`, no
  serialisation, no conditional skip. Their coverage is unchanged and they now
  pass every run.
- [x] 3.5 Leave the six `req=$(cat)` adapters and the four recorded fixtures in
  `tests/fixtures/governance-audit/` untouched, and confirm the `Ok` path is
  byte-identical.
- [x] 3.6 Confirm the diff touches nothing owned elsewhere: no `contracts/**`,
  no `scripts/contract-parity.sh`, no `src/portfolio/share/**`, no
  `tests/portfolio_share_*`, no `tests/manifest_wire_contract.rs`, and neither
  of the two other in-flight change directories.
- [x] 3.7 Confirm `driftwatch.toml` remains untracked and unstaged.

## 4. Verification

- [x] 4.1 `cargo test --test governance_contract` at default parallelism,
  **245 consecutive runs across five blocks**. **244 passed, 1 failed.** The
  final contiguous block of 20 was **20/20 passed** (`21 passed; 0 failed`
  each); the single failure is recorded in "Outstanding" below and is **not**
  claimed as fixed. Pre-fix baseline for the same command: 13 failures across
  8 of 20 runs.
- [x] 4.2 `cargo test --test governance_contract -- --test-threads=8`,
  **20 runs, 20 passed** (`21 passed; 0 failed` each).
- [x] 4.3 `cargo test --test governance_contract -- --test-threads=1`,
  **20 runs, 20 passed**. Pre-fix baseline: 7 failures across 4 of 20 runs,
  which is the measurement that disproved the shared-state hypothesis.
- [x] 4.4 `cargo test --workspace --all-targets --no-fail-fast -- --skip
  generate::tests::rust_scaffold_builds_and_tests_with_native_toolchain`.
  Run 1: **2296 passed / 1 failed / 3 ignored**. The one failure is
  `tests/studio_preview_contract.rs::preview_port_collision_is_refused_without_killing_a_listener`
  (`left: 63, right: 64`) — a separate, pre-existing flake outside this change,
  analysed in "Outstanding". Run 2: **2297 passed / 0 failed / 3 ignored**,
  exit 0. The 3 ignored are the pre-existing `contract::tests::parity_walk`,
  the pre-existing `packing_leaves_the_sibling_checkout_byte_identical`, and
  the `manifest-wire-contract-shape` acceptance test.
- [x] 4.5 `cargo fmt --check`: clean.
- [x] 4.6 `cargo clippy --workspace --all-targets`: exit 0, and
  `--message-format=short` attributes **zero** diagnostics to
  `src/governance.rs` or `tests/governance_contract.rs`. Every remaining
  warning is in a file this change does not own.
- [x] 4.7 `git diff --check`: PASS.
- [x] 4.8 `node scripts/check-openspec-change-names.mjs`: PASS.
- [x] 4.9 `openspec validate --all --strict --no-interactive`:
  **66 passed / 0 failed**, including `change/governance-adapter-request-write-race`.
- [ ] 4.10 Archive. Deliberately not run: the owner authorised this work

## 5. Outstanding

- **One unexplained failure in 245 runs** at default parallelism. The capture
  recorded `20 passed; 1 failed` but not which test or why. Not reproduced in
  20 runs at `--test-threads=8`, 20 at `--test-threads=1`, or 220 further
  default runs. Not claimed as fixed.
- **`studio_preview_contract::preview_port_collision_is_refused_without_killing_a_listener`
  is a separate, pre-existing flake.** It binds the fixed range
  45800–45863, which lies inside this machine's Linux ephemeral range
  (`/proc/sys/net/ipv4/ip_local_port_range` = `32768 60999`), so a concurrent
  outbound connection from any process can take one of the 64 ports.
  Mechanism **proved**, not assumed: a process holding the range makes the
  same assertion fail (`left: 0, right: 64` with all 64 held). It passes in
  isolation. Owned by its own change, not this one.
- **Three adjacent defects in `run_adapter`**, recorded in `design.md` §5 and
  not fixed here: stdout is not drained while the child runs (a 256 KiB
  allowance against a 64 KiB pipe buffer deadlocks); `git_revision` has no
  deadline; the wait loop polls at 10 ms. None is the cause of this flake.
  alongside the parked `contract-parity-gate-real-digests` and the unarchived
  `manifest-wire-contract-shape`, and `.ai-rules/workflow.md` allows one active
  change at a time. The change is left active with every implementation task
  evidenced.
