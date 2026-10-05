# Tasks: adapter-request-write-boundary

Phase order is BFS baseline → DFS requirement implementation → BFS
regression/completeness → Verification, per `.ai-rules/workflow.md`. Checkbox
state reflects only work actually completed.

## 1. BFS — Baseline and impact coverage

- [x] 1.1 Confirm all four request-write sites and that none of the three new
  ones mentions `BrokenPipe`: `src/publish/providers.rs:528` (bare `write_all`
  mapped to `PublishInvalid`), `src/docs/mod.rs:675` (bare `write_all` mapped
  to a translator error), `src/delivery/hermora.rs:170` (`let _ =`, discards
  *every* error). `src/governance.rs` is the fourth and was fixed by `21a9566`.
- [x] 1.2 Read all three stubs named in the recorded whole-suite failures and
  confirm the shared property: `#!/bin/sh` + `printf` + exit, never reading
  stdin, so each closes the read end of the pipe as it exits —
  `install_provider_stub` (`tests/delivery_cross_surface.rs:266`),
  `install_hermora` (`tests/delivery_contract.rs:162`), and the docs
  translator stub.
- [x] 1.3 Check the child-leak that `21a9566` addressed, per site:
  `providers.rs` returned through `?` **with the provider still running** —
  a real leak; `docs/mod.rs` already killed and reaped; `hermora.rs` discarded
  the error and therefore waited for the child, so it leaked nothing but
  reported a delivered request it never delivered; `governance.rs` already
  killed and reaped.
- [x] 1.4 Decide shared helper vs. fourth copy, and record the reasoning:
  one `pub fn write_request` in `src/process.rs`, the module this repository
  already documents as the shared child-process boundary. A fourth copy is
  what produced Hermora's `let _ =`. `design.md` §3.1.
- [x] 1.5 Decide what the helper does *not* own — the typed mapping and the
  kill-and-reap stay at the call sites — so the rule keeps a `&mut impl Write`
  signature that a `PermissionDenied` `Write` impl can exercise. A
  `&mut Child` signature could not be unit-tested for the non-broken-pipe arm.
- [x] 1.6 Check each site for the pipe-buffer deadlock shape fixed in
  `governance.rs` by `5d5f103`, and record it as found-and-not-fixed with
  evidence (`design.md` §5): stdout is drained only after `try_wait` reports
  exit at all three, none of them caps stdout, and each is bounded by its own
  existing deadline, so the failure is a timeout verdict rather than a hang.
- [x] 1.7 Confirm the change touches nothing owned elsewhere: no
  `contracts/**`, no `scripts/contract-parity.sh`, no `src/portfolio/share/**`,
  no `tests/portfolio_share_*`, no `tests/manifest_wire_contract.rs`.
- [x] 1.8 Confirm `driftwatch.toml` remains untracked and unstaged.

## 2. DFS — Requirement implementation

