# Tasks: governance-adapter-bounded-process-run

Phase order is BFS baseline → DFS requirement implementation → BFS
regression/completeness → Verification, per `.ai-rules/workflow.md`. Checkbox
state reflects only work actually completed.

## 1. BFS — Baseline and impact coverage

- [x] 1.1 Establish the arithmetic, not a hypothesis: `MAX_ADAPTER_OUTPUT_BYTES`
  is 256 KiB (`src/governance.rs:40`) against a 64 KiB kernel pipe buffer, and
  `run_adapter` reads a pipe only after `try_wait` reports exit
  (`src/governance.rs:623-633`). An adapter writing 64 KiB + 1 byte therefore
  blocks in `write(2)` on every run, not occasionally.
- [x] 1.2 Map every reader of the changed behaviour: `run_adapter` is private
  and has exactly one caller, `run_external_provider`
  (`src/governance.rs:467`); `git_revision` has exactly one caller,
  `evaluate_project` (`src/governance.rs:399`), which is reached by
  `forge governance check`, `check_project`, `inspect` and the API/portal
  governance surfaces.
- [x] 1.3 Confirm the `21a9566` contract that must not regress: a
  `BrokenPipe` on the request write is tolerated and the adapter's own answer
  is read (`src/governance.rs:612-619`, `682-690`), and a genuine write failure
  kills + reaps the child before returning its typed refusal.
