# Design: adapter-request-write-boundary

## 1. The defect, precisely

Four Forge surfaces hand a JSON request to a child process on its standard
input and then wait for it. All four did the write differently, and three of
the four were wrong in a different way from the other three.

| Site | Pre-fix code | Pre-fix outcome on `BrokenPipe` | Child reaped on a genuine write error? |
|---|---|---|---|
| `src/governance.rs:610` (`run_adapter`) | `Err(BrokenPipe) => Ok(())` — fixed by `21a9566` | fall through to the answer | yes, since `21a9566` |
| `src/publish/providers.rs:528` | `stdin.write_all(&input).map_err(PublishInvalid)?` | `PublishInvalid: cannot send request to publish provider 'openpanel': Broken pipe (os error 32)` | **no** — returned while the provider was still running |
| `src/docs/mod.rs:675` | `Err(err) => translator stdin write failed` | `translator stdin write failed: Broken pipe (os error 32)` | yes |
| `src/delivery/hermora.rs:170` | `let _ = stdin.write_all(&payload);` | falls through — *by accident* | n/a: the error is discarded, so the code waits for the child anyway |

The mechanism is the one `21a9566` measured, unchanged:

* a stub adapter that is `#!/bin/sh` + `printf` + `exit` **never reads stdin**;
* as it exits it closes the read end of the pipe;
* Forge writes immediately after `spawn()` returns, so which of the two
  happens first is pure scheduling, widened by whatever else the machine is
  doing;
* Rust's runtime installs `SIGPIGE = SIG_IGN` (Rust installs
  `SIGPIPE = SIG_IGN`), so this is an `errno` — `EPIPE`, `os error 32` — never
  a signal.

The request is ~150 bytes and a pipe buffer is 64 KiB, so the write cannot
legitimately block: an `EPIPE` here is only ever "the peer already left".

Hermora's row is the interesting one. It is the only site whose *pre-fix
behaviour* was the behaviour we want, and it got there by discarding **all**
errors. That is a latent silent-failure bug, not a flake: an `EIO` while
writing a real request is dropped, and Forge then reports the adapter's own
exit status for an adapter that never received the request. Its comment
already said the right thing; only the code was wrong.

## 2. Ownership

All four sites are product code, so all four are fixed in product code. The
test files are *reporters* of this defect class, not its cause: no test is
ignored, annotated, serialised or made conditional. The end-to-end evidence
already exists and is left exactly as it is:

| Target | Test | Stub |
|---|---|---|
| `delivery_cross_surface` | `a_successful_preflight_writes_a_journal_row_visible_on_both_transports` | `install_provider_stub`: `#!/bin/sh` + `printf`, never reads stdin |
| `delivery_contract` | the `hermora-retry` cases | `install_hermora`: `#!/bin/sh` + `printf`, never reads stdin |
| `docs_contract` | `provider_failure_keeps_prior_derivative_and_redacts_secrets` | translator stub, never reads stdin |

## 3. The change

### 3.1 One shared rule, in `src/process.rs`

```rust
pub fn write_request(stdin: &mut impl Write, request: &[u8]) -> std::io::Result<()> {
    match stdin.write_all(request) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::BrokenPipe => Ok(()),
        Err(err) => Err(err),
    }
}
```

