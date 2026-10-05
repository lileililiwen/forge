# Design: runtime-hardening-and-contract-closure

This package specifies code that is already on `main`. It records three things the
seven source packages could not record between them: where each merged requirement
lives and why, the measured evidence behind it, and the boundaries that remain open.

## 1. Why one package rather than seven archives

`.ai-rules/workflow.md` sequences implement → verify → strict validate → **archive** →
commit → advance pointer. Seven packages completed everything but the archive, each
citing the others. Independently, archiving them separately would have promoted
duplicated requirements, because three of the seven share code:

| File | Edited by |
|---|---|
| `src/governance.rs` | `21a9566`, `5d5f103`, `e3a156b` |
| `tests/governance_contract.rs` | `21a9566`, `5d5f103` |
| `tests/studio_preview_contract.rs` | `5d5f103`, then rewritten by `f078a4c` |

`f078a4c` deleted ~100 lines `5d5f103` had written into
`tests/studio_preview_contract.rs` and moved them into
`tests/support/studio_ports.rs`, so the earlier package's delta described a per-test
implementation that no longer exists. A merge before promotion is the only way the
canonical specs can state what the tree does.

## 2. Requirement placement after the merge

**Request write → `governance-provider-contract` only.** Two deltas stated it:
`governance-adapter-request-write-race` in `governance-provider-contract`, and
`adapter-request-write-boundary` in `runtime-hardening-and-test-isolation`, with
three of four scenarios renamed rather than new. The merged requirement keeps the
wider scope (every external adapter, translator and delivery provider, one shared
boundary, no error discarded at a call site) and the original home, because the
observable promise is about what a *provider* reports: a lost write race must not
become `unavailable`, `PublishInvalid` or a swallowed delivery failure.
`runtime-hardening-and-test-isolation` therefore gains only the port requirement. Its
existing "Bounded release subprocess cleanup" already covers the process-runtime
angle, so nothing is left unsaid by dropping the second copy.

**Studio test ports → `runtime-hardening-and-test-isolation` only, one requirement.**
`studio-preview-contract-port-range` and `studio-test-port-range` both added a
requirement to that spec with identical first, second and third scenarios and bodies
that differ only in whether the window is one test's or shared. Merged: run-time
selection outside the host ephemeral window, every port bindable when chosen,
assertions computed from the configured base, listeners held by the test that needs
its range occupied, and a per-target candidate index.

The allocator invariant itself — refusing a busy range must not kill a listener —
stays in `site-studio-preview-refinement`, where it describes product behavior rather
than test hygiene.

## 3. Mechanisms as shipped

### 3.1 Request write (`src/process.rs:169`)

`pub fn write_request(stdin: &mut impl Write, request: &[u8]) -> std::io::Result<()>`

`BrokenPipe` returns `Ok(())`: it can only mean the child closed its input, so the
caller proceeds to that child's real exit status and output. Every other error is
returned for the caller to map to its own typed refusal after kill and reap.

| Call site | Pre-fix | Post-fix |
|---|---|---|
| `src/publish/providers.rs:529` | bare `write_all` → `PublishInvalid`, child left running | shared rule, kill + reap |
| `src/docs/mod.rs:676` | bare `write_all` → translator error | shared rule, existing kill + reap kept |
| `src/delivery/hermora.rs:173` | `let _ = stdin.write_all(&payload)` — every error discarded | shared rule, kill + reap, `DeliveryUnavailable` |
| `src/governance.rs:902` | already correct (`21a9566`) | delegates to the shared rule |

Hermora was the instructive one: its comment already stated the rule and the code
under it threw the error away, so a genuine `EIO`/`ENOMEM` was swallowed and the
adapter's exit status was reported for a request that never arrived. Four copies is
how that was written; the helper takes `&mut impl Write` rather than `&mut Child`
specifically so the non-`BrokenPipe` arm is unit-testable with a `Write` impl that
always fails `PermissionDenied`. That guard catches a reintroduced `let _ =`.

### 3.2 Bounded adapter run (`src/governance.rs:601`, `:728`, `:1040`)

