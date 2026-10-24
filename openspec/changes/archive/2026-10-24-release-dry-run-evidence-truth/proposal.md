# Change: release-dry-run-evidence-truth

## Why

`forge release apply --dry-run` states that nothing happened. The
release engine still writes.

```rust
// src/release/engine/apply.rs, before
let note = if dry_run {
    "dry-run: no side effects were attempted".to_string()
} else if healthy {
```

The release report contradicts itself, and the repository agrees with the
report rather than with the note. The owning agent's own run of the
release engine on a fixture produced:

```
NOTE=dry-run: no side effects were attempted
STAGE=tag STATUS=delivered NOTE=tag `v1.0.0` delivered for release `rel-scratch-1.0.0-aef2a7e9d820`
TAGS=v1.0.0
```

The note says no side effect was attempted. The stage outcome in the same
report says a tag was delivered. And the tag is in the repository.

Only three of the eight stages are gated on `--confirm`:

| stage | dry-run behaviour today |
|---|---|
| commit | commits the changelog — **ungated** |
| tag | creates a real annotated git tag — **ungated** |
| push | disabled (`--confirm` gate) |
| mirror | plan only, but recorded as **`delivered`** |
| package | disabled (`--confirm` gate) |
| container | disabled (`--confirm` gate) |
| docs | translates and writes files — **ungated** |
| notes | renders the notes file |

The mirror stage is the second half of the same defect. A plan-only
release records the mirror stage as a delivery:

```rust
// src/release/engine/stages.rs, before
status: if plan.healthy() {
    STATUS_DELIVERED.to_string()
} else {
    STATUS_FAILED.to_string()
},
```

while the stage's own note says `mirror plan only: primary ...`. The
plan is healthy because a plan is always healthy — `plan_mirror` hard-codes
`healthy: true` — so the stage that pushed nothing is the one stage that
claims it pushed.

This breaks the rule the repair programme is built on: Forge states
something its own gathered evidence does not support. A dry run's whole
value is that it is safe. A dry run that tags the repository is not a dry
run, and the note that says so is the one claim in the report nobody
verified.

## What Changes

- **One gate decides whether a stage may write.** It lives on the
  release request, in one place, and every stage reads it: a stage writes
  only when the release is confirmed **and** the run is not a dry run.
- **Every write-capable stage consults the gate.** A stage that may not
  write records a non-delivered status and a note that names the reason
  it did not write.
- **The mirror stage of a plan-only release is no longer `delivered`.**
  It keeps the plan evidence it already collects and reports the plan.
- **The dry-run note names the planned stages**, and the registry
  journal records a dry run as `planned` rather than `done`.

## BFS Impact Map

| Surface | Owner | In this package | Notes |
|---|---|---|---|
| The write gate | `forge` | Yes | `src/release/model.rs`: `ReleaseRequest::may_write()` and `ReleaseRequest::write_refusal()`. |
| The commit stage | `forge` | Yes | `src/release/engine/apply.rs`: it takes the request and consults the gate. |
| The tag stage | `forge` | Yes | `src/release/engine/apply.rs`: it takes the request and consults the gate. |
| The push stage | `forge` | Yes | `src/release/engine/stages.rs`: the `--confirm` gate becomes the shared gate. |
| The mirror stage | `forge` | Yes | `src/release/engine/stages.rs`: the plan-only path is no longer `delivered`. |
| The package stage | `forge` | Yes | `src/release/engine/stages.rs`: the `--confirm` gate becomes the shared gate. |
| The container stage | `forge` | Yes | `src/release/engine/stages.rs`: the `--confirm` gate becomes the shared gate. |
| The docs stage | `forge` | Yes | `src/release/engine/stages.rs`: it takes the request and consults the gate. |
| The notes stage | `forge` | Yes | `src/release/engine/stages.rs`: it takes the request and consults the gate. |
| The release note | `forge` | Yes | `src/release/engine/apply.rs`: the dry-run note names the planned stages. |
| The registry journal | `forge` | Yes | `src/cli/release.rs`: a dry run is recorded as `planned`. |
| The release report | `forge` | Read only | `src/release/model.rs`: `ReleaseReport` keeps every field, including `healthy`. |
| The stage statuses | `forge` | Read only | `delivered`, `skipped`, `disabled`, `failed`, `conflict` and `diverged` are unchanged. |
| The release state file | `forge` | Read only | `.forge/release/<id>/state.json` is still written, and still holds the per-stage record. |
| The release contract version | `forge` | Read only | `RELEASE_CONTRACT_VERSION` is unchanged. |
| The `release-dry-run-evidence-truth` capability | `forge` | Added | this package |
| The `release-publishing` capability | `forge` | Modified | the confirm-gated apply requirement names the rule. |

| Concern | Result |
|---|---|
| Requirements | `requirement.md` §29, §34, §43 own the gated release; the `release-publishing` capability owns the confirm-gated apply. |
| Contracts | No field is added, removed or renamed. The `note` string and the per-stage status change. The contract version is unchanged. |
| Callers | `forge release apply` (human and JSON), its exit reasons and its journal row; the MCP release surface, which calls the same engine. |
| Persistence | The release state file is unchanged. The registry journal state for a dry run becomes `planned`. |
| Integrations | None. No adapter is changed; the notes adapter already receives `--dry-run`. |
| Failures | No new error code. A dry run still exits 0; a real apply that fails still exits 1. |
| Tests | Unit tests in `src/release/engine/engine_tests.rs`; a new contract test `tests/release_dry_run_truth_contract.rs`. |
| Dependencies | None added. |
| Concerns | Quality: a dry run now has no side effect, so its note is true. Compatibility: a reader matching the phrase "dry-run: no side effects were attempted" still finds it, with the planned stage count appended; a reader matching a `delivered` mirror stage of a plan-only release must read `disabled`. Security: no new surface; no secret is involved. |

## Capabilities

| Capability | Owner | Type | State | Notes |
|---|---|---|---|---|
| `release-dry-run-evidence-truth` | forge | Rust | added | this package |
| `release-publishing` | forge | Rust | modified | the confirm-gated apply requirement names the rule. |

## Non-Goals

- **The confirm gate is not weakened.** A release that is neither
  confirmed nor a dry run is still refused with
  `release-invalid` before any stage runs.
- **A confirmed, non-dry release still writes.** The gate opens exactly
  as before for `forge release apply --confirm`; the tag, the commit and
  the provider stages are unchanged for that run.
- **No new stage status is introduced.** `disabled` already means "this
  stage did not run"; the note now names the reason.
- **The release report's shape is unchanged.** `healthy` still means "no
  stage failed"; a dry run that planned every stage is healthy because
  nothing failed, and its note says that nothing was written.
- **The mirror contract is unchanged.** The plan still collects the same
  per-remote evidence; only the stage status stops claiming a delivery.