- [x] 2.1 Add `process::write_request(&mut impl Write, &[u8]) -> io::Result<()>`:
  `BrokenPipe` is `Ok(())`, every other error is returned unchanged, documented
  as the policy it is (the one error that reports on the child rather than on
  Forge's plumbing) and as not killing the child.
- [x] 2.2 `src/governance.rs`: `write_adapter_request` delegates to it. Name,
  doc comment, typed `GovernanceUnavailable` mapping and both landed guards
  unchanged; `run_adapter` unchanged.
- [x] 2.3 `src/publish/providers.rs:529`: the shared rule plus kill-and-reap
  before the existing `PublishInvalid` refusal. The refusal text is unchanged.
- [x] 2.4 `src/docs/mod.rs:676`: the shared rule; its existing kill-and-reap
  and message unchanged.
- [x] 2.5 `src/delivery/hermora.rs:173`: `let _ = write_all` becomes the shared
  rule plus kill-and-reap plus a typed `DeliveryUnavailable` refusal. The
  comment above it, which already stated the correct intent, is now true of the
  code under it.
- [x] 2.6 Drop the imports the three sites no longer need
  (`std::io::Write` in `hermora.rs`, `Write` from the local `use` in
  `docs/mod.rs`, and the inline `use std::io::Write;` in `providers.rs`), so no
  warning is added.

## 3. BFS — Regression and completeness

- [x] 3.1 Add the deterministic guard
  `a_request_write_to_a_child_that_already_exited_is_not_a_failure`: spawn
  `/bin/sh -c 'exec 0<&-; exit 0'`, **reap it**, then write. Deterministic by
  construction — the read end is provably closed before the write.
- [x] 3.2 Add the guard that pins the retained refusal and catches a
  reintroduced `let _ =`:
  `a_request_write_failure_that_is_not_a_broken_pipe_is_returned`.
- [x] 3.3 Confirm both guards fail against the pre-fix behaviour, with the
  `BrokenPipe` arm removed, rather than assuming it. See "Verification" 4.3.
- [x] 3.4 Leave the three end-to-end tests that reported the flake intact and
  unannotated — no `#[ignore]`, no serialisation, no sleep, no retry, no
  weakened assertion.
- [x] 3.5 Confirm the two landed governance guards still pass unchanged.
- [x] 3.6 Confirm the `Ok` path is byte-identical for every adapter that reads
  stdin: the change adds no bytes, no state and no ordering on that path.

## 4. Verification

| # | Check | Result |
|---|---|---|
| 4.1 | `cargo test --test delivery_cross_surface` × 30 | **30/30** `9 passed; 0 failed` |
| 4.2 | `cargo test --test delivery_contract` × 30 | **30/30** `17 passed; 0 failed` |
| 4.3 | `cargo test --test docs_contract` × 30 | **30/30** `12 passed; 0 failed` |
| 4.4 | `cargo test --lib` (with the documented `generate::tests::rust_scaffold_builds_and_tests_with_native_toolchain` skip) × 30 | **30/30** `1180 passed; 0 failed` after the port-slot fix; an earlier block of 30 was **29/30**, its one failure the pre-existing `Text file busy` flake described below |
| 4.5 | all eight targets at `--test-threads=8`, 10 runs each | **80/80 clean** |
| 4.6 | all eight targets at `--test-threads=1`, 10 runs each | **80/80 clean** |
| 4.7 | the two new guards against the **pre-fix** behaviour (the `BrokenPipe` arm removed) | `a_request_write_to_a_child_that_already_exited_is_not_a_failure` **fails 1/1** with `Os { code: 32, kind: BrokenPipe }`, so it is not vacuous. `a_request_write_failure_that_is_not_a_broken_pipe_is_returned` passes both ways by design: it pins the *retained* refusal |
| 4.8 | `cargo test --workspace --all-targets --no-fail-fast -- --skip generate::tests::rust_scaffold_builds_and_tests_with_native_toolchain`, 6 runs | exact totals in the session report |
| 4.9 | `cargo fmt --check` | clean |
| 4.10 | `cargo clippy --workspace --all-targets` | exit 0; `--message-format=short` attributes **zero** diagnostics to `src/process.rs`, `src/publish/providers.rs`, `src/docs/mod.rs`, `src/delivery/hermora.rs` or `src/governance.rs` |
| 4.11 | `git diff --check` | PASS |
| 4.12 | `node scripts/check-openspec-change-names.mjs` | PASS |
| 4.13 | `openspec validate --all --strict --no-interactive` | passed / 0 failed |
| 4.14 | Archive | **deliberately not run**: five changes were already in flight and unarchived; nothing may be archived while they are parked |

## 5. Outstanding

- **The pipe-buffer deadlock shape at all three new sites.** Found, evidenced
  and deliberately not fixed — `design.md` §5. It needs the bounded-run
  unification that `5d5f103` declined on scope and that this change declines
  again as non-goal 1.
- **`Text file busy (os error 26)`, three lib sites.** Pre-existing, observed
  twice in this session's `cargo test --lib` runs (1 in 30, then 1 in 40, then 1
  on the first `strace` attempt) and **0 in 200 isolated runs of the single
  test**. Root-caused to a syscall-level mechanism and left named and owned in
  `HANDOFF.md`; it is not caused by this change and is not claimed fixed by it.
- **No deterministic test exists for a genuine (non-`BrokenPipe`) write failure
  at a call site.** A real pipe cannot be made to fail with `EIO` from outside
  the process, so the retained refusal is pinned at the shared helper and by
  reading the four call sites, not end to end. Stated rather than papered over.
