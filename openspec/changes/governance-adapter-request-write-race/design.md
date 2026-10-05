# Design: governance-adapter-request-write-race

## 1. The defect, precisely

`run_adapter` (`src/governance.rs:588-669`) performs three steps in order:
spawn, write the request, then poll `try_wait` until the child exits, the
deadline passes, or the wait fails.

Step 2 is the only one whose outcome is not determined by the adapter. A child
that reads its request blocks the writer until the bytes are consumed. A child
that **never reads** stdin — `printf` and exit — closes the read end of the
pipe as it exits, and the kernel then answers Forge's `write` with `EPIPE`
(`Broken pipe (os error 32)`; Rust's runtime installs `SIGPIPE = SIG_IGN`, so
this is an `errno`, never a signal).

Whether the write lands first is pure scheduling. The window is the
`execve` → `printf` → `exit` of the child against one `write` syscall in the
parent, widened by whatever else the machine is doing. In the test target that
is 20 tests spawning adapters and `git` processes on 12 cores.

Pre-fix, an `EPIPE` here is `Err(ForgeError::GovernanceUnavailable{..})`, which
`run_external_provider` (`src/governance.rs:475-487`) correctly turns into
`ProviderStatus::Unavailable`. The error mapping was never wrong. The input was.

The child's answer was already complete on stdout at that moment, and the
existing wait/read path three lines later would have returned it.

## 2. Ownership of the fix

The race is in `src/governance.rs`, so that is where it is fixed. The test
file is a *reporter* of this defect, not its cause; no test is ignored,
annotated, serialised or made conditional.

## 3. The change

### 3.1 A named boundary for the request write

The write is extracted into one private function so the decision is stated once
and is directly testable:

```rust
fn write_adapter_request(
    stdin: &mut impl Write,
    request: &[u8],
) -> Result<(), ForgeError>
```

Its contract:

| Adapter behaviour | `write_all` result | Outcome |
|---|---|---|
| read the request | `Ok` | `Ok(())` — unchanged |
| answered without reading; input already closed | `Err(BrokenPipe)` | `Ok(())` — **new**; Forge proceeds to read the answer |
| genuinely unreachable / refused | any other `Err` | `Err(GovernanceUnavailable)` — unchanged |

`BrokenPipe` is the one error that carries information about the *adapter*
rather than about the transport: it can only be raised because the peer closed
its end. Any other `io::ErrorKind` (`PermissionDenied`, `EBADF`, `EIO`, …) says
something about Forge's own plumbing and keeps its typed refusal.

### 3.2 Reap on the genuine-error path

Pre-fix, a non-`BrokenPipe` write error returned while the child was still
running, leaking it. The new path kills and waits it first, matching the
deadline path at `src/governance.rs:650-658` and the
`runtime-hardening-and-test-isolation` requirement ("kill and reap ... before
returning a timeout failure"). The `BrokenPipe` path deliberately does **not**
kill: the child is either gone or is precisely the process whose answer we
intend to read.

### 3.3 What deliberately does not change

- The wait loop, the `MAX_ADAPTER_OUTPUT_BYTES` bound, the deadline, the
  synthetic timeout status, and the stdout/stderr collection are untouched.
- `run_external_provider`'s error mapping is untouched.
- The wire contract, the observation shape, `observed_at`, evidence bounding
  and redaction are untouched. For every adapter answer the resulting
  observation is byte-identical to what the old code produced *when it did not
  lose the race*; the only behaviour that changes is the case where the old
  code discarded a complete answer.

## 4. Compatibility

- **Adapters that read stdin** — including all four recorded fixtures in
  `tests/fixtures/governance-audit/` and the two `req=$(cat)` scripts — take
  the `Ok` path exactly as before. Byte-identical behaviour.
- **Adapters that ignore stdin** previously had a ~50 % chance of being
  reported `Unavailable` with no detail about why. They now report their real
  answer every time. This is the fix, and it is strictly more information.
- **A partial write followed by `EPIPE`.** `write_all` can report an error
  after delivering some bytes. Under the new rule Forge proceeds, and an
  adapter that was mid-read sees a truncated request and answers as it sees
  fit — `Incompatible` in practice. That is not worse than the old outcome
  (`Unavailable`), and it is honest: the adapter really was given a truncated
  request. No adapter in this repository can reach that state, because the
  request is ~150 bytes and a pipe buffer is 64 KiB.
- No migration, no persisted shape, no CLI, no environment variable, no
  flag.

## 5. Outstanding, deliberately not fixed here

Recorded rather than silently absorbed into this change:

1. **stdout is not drained while the child runs.** `MAX_ADAPTER_OUTPUT_BYTES`
   is 256 KiB; a pipe buffer is 64 KiB. An adapter writing more than one
   buffer deadlocks until its deadline and is reported `Unavailable`. Fixing
   it means reading stdout and stderr concurrently with the deadline wait —
   a materially larger change to `run_adapter` that this change does not
   authorise. It is **not** the cause of the flake: no adapter in this
   repository produces more than ~10 KiB.
2. **`git_revision` (`src/governance.rs:796-805`) has no deadline.** A hung
   `git rev-parse` hangs the check forever. Unrelated to the write race.
3. **The wait loop polls at 10 ms** (`src/governance.rs:659`), so a fast
   adapter costs up to 10 ms of latency. Correct, just not tight.

## 6. Verification strategy

The regression guard cannot be an end-to-end timing test: reproducing `EPIPE`
deterministically would require the child to exit *before* the parent writes,
and the parent does not wait. So the guard is layered:

| Layer | Test | Determinism |
|---|---|---|
| Boundary (unit) | `a_request_write_to_an_adapter_that_already_exited_is_not_a_failure` — spawns `/bin/sh -c 'exec 0<&-; exit 0'`, **reaps it**, then writes | **deterministic**: the child is provably gone before the write |
| Boundary (unit) | `a_request_write_failure_that_is_not_a_broken_pipe_still_refuses` — a `Write` impl that always fails `PermissionDenied` | deterministic: no timing involved |
| End to end | `an_adapter_that_never_reads_its_request_still_answers` — real `save_provider_selection` + `check_project`, adapter closes its own stdin then answers | **not** a deterministic pre-fix detector — see below |

Measured against the **pre-fix** behaviour (the `BrokenPipe` arm removed):

| Guard | Pre-fix result |
|---|---|
| `a_request_write_to_an_adapter_that_already_exited_is_not_a_failure` | **failed 1/1**, with `GovernanceUnavailable { reason: "cannot write adapter request: Broken pipe (os error 32)" }` |
| `a_request_write_failure_that_is_not_a_broken_pipe_still_refuses` | passes both before and after (it pins the *retained* refusal, so a future change that swallows every write error is caught) |
| `an_adapter_that_never_reads_its_request_still_answers` | **0/10** whole-target runs detected it |

The end-to-end guard is honestly a weaker instrument, and the reason is
structural: `run_adapter` writes immediately after `spawn()` returns, so the
child needs to win a sub-millisecond race. A harness that raised the odds by
adding CPU pressure or repetition would be exactly the nondeterministic
mechanism this change exists to remove, so it was rejected.

Its value is different and real: it is the **only** place in the suite that
names the scenario as a requirement — an adapter entitled to answer without
reading its request must be reported by what it answered — and after the fix it
is 100 % stable, so it pins the *fixed* behaviour against any future
regression that reintroduces a timing dependence. The deterministic pin on the
violated invariant is the unit guard.
