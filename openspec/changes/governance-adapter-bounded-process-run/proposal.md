# Proposal: the governance adapter subprocess has no bounded, non-deadlocking run

## Why

`run_adapter` (`src/governance.rs:588-671`) is the single place where Forge
executes an **externally owned** governance adapter. It has three separate
defects that the change landed in `21a9566` recorded as outstanding in
`design.md` §5 and that nobody has fixed. All three are reachable today from
the command line.

### 1. A large stdout deadlocks until the deadline

| Constant | Value | Source |
|---|---|---|
| `MAX_ADAPTER_OUTPUT_BYTES` | 256 KiB | `src/governance.rs:40` |
| Linux default pipe buffer | 64 KiB | kernel default, `pipe-user-pages-soft` bounded |

`run_adapter` reads the child's stdout **only after** `try_wait` reports the
child exited (`src/governance.rs:623-633`). A child that writes more than one
pipe buffer therefore blocks in `write(2)` forever, so it never exits, so
Forge never reads. The two ends are each waiting on the other. The only thing
that resolves it is the deadline, which reports a *timeout* for an adapter
that answered perfectly:

```
status=Unavailable detail=Some("adapter exceeded timeout of 5000 ms")
```

The allowance and the mechanism contradict each other: Forge promises to
accept 256 KiB of stdout but cannot accept 65 KiB.

### 2. `git_revision` has no deadline at all

`git_revision` (`src/governance.rs:817-826`) runs `git rev-parse HEAD` through
`Command::output()`, which blocks until the child exits with no timeout. It is
called from `evaluate_project` (`src/governance.rs:399`) on **every**
`forge governance check`, for the local provider as well as the external one.
A `git` that blocks — a stale network filesystem, a repository on a stalled
mount, a lock held by another process — hangs the check forever, and no
deadline anywhere in the adapter boundary can interrupt it.

### 3. The wait loop polls at a fixed 10 ms

`src/governance.rs:661` sleeps a fixed 10 ms between `try_wait` calls. Every
adapter run costs up to 10 ms of pure latency, and the wake-up is blind: it
happens whether or not the child has produced anything.

### Why this is not fixed by the `21a9566` change

`21a9566` fixed the **request write** and, correctly, scoped the rest out. It
did not regress anything here and this change does not regress it: the
`BrokenPipe`-is-not-a-failure rule and the kill-and-reap on a genuine write
failure are carried through untouched, and both are covered by existing
guards.

## What Changes

- **`run_adapter` drains stdout and stderr concurrently with the wait.** Two
  reader threads own one pipe each and read to end while the parent waits for
  exit, so an adapter writing up to `MAX_ADAPTER_OUTPUT_BYTES` is fully read
  and cannot block. Bytes past the cap are counted but not buffered, so the
  cap stays a real bound on memory while the child still runs to completion.
- **The wait is a blocking wait with a timeout, not a poll loop.** The parent
  blocks on `recv_timeout(remaining_deadline)` and wakes on a pipe reaching
  end-of-file or at the deadline. The fixed 10 ms sleep is gone.
- **`git_revision` respects the same bounded-wait discipline.** It is bounded
  by the selected provider's existing `timeout_ms` — the same constant
  `validate_config` already constrains to `1..=300000`, and the same value
  `run_adapter` already uses — and its output is capped.
- Both paths share one private helper so the bounded run is stated once.

No contract version, schema, persisted shape, CLI flag, environment variable,
provider-selection behaviour or observation field changes. An adapter that
answers within its deadline produces the **same observation it produced
before**; only adapters that today deadlock (and are wrongly reported as
timed out) change, and they now report what they actually answered.

## BFS Impact Map

