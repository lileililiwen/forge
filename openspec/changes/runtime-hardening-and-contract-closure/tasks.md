# Tasks: runtime-hardening-and-contract-closure

Evidence is labelled by source. **RR** = re-run by this session on 2026-10-05.
**R** = recorded by the absorbed package on 2026-10-05 and not re-measured here.

## 1. BFS — Baseline and impact coverage

- [x] 1.1 Establish that the seven packages were implemented and committed but not
  promoted: `git log --oneline -- openspec/specs` returns nothing after `5019d12`,
  while `c138f08`..`1c3e1f5` carry the code. **RR**
- [x] 1.2 Record why each archive was skipped and test the reason. The blocker in
  `contract-parity-gate-real-digests` §4.4 (`check-spec-governance.mjs` failing on
  `scaffold-prewires-shared-layer/spec.md`) was fixed by `5019d12`; the check now
  reports PASS. The other six reasons were "the other changes are parked", which
  is circular. **RR**
- [x] 1.3 Map the file-level entanglement that made separate promotion wrong:
  `src/governance.rs` edited by `21a9566` + `5d5f103` + `e3a156b`,
  `tests/governance_contract.rs` by two, `tests/studio_preview_contract.rs` written
  by `5d5f103` and rewritten by `f078a4c`. **RR**
- [x] 1.4 Confirm the two duplicate-prone delta pairs before merging: the port
  deltas shared scenarios 1–3 verbatim, and the request-write deltas shared three of
  four scenarios. **RR**
- [x] 1.5 Confirm no merged requirement name collides with an existing canonical
  requirement in the five target specs. **RR**
- [x] 1.6 Confirm the host ephemeral window that made the port tests flaky:
  `/proc/sys/net/ipv4/ip_local_port_range` = `32768 60999`. **R**

## 2. DFS — Requirement implementation

Committed before this package existed. Cited by commit, not restated.

- [x] 2.1 `src/process.rs::write_request` is the one request-write boundary;
  `BrokenPipe` is `Ok(())`, every other error returns. Call sites converted:
  `src/publish/providers.rs:529`, `src/docs/mod.rs:676`, `src/delivery/hermora.rs:173`,
  `src/governance.rs:902`. `e3a156b`
- [x] 2.2 `run_adapter` drains both pipes concurrently, waits on `recv_timeout`,
  keeps the read bounded by `MAX_ADAPTER_OUTPUT_BYTES`, kills and reaps on every
  timeout, wait-failure and read-failure path, and detains its drain threads
  because `/bin/sh -c` forks. `5d5f103`
- [x] 2.3 `git_revision` is bounded by the selected provider's `timeout_ms` and
  records no revision on failure, timeout or unusable output. `5d5f103`
- [x] 2.4 `tests/support/studio_ports.rs` selects one run-time window outside the
  ephemeral range for all four Studio targets, per-target candidate index, holding
  the verifying listeners for the collision test. `5d5f103` + `f078a4c`
- [x] 2.5 The preview port allocator refuses a fully busy range with
  `studio-port-unavailable` and leaves every unrelated listener alive and bound.
  `5d5f103`
- [x] 2.6 `scripts/contract-parity.sh` compares real bytes, counts comparisons,
  refuses a zero-comparison pass, an unresolvable source, and a self-referencing
  source. `c138f08`
- [x] 2.7 `wire_manifest_revision` emits `platform.public-portfolio-manifest`,
  `"1.0.0"` and `"rev_<n>"`, with the internal revision left an integer in storage,
  approval, audit, report and envelope. `619b945`
- [x] 2.8 Guards pinning each mechanism, all checked against the pre-fix code by
  reverting the fix rather than trusting it: the two `write_request` unit guards,
  the four bounded-run guards, the Studio liveness guard, the held-range negative
  control. `21a9566` + `5d5f103` + `e3a156b` + `f078a4c`. Detail in `design.md` §6

## 3. Consolidation actions

- [x] 3.1 Merge the request-write rule into one requirement in
  `governance-provider-contract`, with the wider four-boundary scope and all four
  unioned scenarios; do not also file it under `runtime-hardening-and-test-isolation`.
- [x] 3.2 Merge the two Studio port requirements into one in
  `runtime-hardening-and-test-isolation` carrying the union of six scenarios.
- [x] 3.3 Carry `site-studio-preview-refinement`, `platform-contract-consumption`
  and `portfolio-share` deltas verbatim. Verified byte-identical to their sources.
- [x] 3.4 Remove the seven superseded active packages. Their full text remains in
  git from `c138f08` to `1c3e1f5`.
- [x] 3.5 Correct the two statements that were true of their own scope and
  contradictory across packages: bounded-process-run §6 lists the three sibling
  request writes as unfixed (fixed by `e3a156b`), and write-race §5 calls the Studio
  port flake an outside pre-existing flake (fixed by `5d5f103` / `f078a4c`). Both are
  now in scope of this package and recorded as closed.

