# Design: governance-adapter-bounded-process-run

## 1. The defect, precisely

`run_adapter` (`src/governance.rs:588-671`) performs: spawn → write the request
→ poll `try_wait` → on exit, read both pipes to end → enforce the cap.

The ordering is the defect. Reading a pipe only after the writer has exited
inverts the data flow the pipe exists for. A pipe is a bounded buffer, not a
queue:

```
child                                        parent (today)
  |                                              |
  |-- write 64 KiB ------> [buffer full]          |  try_wait -> None
  |   write 64 KiB+... -> BLOCKED                 |  sleep 10 ms
  |                                              |  try_wait -> None
  |                                              |  ... until deadline
  X never exits                                  X kills it, "exceeded timeout"
```

Nothing about this depends on timing. It is arithmetic: 64 KiB of pipe against
a 256 KiB allowance. Any adapter that writes 64 KiB + 1 byte hits it every
single time.

## 2. Ownership

`src/governance.rs`, for the same reason `21a9566` used it: the test file is a
reporter of this defect, not its cause. No test is ignored, serialised, slept
or retried.

## 3. The change

### 3.1 One bounded run, shared by both call sites

```rust
/// A drained pipe: the first `cap` bytes, the total the child produced, and
/// any read error.
struct PipeDrain { bytes: Vec<u8>, total: usize, error: Option<String> }

/// Spawn `child`'s drain threads, then wait for it to exit bounded by
/// `deadline`. Never returns while the child is alive.
fn run_bounded(child: &mut Child, deadline: Duration, stdout_cap: usize, stderr_cap: usize)
    -> Result<BoundedRun, ForgeError>
```

`BoundedRun` carries `status`, both `PipeDrain`s, and `timed_out`. The two
call sites keep their own error taxonomy on top of it; the helper only
decides *when Forge stops waiting*.

### 3.2 Drain to end, buffer to the cap

```rust
loop {
    match stream.read(&mut chunk) {
        Ok(0) => break,
        Ok(n) => { total += n; if bytes.len() < cap { bytes.extend(..) } }
        Err(Interrupted) => continue,
        Err(err) => { error = Some(err); break; }
    }
}
```

Bytes past the cap are **counted and discarded, not buffered and not
unread**. That distinction is the whole fix: discarding keeps the pipe
draining, so the child still runs to completion and Forge still gets its exit
status, while `total` still reports the true size so the existing cap
refusal is unchanged. Stopping the read at the cap would re-create the
deadlock for an over-cap adapter, just at a different threshold.

### 3.3 Blocking wait with a timeout, not a poll

```rust
loop {
    match child.try_wait() { Ok(Some(status)) => break, Err(err) => { kill+wait; refuse } Ok(None) => {} }
    if Instant::now() >= deadline { kill; wait; return timed_out }
    match events.recv_timeout(deadline - now) { Ok(_) => {}, Err(Timeout) => { kill; wait; return timed_out } ... }
}
```

Each drain thread sends exactly one message when its pipe reaches
end-of-file. The parent holds a spare `Sender` for the whole call, so the
channel cannot disconnect and `recv_timeout` always blocks for real instead of
returning `Disconnected` immediately and spinning. Consequences:

- **Quiet child**: the parent sleeps in one `recv_timeout` for the entire
  remaining budget. Zero wake-ups, and the deadline is honoured to the
  millisecond because the wake-up *is* the deadline.
- **Adapter exits**: both pipes reach end-of-file within microseconds of the
  exit, so the parent wakes on the event, not on a timer. The 10 ms of
  latency is gone rather than reduced.
- **Adapter closes its pipes then keeps running**: end-of-file arrives, the
  parent re-checks `try_wait`, sees `None`, and blocks again until the
  deadline. This is the case a naive "end-of-file means exit" rewrite would
  get wrong; the loop gets it right for free.

`try_wait` is retained deliberately: it is the only std call that reports the
child's exit **without** reaping it, so the parent keeps the `Child` and can
still `kill` + `wait` it on every failure path. A design that moved
`child.wait()` into a waiter thread would have had to kill by pid, which
loses the guarantee that the pid is still ours.

### 3.4 `git_revision` gets the same bound

```rust
fn git_revision(project_root: &Path, timeout_ms: u64) -> Option<String>
```

bounded by the selected provider's existing `timeout_ms` — the value
`validate_config` already constrains to `1..=300000` and the value
`run_adapter` already uses — so no new magic number enters the boundary.
`stdin` is set to `null` explicitly (matching what `Command::output()` did),
and stdout is capped at `MAX_GIT_REVISION_BYTES` (4 KiB, documented: a
`rev-parse HEAD` answer is one 40- or 64-character object name).

Every outcome that is not a successful, non-empty answer stays `None`, which
is exactly what the old `output()` chain produced. `git_revision` therefore
has no new error surface: a hung `git` becomes an absent `source_revision`, a
missing attribute on an observation, rather than a hang.