| Surface | State today | Action |
|---|---|---|
| `run_adapter` wait/read loop (`src/governance.rs:621-671`) | polls `try_wait` every 10 ms; reads both pipes only after exit; a >64 KiB stdout deadlocks until the deadline | replaced by one bounded concurrent run |
| `MAX_ADAPTER_OUTPUT_BYTES` (`src/governance.rs:40`) | 256 KiB, checked only after the child exited | unchanged value; now enforced by the drain thread while the child runs |
| `write_adapter_request` + its kill-and-reap (`src/governance.rs:612-619`, `682-690`) | `BrokenPipe` tolerated, other errors kill + reap first | **unchanged** — the `21a9566` contract is preserved |
| timeout path (`src/governance.rs:652-659`) | kills, reaps, returns synthetic failure with `adapter exceeded timeout of {timeout_ms} ms` | **unchanged shape** — same status, same message |
| stdout cap path (`src/governance.rs:641-645`) | `GovernanceInvalid` "adapter stdout exceeds 262144 bytes" | **unchanged shape**; now reached by draining instead of by the deadline |
| `git_revision` (`src/governance.rs:817-826`) | `Command::output()`, no bound, unbounded stdout capture | bounded run against the provider's `timeout_ms`, capped output |
| `evaluate_project` (`src/governance.rs:399`) | `git_revision(project_root)` | `git_revision(project_root, provider.timeout_ms)` |
| `src/process.rs::spawn_with_timeout` | the shared helper other adapters use; reader threads, 50 ms poll, no stdin write, `String` errors | **not touched** — see `design.md` §5 for why it is not reused |
| `src/governance.rs` in-module `mod tests` | covers the request-write boundary only | add the bounded-run and revision-deadline guards |
| `tests/governance_contract.rs` | no adapter in the target writes more than a few hundred bytes | add the large-stdout and over-cap end-to-end guards |
| `governance-adapter-request-write-race` | active, implemented, unarchived, `21a9566` | **not archived, not edited**; its two guards must still pass |
| `contracts/**`, `scripts/contract-parity.sh`, `src/portfolio/share/**`, `tests/portfolio_share_*`, `tests/manifest_wire_contract.rs` | owned elsewhere | **not touched** |
| `tests/studio_preview_contract.rs` | separate pre-existing flake | owned by `studio-preview-contract-port-range` |

### Callers of the changed functions

| Caller | Reachability |
|---|---|
| `evaluate_project` | `forge governance check` (CLI), `check_project` (persisting), `inspect` (read-only), and the API/portal governance surfaces |
| `run_external_provider` | the only `run_adapter` caller; the only caller of the timeout and cap paths |

No other module calls `run_adapter` or `git_revision`; both are private to
`src/governance.rs`.

### Concurrency review

The new threads are strictly bounded: one per pipe, each terminating at
end-of-file or at the deadline enforced by the parent. The parent never
returns while the child is alive — every exit path calls `kill` **and**
`wait`. If the child exits but a descendant inherited a pipe, the parent's
post-exit drain wait is bounded by the same deadline and the observation
carries an explicit truncated-drain note instead of hanging; that state is
unreachable for every adapter in this repository and is reported rather than
assumed away.

## Capabilities

- `governance-provider-contract` — the adapter subprocess run is bounded,
  cannot deadlock against a pipe, and the revision lookup obeys the same
  bound.

## Non-goals

- **Reusing `src/process.rs::spawn_with_timeout`.** It cannot express the
  request write with its `BrokenPipe` tolerance, cannot express the
  256 KiB `GovernanceInvalid` cap, and returns `String` errors where
  `run_adapter` has a typed taxonomy. Reasoned in `design.md` §5.
- **Changing `MAX_ADAPTER_OUTPUT_BYTES` or `DEFAULT_TIMEOUT_MS`.** The 256 KiB
  allowance is the contract; the defect is that the mechanism could not
  deliver it.
- **Archiving `governance-adapter-request-write-race`** or either other
  in-flight change.
- **Unifying every `try_wait` loop in the tree.** Seventeen other call sites
  have the same shape; each has its own semantics and this change does not
  own them. Recorded as remaining work in `design.md` §6.
- **`tests/studio_preview_contract.rs::preview_port_collision_is_refused_without_killing_a_listener`**,
  a separate pre-existing flake owned by `studio-preview-contract-port-range`.
- Any retry, sleep, `#[ignore]` or weakened assertion.
