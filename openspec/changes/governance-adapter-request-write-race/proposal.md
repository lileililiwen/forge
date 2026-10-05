# Proposal: a governance adapter that never reads its request can still answer

## Why

`tests/governance_contract.rs` is nondeterministic. It passes in isolation and
fails roughly half the time when the whole target runs, with a varying number
and identity of failures. Over 20 measured runs of the target: **13 failures
across 8 of 20 runs, in exactly four tests**, and never in any other test.

| Test | Failures in 20 runs |
|---|---|
| `valid_external_response_is_normalized_and_redacted` | 5 |
| `unknown_external_status_is_incompatible` | 3 |
| `malformed_external_response_is_incompatible` | 3 |
| `workspace_governance_evidence_is_redacted_and_bounded` | 2 |
| every other test in the target | 0 |

The four tests that fail are exactly — and not coincidentally — the four whose
adapter script **does not read standard input**:

```sh
#!/bin/sh
printf '%s' '{"provider":"external", ...}'
```

Every adapter that *does* read its request starts with `req=$(cat)`, which
blocks the writer until the request has been consumed, and those tests never
failed once.

The mechanism is in `run_adapter` (`src/governance.rs:612-618`): Forge writes
the request to the child's stdin, then polls `try_wait`. A child that never
reads stdin exits immediately and the kernel closes the read end, so the
write's outcome is decided by whether Forge's `write` syscall happens to land
before the child's exit. Losing that race returns `Broken pipe (os error 32)`,
which `run_external_provider` maps to `ProviderStatus::Unavailable`. The
child's own answer — already on stdout, already complete — is never read.

The deciding evidence is that **serialising the tests does not fix it**:

```
$ for i in $(seq 1 20); do cargo test --test governance_contract -- --test-threads=1; done
7 runs FAILED (1-3 tests each), 13 runs ok
```

So the flake is not cross-test shared mutable state: no environment variable,
`OnceLock`, `static`, current-directory change, shared temp path or fixed port
is involved. `save_provider_selection` and `persist_observation` key their
temp files on `std::process::id()`, but every test owns a distinct
`TempDir::new()`, so those names cannot collide. Serialising the target leaves
the failure rate essentially unchanged, which is what a per-test race does and
what shared state does not.

This is a **product defect**, not a test defect. The provider contract
declares adapters to be optional external executables that Forge does not own;
`Provider failure isolation` requires Forge to surface a real failure
explicitly. Deciding "unavailable" because Forge lost a write race against an
adapter that had already answered is not a provider failure — it is Forge
misreporting its own timing.

## What Changes

- `run_adapter` distinguishes "the adapter closed its input" from "Forge
  could not talk to the adapter". A `BrokenPipe` on the request write is no
  longer an error; Forge proceeds to read the exit status, stdout and stderr
  the adapter actually produced, and the adapter's real answer decides the
  observation. Every other write error keeps its existing typed
  `GovernanceUnavailable` refusal.
- A write failure that is genuinely not `BrokenPipe` now kills and reaps the
  child before returning, so the refusal path cannot leave a live adapter
  behind. This is the same kill-and-reap discipline
  `runtime-hardening-and-test-isolation` already requires of the release
  adapter path, applied to the sibling path that lacked it.
- A deterministic regression guard for the violated invariant, plus an
  end-to-end test whose adapter closes its own stdin before answering, so the
  scenario is named in the suite rather than left to scheduling luck.

No contract, schema, persisted shape, CLI flag, environment variable or
provider-selection behaviour changes. The observation for a given adapter
answer is byte-identical; only the case where the previous code threw the
answer away changes.

## BFS Impact Map