1. `MAX_ADAPTER_OUTPUT_BYTES` is 256 KiB against a 64 KiB pipe buffer, and the old
   code read a pipe only after `try_wait` reported the child gone. Any adapter
   writing more than one buffer blocked in `write(2)` and was reported
   `Unavailable: adapter exceeded timeout of 5000 ms`. Arithmetic, not a race. Both
   pipes are now drained on their own threads while the parent waits; bytes past the
   cap are counted and discarded, so the cap stays a real memory bound.
2. `git_revision` used `Command::output()`, which blocks until the child exits, on
   every check including the local provider. It is now bounded by the selected
   provider's existing `timeout_ms` — the constant `validate_config` already
   constrains — so no new number enters the boundary. Against the pre-fix expression
   the guard never returned: `timeout 8` → exit **124**.
3. The wait polled every 10 ms; it is now a blocking `recv_timeout` that wakes on
   either pipe reaching end-of-file or at the deadline.

Two measured hazards that shaped the implementation:

- `/bin/sh -c '…'` forks on this host, so killing a child does not close a pipe a
  descendant inherited (the pipe stayed open the full 30 s). The drain threads are
  therefore **detached, not joined** — joining one took the full 30 s, an unbounded
  wait in exactly the case the bound exists for.
- End-of-file is not an exit event. A waiter that woke only on end-of-file reported
  `exceeded timeout` for adapters that had answered completely: **2–4 failures per
  run**, at exact multiples of the 5 s and 10 s test budgets; 0 failures and 0.02 s
  after the fix. The one remaining short re-check — both pipes done, child not yet
  reaped — has its own deterministic guard.

`src/process.rs::spawn_with_timeout` was deliberately not reused: it cannot express
the request write with its `BrokenPipe` tolerance, cannot express the 256 KiB
`GovernanceInvalid` cap, and returns `String` errors where `run_adapter` has a typed
taxonomy. Two bounded-run implementations with different error taxonomies now coexist.
That is an open decision, not an oversight (§5.2).

### 3.3 Studio test port windows (`tests/support/studio_ports.rs`)

The host hands out outbound source ports from
`/proc/sys/net/ipv4/ip_local_port_range` = `32768 60999` without regard for who is
already listening inbound. Four Studio targets hardcoded bases inside it:
`45800` (preview, proved first, `left: 56, right: 64`), `47100`
(`studio port unavailable: no free port in range 47100..=47163`), `47300`, `48200`.

One shared selection now serves all four. `CANDIDATE_STEP` is 128 and each target
starts at its own index, preferring `4100`, `4228`, `4356` or `4484` — rotated, not
filtered, so every candidate stays reachable. The rotation was itself a fix to a
defect this work introduced: with one shared preference list all four preferred
`4100`, and a `react_web_native_preview` run overlapping a Studio run produced
`studio-start-timeout: profile runner exited before binding the reserved port (port
4100)`. `cargo test` runs test binaries one at a time; two `cargo test` invocations
do not. Measured with `ss -ltn` while three targets ran: `4228` and `4356` in use at
once, from two different targets.

The collision test keeps the listeners that verified its window was free, so choice
and occupancy cannot race. Neither half of that assertion can prove *ownership* —
nothing observable from outside a process can — and the record says so.

### 3.4 Contract parity (`scripts/contract-parity.sh`)

The previous version performed zero comparisons and printed an unconditional
`contract-parity: OK`. It now derives the compared set from what the mirror retains
and what the source's own `manifest.json` publishes, takes the digest authority from
the source rather than from Forge's self-referential manifest, counts its
comparisons, and exits non-zero on a mismatch, on zero comparisons, on an
unresolvable source, and on a source resolving to the mirror itself.
`PLATFORM_CONTRACTS_DIR` locates the source and has no other effect; there is no mode
that softens a comparison.

### 3.5 Manifest wire shape (`src/portfolio/share/manifest.rs`)