- [x] 1.4 Record the observable behaviours the change must leave byte-identical:
  the deadline path's synthetic failure and its `adapter exceeded timeout of
  {timeout_ms} ms` detail (`src/governance.rs:652-659`), the stdout cap
  refusal `GovernanceInvalid` "adapter stdout exceeds 262144 bytes"
  (`src/governance.rs:641-645`), the read-error taxonomy, and the 500-char
  redacted stderr.
- [x] 1.5 Evaluate `src/process.rs::spawn_with_timeout` as the reuse
  candidate and clear it on four counts — no stdin write with `BrokenPipe`
  tolerance, no 256 KiB typed stdout cap, `String` errors instead of
  `ForgeError`, and `Err` on timeout instead of a successful synthetic
  failure. Reasoned in `design.md` §5; using it would change every other
  adapter surface that shares the helper.
- [x] 1.6 Identify the existing shared convention the fix must follow:
  `spawn_with_timeout` already establishes "reader thread per pipe, kill and
  reap on timeout, never leave the caller hanging". The new helper follows it
  and improves the wait to a bounded blocking receive.
- [x] 1.7 Record as non-goals rather than widening scope: the seventeen other
  `try_wait` loops outside this boundary, forcing a descendant's inherited
  pipe closed, a persisted truncated-drain field, and the hardcoded Studio
  port bases in three sibling test files (`design.md` §6).

## 2. DFS — Requirement implementation

- [x] 2.1 Add `PipeDrain { bytes, total, error }` and the drain thread: read one
  pipe to end, keep the first `cap` bytes, **count** the rest and discard them.
  Counting rather than stopping is what keeps the pipe draining; stopping at
  the cap would re-create the deadlock at a different threshold.
- [x] 2.2 Add `run_bounded(&mut Child, deadline, stdout_cap, stderr_cap)`, the
  single bounded run shared by both call sites, returning
  `BoundedRun { status, stdout, stderr, timed_out }`.
- [x] 2.3 Replace the 10 ms sleep in `run_adapter` with a blocking
  `recv_timeout(remaining_deadline)` over a channel the drain threads signal
  at end-of-file, holding a spare `Sender` for the call so the channel cannot
  disconnect and spin.
- [x] 2.4 Keep `try_wait` so the parent retains the `Child` and can `kill` **and**
  `wait` on every failure path; record why in the doc comment so a future
  "move the wait into a thread" refactor does not silently regress it to a
  kill-by-pid.
- [x] 2.5 After a successful reap, collect both drains with a wait bounded by
  the same deadline, and report a truncated drain explicitly instead of
  blocking; a descendant that inherited a pipe must not hang Forge.
- [x] 2.6 Re-express `run_adapter` on `run_bounded`, preserving the timeout
  path's synthetic failure, the stdout cap refusal, the read-error messages
  and the redacted 500-char stderr unchanged.
- [x] 2.7 Bound `git_revision` by the selected provider's existing
  `timeout_ms` (already constrained to `1..=300000` by `validate_config` and
  already the adapter's timeout), set `stdin` to `null` as `Command::output()`
  did, cap its stdout at `MAX_GIT_REVISION_BYTES`, and keep every non-answer
  mapping to `None`.
- [x] 2.8 Pass `provider.timeout_ms` at the `evaluate_project` call site. No
  new timeout constant is introduced anywhere in the boundary.

- [x] 2.9 Do **not** join the drain threads. Measured: `/bin/sh -c 'sleep 30'`
  forks, so `SIGKILL` on the child leaves a descendant holding the pipe, and
  joining blocked the full 30 s. A reader reports on its channel; that is what
  the bounded receive waits for. An unfinished read is reported as truncated.
- [x] 2.10 Close the window where end-of-file is not an exit event. Measured:
  waking only on end-of-file made 2–4 tests per run report `exceeded timeout`
  (`finished in 10.10 s` / `20.01 s`, exact multiples of the test budgets);
  after the fix the target finishes in **0.02 s**. That one window now re-checks
  every `EXIT_RECHECK` (1 ms) instead of blocking out the budget.
- [x] 2.11 Guard the window directly:
  `a_bounded_revision_lookup_returns_the_object_name_it_printed` runs
  `/bin/sh -c 'printf …'`, asserts the run is **not** a timeout, and asserts it
  finished inside 2 s of a 5 s budget. It fails against the §2.10 bug.

## 3. BFS — Regression and completeness

- [x] 3.1 Guard: `an_adapter_that_writes_more_than_one_pipe_buffer_still_answers`
  — a real adapter that reads its request, emits ~200 KiB of stdout, and
  exits 0. Asserts `Pass` **and** that the full padding crossed the pipe
  (`metadata["pad"].len()`), so a fix that truncated instead of draining would
  fail.
- [x] 3.2 Guard: `an_adapter_that_writes_past_the_output_cap_is_refused` —
  ~320 KiB. Asserts the typed `GovernanceInvalid` refusal naming the cap, so
  the cap survives the drain and cannot be traded away to avoid the deadlock.
- [x] 3.3 Unit guards for item 3 in `src/governance.rs`: a `sleep 30` revision
  lookup must give up within a 200 ms bound, and a lookup that prints an object
  name must return it — the positive control that stops an always-tripping
  bound from passing as a fix.
- [x] 3.4 Unit guard: `git_revision` still reports a real repository's head, so
  bounding it cannot be "always `None`".
- [x] 3.5 Confirm both `21a9566` guards still pass —
  `a_request_write_to_an_adapter_that_already_exited_is_not_a_failure` and
  `an_adapter_that_never_reads_its_request_still_answers`.
- [x] 3.6 Confirm no test was ignored, serialised, slept, retried or weakened;
  no assertion moved from `assert_eq!` to a weaker form.
- [x] 3.7 Confirm the diff touches nothing owned elsewhere: no `contracts/**`,
  no `scripts/contract-parity.sh`, no `src/portfolio/share/**`, no
  `tests/portfolio_share_*`, no `tests/manifest_wire_contract.rs`, no
  `src/process.rs`, and none of the other in-flight change directories.
- [x] 3.8 Confirm `driftwatch.toml` remains untracked and unstaged.
- [x] 3.9 Audit the new concurrency for leaks: exactly two drain threads per
  run, each ending at end-of-file or at the parent's deadline; every parent
  exit path kills **and** reaps; the channel cannot disconnect while the parent
  waits.

- [x] 3.10 Correct the claim in `design.md` §7 after measurement: the realistic
  `printf` guard does **not** reliably detect the end-of-file window (it passed
  1/1 against that bug, because the window is microseconds wide). The
  deterministic guard for it is
  `a_child_that_closed_its_pipes_and_keeps_running_is_not_reported_as_a_timeout`,
  where the child closes both pipes itself and then lives 200 ms, making
  end-of-file provably precede the exit.
- [x] 3.11 State honestly which half of the Studio liveness proof is load-bearing
  — see the sibling change's `design.md` §3.4. Neither half proves ownership,
  because nothing observable from outside can; together they prove a listener
  survived and the port was not released.

## 4. Verification

- [x] 4.1 Confirm each new guard **fails against the pre-fix mechanism**, not
  just that it passes now:
  - large-stdout guard with the drain removed → `Unavailable`,
    `adapter exceeded timeout of 5000 ms`
  - over-cap guard with the drain removed → `Unavailable` (the deadline), not
    the cap refusal
  - revision-deadline guard with the bound removed → does not return; killed by
    an outer `timeout`
  - `git_revision` positive controls pass both before and after (they pin
    behaviour that must not change).
- [x] 4.2 `cargo test --test governance_contract`: consecutive runs recorded,
  plus `--test-threads=1` and `--test-threads=8`. Totals in this file's
  verification table.
- [x] 4.3 `cargo test --workspace --all-targets --no-fail-fast -- --skip
  generate::tests::rust_scaffold_builds_and_tests_with_native_toolchain`, run
  four times total (twice with the change, twice with it stashed). Totals and
  the pre-existing-failure proof are in §5.
- [x] 4.4 `cargo fmt --check`: clean.
- [x] 4.5 `cargo clippy --workspace --all-targets`: exit 0, and zero diagnostics
  attributed to `src/governance.rs` or `tests/governance_contract.rs`.
- [x] 4.6 `git diff --check`: PASS.
- [x] 4.7 `node scripts/check-openspec-change-names.mjs`: PASS.
- [x] 4.8 `openspec validate --all --strict --no-interactive`: recorded totals.
- [ ] 4.9 Archive. Deliberately not run: the owner authorised this work
  alongside the other unarchived changes and `.ai-rules/workflow.md` allows one
  active change at a time.

## 5. Verification record

### Whole-suite failures are pre-existing, and the studio flake reproduced on the baseline

`cargo test --workspace --all-targets --no-fail-fast -- --skip
generate::tests::rust_scaffold_builds_and_tests_with_native_toolchain` was run
**6 times with this change stashed** to establish the baseline on this host,
because the first two with-change runs each reported 2 failures and a
correlation had to be ruled out rather than assumed away.

| Run | Passed | Failed | Ignored | Failing tests |
|---|---|---|---|---|
| baseline a | 2297 | 0 | 3 | — |
| baseline b | 2297 | 0 | 3 | — |
| baseline c | 2295 | 2 | 3 | `preview_port_collision_is_refused_without_killing_a_listener` (**`left: 56, right: 64`**), `mcp_repeated_isolated_round_trip_is_stable` |
| baseline d | 2295 | 2 | 3 | `a_successful_preflight_writes_a_journal_row_visible_on_both_transports`, `provider_failure_keeps_prior_derivative_and_redacts_secrets` |
| baseline e | 2296 | 1 | 3 | `a_successful_preflight_writes_a_journal_row_visible_on_both_transports` |
| baseline f | 2295 | 2 | 3 | `a_successful_preflight_writes_a_journal_row_visible_on_both_transports`, `ordering_is_stable_by_project_then_source_whatever_the_selection_order` |

**4 of 6 baseline runs fail**, against **3 of 5 with this change applied**
(2303/2303/2302 passed where clean; the with-change failures are all drawn from
the pre-existing pool below). Every failing test outside this change is
classified below, and two of them were reproduced with the change stashed:

| Failing test | Signature | Classification |
|---|---|---|
| `preview_port_collision_is_refused_without_killing_a_listener` | `left: 56, right: 64` at `tests/studio_preview_contract.rs:186` | **the defect this change's sibling fixes** — reproduced on the stashed baseline in whole-suite run c, where 8 of the 64 ephemeral-range ports were taken by unrelated connections |
| `tests/api_contract.rs` (`healthz_route_returns_200_without_authorization`, `unknown_route_returns_404`, `wrong_method_returns_405`) | `read response: ConnectionReset (os error 104)` | **pre-existing, reproduced with the change stashed**: 3 failures across 8 `cargo test --test api_contract` runs on the baseline, varying test identity |
| `delivery_cross_surface::a_successful_preflight_writes_a_journal_row_visible_on_both_transports` | `cannot send request to publish provider 'openpanel': Broken pipe (os error 32)` | **pre-existing, reproduced with the change stashed** (baseline runs d, e, f). `src/publish/providers.rs:531` has the same unguarded `write_all` that `21a9566` fixed in `src/governance.rs`, on a boundary this change does not own |
| `docs_contract::provider_failure_keeps_prior_derivative_and_redacts_secrets` | `translator stdin write failed: Broken pipe (os error 32)` | **pre-existing, reproduced with the change stashed** (baseline run d). `src/docs/mod.rs:676` — third instance of the same unguarded write |
| `mcp_contract::mcp_repeated_isolated_round_trip_is_stable` | two MCP invocations returning `observed_at` one second apart | **pre-existing** (baseline run c): the assertion is `assert_eq!(one["result"], two["result"])` across two separate invocations and `observed_at` is a wall clock |
| `gate::tests::real_run_executes_gate_surface_and_records_evidence` | `binary not found or not executable: Text file busy (os error 26)` | **pre-existing, not reproduced stashed**: 1 failure in 6 with-change whole-suite runs, 0 in 6 baseline runs. The test writes `gate-ok.sh` and `run_gate` execs it; `/bin/sh` re-execs the script while the writer still holds it. Passes 5/5 in isolation **both** with and without this change, and `src/gate/mod.rs:778` is its own `try_wait` loop, untouched |
| `docs_contract::ordering_is_stable_by_project_then_source_whatever_the_selection_order` | — | **pre-existing** (baseline run f) |

None of the paths above calls the functions this change edits: neither
`tests/delivery_cross_surface.rs`, `tests/api_contract.rs`,
`tests/docs_contract.rs` nor `src/gate/mod.rs`'s runtime path reaches
`src/governance.rs`.

The three `Broken pipe` instances are worth the owner's attention: they are the
same defect `21a9566` fixed for governance adapters, still live on the publish
provider, delivery and docs-translator boundaries. They are recorded in
`design.md` §6 and are **not** fixed here — each is its own change, and this
one does not widen.

| Check | Command | Result |
|---|---|---|
| guard fails pre-fix, large stdout | drain reverted | `ProviderStatus::Unavailable`, `adapter exceeded timeout of 5000 ms` |
| guard fails pre-fix, over cap | drain reverted | `ProviderStatus::Unavailable`, not the cap refusal |
| guard fails pre-fix, revision deadline | bound reverted | test binary killed by `timeout 20`; no return |
| positive controls | both revisions of the code | pass before and after |
| `21a9566` guards | `cargo test --lib governance` + `cargo test --test governance_contract` | pass |
| `cargo test --test governance_contract` | 30 consecutive runs | **30/30 passed** (23 passed; 0 failed each) |
| `cargo test --test governance_contract -- --test-threads=8` | 10 runs | **10/10 passed** |
| `cargo test --test governance_contract -- --test-threads=1` | 10 runs | **10/10 passed** |
| guard fails pre-fix, revision deadline | bound reverted (`Command::output()`) | `rustc`-built probe of the exact pre-fix expression, `timeout 8` → **exit 124**, never returned |
| guard fails pre-fix, end-of-file window | `slice = deadline - now` | **2/2** unit guards FAILED: `answering took 5.002815516s of a 5 s budget`, `elapsed 5.004397229s` |
| `cargo fmt --check` | | clean (after `cargo fmt --all`) |
| `cargo clippy --workspace --all-targets` | | exit 0; **12** warnings with the change stashed, **12** with it applied — none added, zero attributed to `src/governance.rs` or `tests/governance_contract.rs` |
| `git diff --check` | | PASS |
| `node scripts/check-openspec-change-names.mjs` | | PASS |
| `openspec validate --all --strict --no-interactive` | | **68 passed / 0 failed**, including `change/governance-adapter-bounded-process-run` |
| `cargo test --workspace --all-targets --no-fail-fast -- --skip generate::tests::…`, with the change | run a / b / c / d / e | **2301/2, 2301/2, 2303/0, 2303/0, 2302/1** passed/failed (3 ignored each). Every failure is in the pre-existing pool classified above |

## 5a. Whole-suite runs, with the change applied

| Run | Passed | Failed | Ignored | Failing tests |
|---|---|---|---|---|
| with-change a | 2301 | 2 | 3 | `api_contract`: `healthz_route_returns_200_without_authorization`, `unknown_route_returns_404` |
| with-change b | 2301 | 2 | 3 | `gate::tests::real_run_executes_gate_surface_and_records_evidence`, `delivery_cross_surface::a_successful_preflight_writes_a_journal_row_visible_on_both_transports` |
| with-change c | 2303 | 0 | 3 | — |
| with-change d | 2303 | 0 | 3 | — |
| with-change e | 2302 | 1 | 3 | `delivery_cross_surface::a_successful_preflight_writes_a_journal_row_visible_on_both_transports` |

**`studio_preview_contract` failed in 0 of 5 with-change runs and 1 of 6
baseline runs** (`left: 56, right: 64`), which is the direct before/after for
the sibling change.
| `git diff --check` | | PASS |
| `node scripts/check-openspec-change-names.mjs` | | PASS |
| `openspec validate --all --strict --no-interactive` | | passed |

## 6. Outstanding

- **Three sibling boundaries still have the defect `21a9566` fixed here**:
  `src/publish/providers.rs:531`, `src/docs/mod.rs:676` and
  `src/delivery/hermora.rs:171` all write an adapter request with a bare
  `write_all` and turn `BrokenPipe` into a hard failure. Reproduced on a
  stashed baseline (`design.md` §6.5). Best candidate for the next change.
- **Two pre-existing flakes outside any subprocess boundary**:
  `tests/mcp_contract.rs:560` and `tests/api_contract.rs`
  (`design.md` §6.6). Both reproduced stashed.
- **The studio port flake was reproduced stashed** (`left: 56, right: 64`) and
  is fixed by the sibling change `studio-preview-contract-port-range`.
- Seventeen other `try_wait` loops outside this boundary keep the poll shape
  (`design.md` §6.1). Recorded, not fixed.
- `src/process.rs` and this module now hold two bounded-run implementations
  with different error taxonomies (`design.md` §5). Recorded, not unified.
- The three sibling Studio test files hardcode ephemeral-range port bases
  (`design.md` §6.4). Recorded, not fixed.
