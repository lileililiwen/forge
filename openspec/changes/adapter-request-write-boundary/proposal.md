# Change: adapter-request-write-boundary

## Why

`21a9566` proved that a `BrokenPipe` on the request write of an external
adapter is not a provider failure: an adapter that answers *without reading
its request* closes the read end of its stdin pipe as it exits, so Forge's own
write can lose a sub-millisecond race and come back `Broken pipe (os error
32)`. It fixed exactly one of four request boundaries — the governance
adapter — and `5d5f103` recorded the other three as the strongest candidate for
the next change.

Whole-suite evidence that the other three are still live, from the same run
that recorded them:

| Target | Signature seen in a whole-suite run |
|---|---|
| `delivery_cross_surface::a_successful_preflight_writes_a_journal_row_visible_on_both_transports` | ``publish provider 'openpanel': Broken pipe (os error 32)`` |
| `docs_contract::provider_failure_keeps_prior_derivative_and_redacts_secrets` | `translator stdin write failed: Broken pipe (os error 32)` |
| `src/delivery/hermora.rs::invoke` | *(no failure recorded — it never fails, which is the defect: see below)* |

All three stubs are `#!/bin/sh` + `printf` + exit, and none of them reads
stdin, so all three are racing the same way.

The fourth, Hermora, has the **opposite** defect. Its comment states the
correct rule and its code does not implement it:

```rust
// A broken pipe here means the adapter exited early; the
// exit-status surface below reports the real cause rather
// than the EOF.
let _ = stdin.write_all(&payload);
```

`let _ =` discards *every* write error, not just `BrokenPipe`. A genuine
failure of Forge's own plumbing (`ENOMEM`, `EIO`, `EBADF`) is silently
swallowed, and Forge goes on to report whatever the adapter exited with — a
success-shaped answer for a request that was never delivered. The comment is
right; the code under it is not.

## What Changes

1. **One shared rule for the request write**, in `src/process.rs`:

   ```rust
   pub fn write_request(stdin: &mut impl Write, request: &[u8]) -> std::io::Result<()>
   ```

   `BrokenPipe` is `Ok(())` — the caller continues to the child's real exit
   status and output. Every other error is returned for the caller to map to
   its own typed refusal. It is the same table `21a9566` wrote for governance,
   now stated once instead of four times.

2. **`src/governance.rs` delegates to it.** `write_adapter_request` keeps its
   name, its doc comment, its typed `GovernanceUnavailable` mapping and both
   of its landed guards; only its body changes (one line) to delegate the rule.
   Nothing about the landed fix is reverted or restructured.

3. **`src/publish/providers.rs:529`** — the bare
   `write_all(...).map_err(PublishInvalid)?` becomes the shared rule plus
   kill-and-reap. Pre-fix it returned while the provider was still running.

4. **`src/docs/mod.rs:676`** — same rule; its existing kill-and-reap and typed
   message are unchanged.

5. **`src/delivery/hermora.rs:173`** — `let _ = stdin.write_all(&payload)`
   becomes the shared rule, and a genuine failure is a typed
   `DeliveryUnavailable` with the child killed and reaped first.

6. **Two deterministic unit guards** for the shared rule in
   `src/process.rs::tests`: a request written to a child that was *reaped*
   before the write (`BrokenPipe` → `Ok`), and a `Write` impl that always
   fails `PermissionDenied` (returned, not swallowed). The second is what
   catches a future change that reintroduces Hermora's `let _ =`.

## BFS Impact Map

| Surface | Finding |
|---|---|
| Requirement | the request-write rule, stated once, applied at every boundary that writes a request to a child |
| Callers | `invoke_provider` (`src/publish/providers.rs`), `run_with_stdin_timeout` (`src/docs/mod.rs`), `invoke` (`src/delivery/hermora.rs`), `run_adapter` (`src/governance.rs`) |
| Contracts | no wire, JSON, registry or persisted shape changes. `PublishProviderResponse`, `TranslatorWireResponse`, `HermoraResponse` and the governance observation are untouched |
| Error types | `PublishInvalid`, `TranslationFailed` (via the existing `String` reason), `DeliveryUnavailable`, `GovernanceUnavailable` — all pre-existing variants, no new variant |
| Child lifecycle | kill + reap added on the genuine-write-failure path at providers (was missing) and hermora (was swallowed); docs already had it; governance already had it |
| Concurrency | none. The change is a `match` on `io::ErrorKind` plus a kill/reap on an error path |
| Persistence / migrations | none |
| Integrations | none. Each adapter's *answer* is read exactly as before |
| Tests | `tests/delivery_cross_surface.rs`, `tests/delivery_contract.rs`, `tests/docs_contract.rs` already exercise the non-reading-adapter path end to end through real `printf`-and-exit stubs; `src/lib` gains two deterministic unit guards |
| Compatibility | for every child that *reads* its request the behaviour is byte-identical. For a child that does not, a hard failure becomes the child's real answer |
| Deadlock shape | checked per site — see `design.md` §5. Present at all three new sites, deadline-bounded, **not fixed here** |

## Capabilities

- `runtime-hardening-and-test-isolation` — one ADDED requirement for the
  shared request-write boundary and its kill-and-reap obligation.

## Non-goals

1. **Unifying the bounded-run implementations.** `src/process.rs` and
   `src/governance.rs` hold two, and `5d5f103` rejected unifying them on
   scope. This change adds one function to `src/process.rs`; it does not
   refactor either existing runner.
2. **Draining child output concurrently.** See `design.md` §5: all three new
   sites read stdout only after `try_wait` reports exit, so a child that
   writes more than one 64 KiB pipe buffer is reported as a timeout. That is
   real, it is bounded, and fixing it is the bounded-run refactor above.
3. No test is ignored, serialised, slept, retried or given a weaker assertion.
4. No change to `contracts/**`, `scripts/contract-parity.sh`,
   `src/portfolio/share/**`, `tests/portfolio_share_*` or
   `tests/manifest_wire_contract.rs`.