Forge is the producer; a separate static Hugo site is the consumer. Three emitted
fields were rejected by
`platform-contracts/schemas/public-portfolio-manifest.schema.json`
(`platform.public-portfolio-manifest/1.0.0`, digest
`07a3c47769da8923998273cda602ddffb195f983e3dcf344fb1d86540c6bc986`):

| Field | Was | Now |
|---|---|---|
| `schema_family` | `"public-portfolio-manifest"` | `"platform.public-portfolio-manifest"` |
| `schema_version` | JSON number `1` | string `"1.0.0"` |
| `manifest_revision` | JSON number (`u32`) | string `"rev_<revision>"` |

Only the serialized field is a string, produced by `wire_manifest_revision`, so no
call site can invent a second spelling. Storage is unchanged: the
`manifest_revision INTEGER` column, the `i64` approval, audit, publication report and
adapter envelope, and `build_manifest(records, u32)`. The encoding is pure and
injective, so an unchanged catalog still hashes identically.

## 4. Compatibility decision worth naming

An approval made before §3.5 is bound to the old hash, so `publish` refuses it until
the operator previews and approves again. No stored approval or audit entry was
rewritten — that would forge an approval nobody gave. There is no migration and no
back-fill; the refusal names both hashes and the remedy is re-approval.

## 5. Open boundaries

### 5.1 The deadlock shape at the three non-governance boundaries

| Site | stdout drained while the child runs? | cap | a child writing > 64 KiB to stdout |
|---|---|---|---|
| `src/publish/providers.rs:589-607` | no (stderr is, on its own thread, `:546`) | none | blocks in `write(2)`, killed at `provider_timeout()`, reported `timed out` |
| `src/docs/mod.rs:684-693` | no | none | blocked, killed at the translator timeout |
| `src/delivery/hermora.rs:180-200` | no (stderr is `/dev/null`) | `RESPONSE_BYTES_MAX` applies after the wait, bounding the retained response, not the read | blocked, killed at `adapter_timeout()` |

A misclassification, not a hang: each loop returns from an existing deadline, so the
verdict is wrong ("timed out") rather than absent. Closing it means re-deriving the
bounded-run machinery three more times, including the detached-thread fork hazard in
§3.2. No test here produces more than ~10 KiB on these channels, so no failure is
attributable to it and no scenario is written for it.

`governance-adapter-bounded-process-run` recorded these three as outstanding and
unfixed; `adapter-request-write-boundary` fixed their request writes and left their
reads. Both statements were true of their own scope and are contradictory across
packages, which is a cost of the split this package removes.

### 5.2 Two bounded-run implementations

`process::spawn_with_timeout` and `governance::run_adapter` both bound a child and
neither is derivable from the other. A third party — the seventeen remaining
`try_wait` loops — matches neither. Unification needs a shared bounded run that carries
a typed error taxonomy and an optional request write.

### 5.3 The allocator's own bind/drop window

A test verifies a window is free, then its runner binds it. Another process can take
a port in between. Slots are a preference, not a reservation. A failure there is
Forge's product defect, not a test port choice, and it is recorded as such rather
than papered over by a retry.

### 5.4 Three further contract mismatches, reachable today

Reproduced while mapping the manifest document and deliberately not fixed here,
because each needs a decision about whether Forge narrows its vocabulary or the
contract widens it — a product decision, not a mechanical one.

| Field | Contract | Forge | Symptom |
|---|---|---|---|
| `visibility` | enum `["public"]` | `Visibility::ALL` admits `unlisted` | `'unlisted' is not one of ['public']` |
| `status_evidence` | `additionalProperties: false`, allows `observed_at`, `source`, `note` | `EVIDENCE_KEYS` admits `source_system`, `source_revision`, `state` | `Additional properties are not allowed ('source_system' was unexpected)` |
| `id` | maxLength 64 | `validate_project_id` pins the pattern, no length bound | `projects/0/id: … is too long` |

### 5.5 `Text file busy (os error 26)`