`src/process.rs` is already the module this repository documents as the shared
child-process boundary ("shared by every adapter and workflow that delegates
to an external executable"), and it already owns the stderr cap and the
bounded spawn. The rule belongs there rather than being written a fourth
time.

**Why a shared helper rather than a fourth copy.** The decision being made —
"`BrokenPipe` reports on the child, so it is not Forge's failure" — is a
*policy*, not an implementation detail of one module. Four copies is how
Hermora's `let _ =` got written in the first place: three sites spelled the
rule and the fourth had nobody to copy it from. One function also gives the
rule a single place to be unit-tested, which is what makes the
not-a-broken-pipe guard possible at all.

**What the helper deliberately does *not* own.** It takes a `&mut impl Write`,
not a `&mut Child`. That keeps it unit-testable with a `Write` impl that always
fails (`PermissionDenied`), which a `&mut Child` signature could not be, and it
keeps each call site's typed error and child lifecycle visible where they
already were. The kill-and-reap obligation stays at the four call sites, which
is where it already lives in the landed governance fix.

### 3.2 The four call sites

`src/governance.rs` — the mapper keeps its name, doc comment, typed error and
both guards; only its body delegates:

```rust
fn write_adapter_request(stdin: &mut impl Write, request: &[u8]) -> Result<(), ForgeError> {
    crate::process::write_request(stdin, request).map_err(|err| ForgeError::GovernanceUnavailable {
        reason: format!("cannot write adapter request: {err}"),
    })
}
```

`src/publish/providers.rs:529` — the shared rule, then the refusal that used to
be skipped by `?`:

```rust
if let Some(mut stdin) = child.stdin.take() {
    if let Err(error) = crate::process::write_request(&mut stdin, &input) {
        let _ = child.kill();
        let _ = child.wait();
        return Err(ForgeError::PublishInvalid { reason: format!(...) });
    }
}
```

`src/docs/mod.rs:676` — same shape, with the kill-and-reap and message it
already had.

`src/delivery/hermora.rs:173` — `let _ =` becomes a refusal, and the comment
above it becomes true:

```rust
if let Err(err) = crate::process::write_request(&mut stdin, &payload) {
    let _ = child.kill();
    let _ = child.wait();
    return Err(ForgeError::DeliveryUnavailable { reason: format!(...) });
}
```

Every site keeps the refusal text it already used, except Hermora, which had
none and now has one naming the adapter and the retryable-intent wording its
sibling refusals use.

### 3.3 Child lifecycle

| Site | Pre-fix, genuine write error | Post-fix |
|---|---|---|
| providers | child left running (leak) | killed and reaped |
| docs | killed and reaped | unchanged |
| hermora | error discarded, child waited for anyway | killed and reaped |
| governance | killed and reaped (landed) | unchanged |

The `BrokenPipe` path never kills at any of the four: that child is the one
whose answer is wanted.

## 4. Compatibility

- **A child that reads its request** takes the `Ok` path exactly as before.
  Byte-identical behaviour at all four sites.
- **A child that ignores stdin** previously had a scheduling-dependent chance
  of a hard failure; it now always reports its real answer. Strictly more
  information.
- **A partial write followed by `EPIPE`.** `write_all` can report an error
  after delivering bytes. Under the new rule Forge proceeds and a child that
  was mid-read sees a truncated request and answers as it sees fit. That is
  honest — it really was given a truncated request — and no child in this
  repository can reach that state, because every request is well under 150
  bytes and a pipe buffer is 64 KiB.
- No wire contract, persisted shape, CLI flag or environment variable changes.

## 5. The pipe-buffer deadlock shape — checked at all three new sites

`5d5f103` fixed one instance of this in `governance.rs`: stdout was read only
after `try_wait` reported the child gone, while the allowance
(`MAX_ADAPTER_OUTPUT_BYTES` = 256 KiB) is four times a 64 KiB pipe buffer, so a
child writing more than one buffer blocked in `write(2)` on every run.

**The shape is present at all three new sites, and worse than in governance:
there is no stdout cap at all.**

| Site | stdout drained while the child runs? | cap on stdout | consequence for a child writing > 64 KiB |
|---|---|---|---|
| `src/publish/providers.rs:589-607` | no — `wait_with_output()` after `try_wait` reports exit (stderr *is* drained on its own thread, `:546`) | none; the response is parsed with `serde_json::from_slice` | child blocks in `write(2)`, never exits, is killed at `provider_timeout()` and reported `publish provider '<id>' timed out` |
| `src/docs/mod.rs:684-693` | no — `read_to_end` after `try_wait` reports exit | none | blocked, killed at the translator timeout, reported `translator exceeded timeout of N seconds` |
| `src/delivery/hermora.rs:180-200` | no — `wait_with_output()` after `try_wait` reports exit (stderr is `/dev/null`) | `RESPONSE_BYTES_MAX` = 16 KiB, applied in `parse_response` **after** the process has been waited for, so it bounds the retained response, not the read | blocked, killed at `adapter_timeout()`, reported as exceeding its budget |

**It is a misclassification, not a hang.** All three loops are bounded: each
has an existing deadline it reaches and returns from. The observable failure is
a timeout verdict about a child that was answering perfectly well, which is the
same wrong verdict `5d5f103` removed from governance — just reachable with a
smaller payload.

**Why it is not fixed here.** Closing it means draining both pipes
concurrently at three more sites, which is exactly the bounded-run
implementation the previous worker refused to unify on scope, and it carries the
hazard that implementation already measured on this host: `/bin/sh -c '…'`
forks, killing a child does not close a pipe a descendant inherited (the pipe
stayed open the full 30 s), so the drain threads must be **detached, not
joined** — a join there took the full 30 s, an unbounded wait in the precise
case the bound exists for. Re-deriving that machinery three more times, with
its own thread-lifecycle risk, would be a second change with its own evidence,
not a fourth edit to this one.

**What is not true, stated plainly.** No test in this repository produces more
than ~10 KiB on any of these three channels, no whole-suite failure has ever
been attributed to this shape, and I have not measured these three sites
deadlocking. It is a latent defect found by reading the code, classified on the
strength of `5d5f103`'s measurements, and it is left for the change that
unifies the bounded runners.

## 6. Verification strategy

| Layer | Instrument | Determinism |
|---|---|---|
| Boundary (unit) | `a_request_write_to_a_child_that_already_exited_is_not_a_failure` — spawns `/bin/sh -c 'exec 0<&-; exit 0'`, **reaps it**, then writes | **deterministic**: the read end is provably closed before the write |
| Boundary (unit) | `a_request_write_failure_that_is_not_a_broken_pipe_is_returned` — a `Write` impl that always fails `PermissionDenied` | deterministic: no timing, and it is what catches a reintroduced `let _ =` |
| Regression (landed, kept) | `governance::tests::a_request_write_to_an_adapter_that_already_exited_is_not_a_failure`, `…not_a_broken_pipe_still_refuses` | unchanged and still passing |
| End to end | `delivery_cross_surface`, `delivery_contract`, `docs_contract` through their existing non-reading stubs | the reported flakes themselves: 30 consecutive runs each |
| Whole suite | 6 runs, every run's exact totals | catches a site I did not notice |