## 4. Verification — re-run at archive time (2026-10-05)

- [x] 4.1 `cargo test --workspace --all-targets --no-fail-fast -- --skip
  generate::tests::rust_scaffold_builds_and_tests_with_native_toolchain` →
  **2305 passed / 0 failed / 3 ignored**. **RR**
- [x] 4.2 `cargo test --test manifest_wire_contract -- --ignored` → **1 passed**
  (`jsonschema` accepted a Forge-produced manifest against the sibling schema). **RR**
- [x] 4.3 `bash scripts/contract-parity.sh` → `13 retained file(s) compared
  byte-for-byte, 10 published family digest(s) checked against the source record`,
  `OK`, **exit 0**. This is the gate that previously printed an unconditional pass
  while comparing nothing. **RR**
- [x] 4.4 `cargo fmt --all -- --check` → clean. **RR**
- [x] 4.5 `cargo clippy --workspace --all-targets` → exit 0; the warnings are in
  test targets this work does not touch (`github_adapter_contract` and peers). **RR**
- [x] 4.6 `node scripts/check-openspec-change-names.mjs` → PASS. **RR**
- [x] 4.7 `node scripts/check-spec-governance.mjs` → PASS. This is the check that
  jammed the archive queue; the blocker it reported is gone. **RR**
- [x] 4.8 `git diff --check` → PASS. **RR**
- [x] 4.9 Targeted targets re-run individually before the aggregate:
  `governance_contract` 23 passed, `studio_preview_contract` 7 passed,
  `studio_api_contract` 2 passed. **RR**

### Not re-measured by this session, carried from the absorbed packages

- [x] 4.10 **R** 400 consecutive runs of the eight affected Studio and governance
  targets (240 default, 80 at `--test-threads=8`, 80 at `=1`), all clean;
  `governance_contract` 244 of 245 (§6.6).
- [x] 4.11 **R** held-range experiment (256 ports of the four old bases held by an
  unrelated process) with its negative control failing on the pre-fix bases.
- [x] 4.12 **R** whole suite 6 runs applied against 6 stashed: 3 of 6 clean applied,
  1 of 6 clean stashed.
- [x] 4.13 **R** live opt-in path `FORGE_NATIVE_REACT_WEB_PREVIEW=1`, 6 runs, 6
  passed, each a real `npm install`, Vite dev server and Playwright render.
- [x] 4.14 **R** manifest consumer acceptance through the consumer's own oracle:
  `validate_manifest.py --strict` → `manifest OK … revision rev_1`, exit 0.
- [x] 4.15 **R** the `ETXTBSY` syscall trace and the 1-in-30 / 1-in-40 / 0-in-200
  frequency measurements.

## 5. Archive

- [x] 5.1 `openspec archive` with spec promotion, no `--skip-specs`. Owner directed
  the consolidation and the archive on 2026-10-05, which supersedes the seven
  "deliberately not run" entries this package replaces.

## 6. Outstanding

- [x] 6.1 **The pipe-buffer deadlock shape at the three non-governance boundaries**
  — `src/publish/providers.rs:589`, `src/docs/mod.rs:684`,
  `src/delivery/hermora.rs:180`. A misclassification ("timed out"), not a hang.
  Needs the bounded-run unification this package declines as a non-goal.
  `design.md` §5.1.
- [x] 6.2 **Two bounded-run implementations** with different error taxonomies
  (`process::spawn_with_timeout` vs `governance::run_adapter`), plus seventeen
  remaining `try_wait` loops. `design.md` §5.2.
- [x] 6.3 **The allocator's bind/drop window**, and slots being a preference rather
  than a reservation. A product defect, produced one measured failure.
  `design.md` §5.3.
- [x] 6.4 **Three contract mismatches reachable today** — `visibility` admits
  `unlisted`, `status_evidence` admits three keys the schema forbids, `id` has no
  length bound. Needs a product decision on who narrows. `design.md` §5.4.
- [x] 6.5 **`Text file busy (os error 26)`**, root-caused to a fork/exec
  write-descriptor race at four fixture-staging sites; no fix fits one change.
  `design.md` §5.5 and `HANDOFF.md`.
- [x] 6.6 **One unexplained failure in 245 runs** of `governance_contract`, never
  attributed. Not claimed fixed. `design.md` §5.6.
- [x] 6.7 **Two `#[ignore]`d tests** remain: `contract::tests::parity_walk` and
  `kit_contract::packing_leaves_the_sibling_checkout_byte_identical` (a heavy
  parallel pack pushes an unrelated 5 s adapter timeout). Both run explicitly.
- [x] 6.8 **No shared Gate pass** is claimed. `forge gate` consumption exists; no
  executed gate pass is recorded for this repository.