A harness race, reproducible on demand, at four fixture-staging sites
(`src/portfolio/share/publish.rs:444`, `src/policy/mod.rs`, `src/gate/mod.rs:1072`,
`tests/inventory_contract.rs:281`). `fs::write` holds an `O_WRONLY` descriptor across
open/write/close and Rust spawns by `fork` + `execvp`, so a child forked inside that
window inherits the descriptor and its exec of that same file is refused. `O_CLOEXEC`
does not help: the kernel checks `ETXTBSY` while opening the executable, before
closing close-on-exec descriptors. Confirmed by trace — `2445600 execve(…dw-junk.sh) =
-1 ETXTBSY`, with `2445597`–`2445603` other tests' spawns interleaved. Explains 0
failures in 200 isolated runs, a different victim each time, and the leaked
`/tmp/forge-share-timeout-*` directories (the panic skips `remove_dir_all`). No fix
fits one change: it needs a lock around every spawn in a 1180-test binary plus every
integration binary, and one of the sites is product code. Root cause and options are
recorded in `HANDOFF.md`.

### 5.6 One unexplained failure

`governance-adapter-request-write-race` recorded 1 failure in 245 default-parallelism
runs of its target where the capture never said which test or why. Not reproduced in
20 runs at `--test-threads=8`, 20 at `--test-threads=1`, or 220 further default runs.
Not claimed fixed.

## 6. Verification

Every number below came from a run, not from reasoning. Where a guard is cited, it was
checked against the pre-fix mechanism by reverting the fix, not by trusting the fix.

| Guard | Against the pre-fix mechanism |
|---|---|
| `an_adapter_that_writes_more_than_one_pipe_buffer_still_answers` | **fails**: `left: Unavailable, right: Pass` |
| `an_adapter_that_writes_past_the_output_cap_is_refused` | **fails**: `Unavailable, "adapter exceeded timeout of 5000 ms"` |
| `a_revision_lookup_that_never_answers_gives_up_within_its_bound` | pre-fix `Command::output()` rebuilt and run, **never returned**: `timeout 8` → exit **124** |
| `a_child_that_closed_its_pipes_and_keeps_running_is_not_reported_as_a_timeout` | **fails**: `elapsed 5.004397229s` of a 5 s budget |
| `a_request_write_to_a_child_that_already_exited_is_not_a_failure` | **fails 1/1**: `Os { code: 32, kind: BrokenPipe }` |
| `a_request_write_failure_that_is_not_a_broken_pipe_is_returned` | passes both ways by design — pins the *retained* refusal |
| Studio liveness loop, one listener removed | **fails**: `listener on 4100 no longer accepts: Connection refused (os error 111)` |
| held-range negative control (pre-fix bases restored, 256 ports held) | `no free port in range 47100..=47163` and `47300..=47363` |

Aggregate runs recorded by the source packages, 2026-10-05: 400 consecutive runs of
the eight affected targets clean (240 default, 80 at `--test-threads=8`, 80 at `=1`);
`governance_contract` 244 of 245 (§5.6); `studio_preview_contract` 30/30 and 10/10 at
`--test-threads=8`; whole suite 6 runs with the changes and 6 with them stashed —
3 of 6 clean applied against 1 of 6 clean stashed, and four of the five stashed
failures are the two `BrokenPipe` tests this work fixes.
`FORGE_NATIVE_REACT_WEB_PREVIEW=1` ran the live path six times, six passed, each a
real `npm install`, a real Vite dev server on the run-time base and a Playwright
render.

Manifest acceptance used the consumer's own oracle: a real
`portfolio share set → preview → approve → publish` cycle, then
`python3 lileililiwen.github.io/scripts/validate_manifest.py --strict` against the
sibling schema → `manifest OK … (schema 1.0.0, 1 project(s), revision rev_1)`, exit 0.

Re-runs for this archive are recorded in `tasks.md` §4, including the check that was
the queue's original blocker.

## 7. Recovery

Nothing here is speculative history: the seven source packages are preserved in git
from `c138f08` through `1c3e1f5`, including each one's own `design.md` with the full
derivation of every measured claim. This package supersedes them as the active record,
not as their only copy.