### 3.4 The one place a short re-check is unavoidable

The first implementation of §3.3 woke **only** on end-of-file. It was wrong,
and the measurement is worth recording because the shape of the bug is not
obvious from the code:

| Run | Failures in `cargo test --test governance_contract` |
|---|---|
| Waiter wakes only on end-of-file | 2–4 tests per run, `finished in 10.10 s` / `20.01 s` — exact multiples of the 5 s and 10 s test budgets |
| Waiter re-checks once both pipes are done | 0, `finished in 0.02 s` |

A pipe reaching end-of-file is **not** the child being reaped. `/bin/sh -c
'…'` forks on this host (measured: `SIGKILL` on the shell leaves `sleep 30`
running with the pipe still open), so both pipes are done microseconds before
the shell itself exits. The waiter woke on end-of-file, re-checked, saw the
shell still alive, and then blocked for the entire remaining budget with no
thread left to wake it — reporting `exceeded timeout` for adapters that had
answered completely.

So the one window with no wake-up source — both pipes at end-of-file, child not
yet reaped — falls back to a 1 ms re-check (`EXIT_RECHECK`). Everywhere else the
receive blocks for the full remaining budget. The alternative designs were
rejected: a waiter thread owning the `Child` would have to kill by pid, losing
the guarantee that the pid is still ours; and a progress event per chunk would
be a poll wearing a disguise.

### 3.5 Reader threads are detached, never joined

The same `/bin/sh -c 'sleep 30'` measurement is why `run_bounded` does not join
its drain threads. A reader's end of the job is to report on its channel, which
is what the bounded receive waits for. Joining it would instead wait for
*every* process holding the pipe to close it, and killing the child does not
close a pipe a descendant inherited. The first implementation joined, and the
deadline guard took the full **30 s** — an unbounded wait in exactly the case
the function exists to bound. A detached reader ends when its pipe does, and a
read that did not finish is reported as `truncated` with a visible marker rather
than waited for.

## 4. Behaviour preserved, behaviour changed

| Situation | Before | After |
|---|---|---|
| adapter answers within its deadline, output < 64 KiB | observation | **identical** observation |
| adapter answers, output 64 KiB – 256 KiB | `Unavailable` "adapter exceeded timeout" | its real answer |
| adapter answers, output > 256 KiB | `Unavailable` "adapter exceeded timeout" | `GovernanceInvalid` "adapter stdout exceeds 262144 bytes" |
| adapter exceeds its deadline | synthetic failure, "adapter exceeded timeout of N ms" | **identical** |
| request write hits `BrokenPipe` | tolerated | **identical** (`21a9566`) |
| genuine request-write failure | child killed + reaped, typed refusal | **identical** (`21a9566`) |
| `git` hangs | check hangs forever | `source_revision` absent, check completes |
| adapter answers in < 10 ms | up to 10 ms of sleep | no sleep |

## 5. Why `src/process.rs::spawn_with_timeout` is not reused

It is the right shape — reader threads, kill-and-reap on timeout, bounded
stderr — and it is the obvious candidate. It cannot be used here without
changing observable behaviour on four counts:

1. **No stdin write.** `run_adapter` must write the request, tolerate
   `BrokenPipe`, and kill + reap on any other write error. `spawn_with_timeout`
   owns the `Child` from spawn, so the write would have to move inside it or
   the `Child` would have to be handed over mid-flight.
2. **No stdout cap with a typed error.** It reads stdout to end unbounded and
   has no `MAX_ADAPTER_OUTPUT_BYTES` rule; this boundary has one and maps the
   breach to `GovernanceInvalid`.
3. **Wrong error type.** It returns `Result<_, String>`. `run_adapter` returns
   `ForgeError` and `run_external_provider` maps `GovernanceUnavailable` to
   `ProviderStatus::Unavailable` — a distinction that has to survive.
4. **Wrong timeout shape.** A timeout is `Err` there and a *successful*
   synthetic failure here, because `run_external_provider` reads the exit
   status and turns a non-zero exit into an `unavailable` observation with the
   adapter's own stderr as detail.

Changing any of those four to fit would alter every other adapter surface
that shares the helper. The blast radius is deliberately not widened.

## 6. Outstanding, deliberately not fixed here

1. **Seventeen other `try_wait` loops** (`src/agent`, `src/release`,
   `src/deploy`, `src/provider`, `src/publish`, `src/gate`, `src/docs`,
   `src/policy`, `src/analytics`, `src/delivery`, `src/portfolio/share`) keep
   the same poll-and-sleep shape at their own intervals. None is on the
   governance adapter boundary, each carries different timeout semantics, and
   this change does not own them. `src/process.rs` is the place to unify them.