| Surface | State today | Action |
|---|---|---|
| `src/governance.rs:612-618` (`run_adapter` request write) | any write error → typed `GovernanceUnavailable`, child left running, child's answer discarded | `BrokenPipe` falls through to the existing wait/read path; other errors kill + reap first |
| `src/governance.rs:475-487` (`run_external_provider` error mapping) | maps `GovernanceUnavailable` to `ProviderStatus::Unavailable` | **unchanged** — the mapping was correct; it was fed a false input |
| `src/governance.rs:619-668` (wait/read/timeout loop) | polls `try_wait`, drains stdout/stderr, enforces `MAX_ADAPTER_OUTPUT_BYTES` and the deadline | **unchanged** — this is where the answer was already being read |
| `src/governance.rs:644-648` (successful output) | returns exit status, stdout, bounded stderr | **unchanged** |
| timeout path `src/governance.rs:650-658` | kills and reaps the child | **unchanged** |
| `tests/governance_contract.rs` — the four non-reading adapters | flaky, `Unavailable` | become deterministic; behaviour asserted end to end |
| `tests/governance_contract.rs` — the six `req=$(cat)` adapters | never flaked | **unchanged** |
| `src/governance.rs` in-module `mod tests` | no coverage of the request-write boundary | add the deterministic guard |
| `tests/fixtures/governance-audit/*` | verbatim recorded adapter output | **untouched** — they are evidence, not test scaffolding |
| `openspec/specs/governance-provider-contract/spec.md` | "Provider failure isolation" | one scenario added |
| `contracts/**`, `scripts/contract-parity.sh`, `src/portfolio/share/**`, `tests/manifest_wire_contract.rs`, `tests/portfolio_share_*` | owned elsewhere | **not touched** |
| `openspec/changes/contract-parity-gate-real-digests/`, `manifest-wire-contract-shape` | other in-flight changes | **not touched** |

### Shared-state audit (the hypothesis that was tested and rejected)

Every candidate cross-test global reachable from this target was checked and
cleared, so the fix does not paper over one:

| Candidate | Result |
|---|---|
| `std::env::set_var` / `remove_var` | none in `tests/governance_contract.rs`; none in `src/governance.rs` or `src/core` |
| `std::env::set_current_dir` | not used anywhere in `src/` |
| `static` / `OnceLock` / `lazy_static` | none in `src/governance.rs`; `src/vocabulary.rs`'s `VOCABULARY_ENV_LOCK` / thread-local override are not on this path |
| shared temp dir | every test uses its own `TempDir::new()` |
| shared temp *file name* | `.providers.yaml.tmp-<pid>` and `.observations.json.tmp-<pid>` share the pid but live under per-test `TempDir`s, so they cannot collide |
| fixed port / socket | no socket is opened on this path |
| shared repo file mutated | the four fixtures under `tests/fixtures/governance-audit/` are only read; `configured_sibling_removed_after_selection_is_unavailable` removes the **staged copy** in its own `TempDir`, not the fixture |
| `Drop` / cleanup race | `TempDir` cleanup is per-test and post-assertion |

## Capabilities

- `governance-provider-contract` — provider failure isolation distinguishes a
  real adapter failure from Forge losing a write race against an adapter that
  has already answered.

## Non-goals

- **`--test-threads=1` as a remedy.** It was run 20 times as *evidence* and
  it does not fix the flake. It is not committed anywhere.
- **Draining stdout while the child runs.** `run_adapter` only reads the
  child's stdout after `try_wait` reports it exited, and a pipe buffer is
  64 KiB against a 256 KiB `MAX_ADAPTER_OUTPUT_BYTES`. An adapter that writes
  more than one pipe buffer therefore deadlocks until its deadline. That is a
  real latent defect, it is **not** the cause of this flake, and fixing it
  needs concurrent drain + deadline handling — recorded in `design.md` §5 as
  outstanding, not fixed here.
- **The unbounded `git rev-parse` call** in `git_revision`
  (`src/governance.rs:796-805`), which has no deadline at all.
- **Archiving.** This change is left active with evidence rather than archived,
  per owner direction alongside the two other unarchived changes.
- **Serialising the target, `#[ignore]`, sleeps or retry loops.** None is used.
- **`tests/studio_preview_contract.rs::preview_port_collision_is_refused_without_killing_a_listener`.**
  A whole-suite run surfaced a *different* pre-existing flake on the way to
  verifying this one. It binds the fixed range 45800–45863, which sits inside
  this machine's Linux ephemeral port range (`32768 60999`), so a concurrent
  outbound connection from any process takes one of the 64 ports and the
  test's own bind returns 63. Mechanism proved with a process holding the
  range. It is a different file, a different concern, and a different fix
  (a free port *range*, not a free port); it is reported, not absorbed.
- **One unexplained failure in 245 runs** of the fixed target. The capture did
  not record which test or why, and it was not reproduced in 220 further runs.
  It is not claimed as fixed.