2. **A descendant that inherits a pipe keeps it open after the adapter exits.**
   The parent's post-exit drain wait is bounded by the same deadline and the
   result carries an explicit truncated-drain note. Forcing the pipes closed
   needs descriptor surgery (`/proc/self/fd`, `dup2` over the child's ends) and
   is not reachable from std.
3. **The truncation note is a stderr string, not a field.** A dedicated
   `truncated_drain` field on `GovernanceObservation` would be a persisted
   shape change; out of scope here.
4. **`tests/studio_api_contract.rs:193`, `tests/studio_cli_contract.rs:505,549`
   and `tests/react_web_native_preview.rs:153`** hardcode Studio port bases
   (`47100`, `47300`, `48200`) inside this host's ephemeral range. Same
   mechanism as the flake this change's sibling fixes, different files;
   recorded rather than absorbed.
5. **Three more boundaries carry the defect `21a9566` fixed here.**
   Reproduced on a stashed baseline, not inferred: `src/publish/providers.rs:531`
   and `src/docs/mod.rs:676` write an adapter's request with a bare `write_all`
   and map `BrokenPipe` to a hard failure, so a provider that answers without
   reading its request is reported `publish invalid: cannot send request to
   publish provider 'openpanel': Broken pipe (os error 32)` /
   `translator stdin write failed: Broken pipe (os error 32)`.
   `src/delivery/hermora.rs:171` is the same shape. `21a9566` deliberately
   scoped itself to the governance adapter; these three are the same defect on
   three other boundaries, and each is its own change. They are the strongest
   candidate for the next piece of work, because the fix is now written and
   reviewed once.
6. **Two more pre-existing flakes**, unrelated to subprocess boundaries:
   `tests/mcp_contract.rs:560` asserts two separate MCP invocations return
   byte-identical results while `observed_at` is a wall clock, and
   `tests/api_contract.rs` intermittently gets `ConnectionReset` reading a
   loopback response. Both were reproduced with this change stashed.

## 7. Verification strategy

| Guard | Layer | Determinism | Pre-fix outcome |
|---|---|---|---|
| `an_adapter_that_writes_more_than_one_pipe_buffer_still_answers` | end to end (`check_project`) | **deterministic** — arithmetic, not a race: 200 KiB > 64 KiB pipe, and pre-fix nothing reads | `ProviderStatus::Unavailable`, `adapter exceeded timeout of 5000 ms` |
| `an_adapter_that_writes_past_the_output_cap_is_refused` | end to end | **deterministic** — 320 KiB > 256 KiB cap | `ProviderStatus::Unavailable` (deadlock), not the cap refusal |
| `a_revision_lookup_that_never_answers_gives_up_within_its_bound` | unit, in `src/governance.rs` | **deterministic** — `sleep 30` against a 200 ms bound | never returns; killed by an outer `timeout` |
| `a_bounded_revision_lookup_returns_the_object_name_it_printed` | unit | deterministic positive control, **and** the pin for the end-of-file-is-not-an-exit window | `timed_out` true after 5 s of a 5 s budget |
| `a_child_that_closed_its_pipes_and_keeps_running_is_not_reported_as_a_timeout` | unit | **deterministic** — the child closes both pipes itself, so end-of-file provably precedes the 200 ms exit | `timed_out` true after the full 5 s budget |
| `git_revision_still_reports_the_repository_head` | unit, real repo | deterministic | passes |

The first two are the required proof for item 2: a guard that **cannot pass
before the fix** because the pre-fix code physically cannot read more than one
pipe buffer while the child is alive.

The next three are the required proof for item 3, with a positive control so a
bound that always trips cannot masquerade as a fix.

### What the guards pin, stated exactly

| Guard | Reaches | Honest limit |
|---|---|---|
| `an_adapter_that_writes_more_than_one_pipe_buffer_still_answers` | `check_project` → `run_adapter` → `run_bounded` | full end to end |
| `an_adapter_that_writes_past_the_output_cap_is_refused` | same | full end to end |
| `a_revision_lookup_that_never_answers_gives_up_within_its_bound` | `run_bounded`, the mechanism `git_revision` is now built on | **not `git_revision` itself.** Controlling the `git` binary needs a `PATH` mutation, which is process-global and would reintroduce exactly the cross-test interference this change exists to remove. The pre-fix expression was measured separately: a `rustc`-built probe of the exact `Command::output()` call returned **exit 124** under `timeout 8`, i.e. never returned |
| `a_bounded_revision_lookup_returns_the_object_name_it_printed` | `run_bounded` success path | pins that a bound which always trips cannot pass |
| `git_revision_still_reports_the_repository_head` | `git_revision` end to end on a real repository | pins that bounding did not become "always `None`" |
| `a_child_that_closed_its_pipes_and_keeps_running_is_not_reported_as_a_timeout` | `run_bounded` EOF-vs-exit window | pins the §3.4 window deterministically |

Bounding the guards: each runs under an outer `timeout` during verification,
and inside the suite the deadline is the provider's own `timeout_ms`, so the
worst case for a regression is a bounded failure, never a hung suite.
