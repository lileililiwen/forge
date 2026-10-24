# Change: release-plan-verdict-truth

## Why

`forge release prepare` records a completed release in the registry
journal.

Reproduced on the current tree, with a project whose Gate evidence
satisfies the shared delivery rule:

```
$ forge release prepare /tmp/rel-probe/proj --version 1.0.0
...
summary: release plan ready for `rel-probe`: 1 check(s) passed at revision `8cee66eb27b`
```

and the append-only row the same run wrote:

```
('release', 'done', 'release prepare `rel-probe` v1.0.0 revision=`8cee66eb27be9c8555f21e9b80e61abca0fc2e88` ready=true dry_run=false')
```

`state='done'`. Nothing was released. The prepare computed a plan; it
committed nothing, tagged nothing, published nothing. In the release
journal `done` is the state the *apply* lane writes after every stage
of an applied release completed — so a preparation and an applied
release are the same row, and a reader of the journal cannot tell a
plan from a delivery.

**The lane already knows the difference.** The sibling command
`forge release apply --dry-run` records `planned`, which is the
canonical rule in the `release-dry-run-evidence-truth` capability:

> **WHEN** a dry run of a release apply is recorded in the registry
> journal
> **THEN** the row records a planned run rather than a completed one

A preparation is a plan in exactly that sense — it released nothing —
but it is recorded as a completed run.

**The prepare's dry run is recorded the same way.** The command takes
`--dry-run`, and the row says so, while still claiming `done`:

```
('release', 'done', 'release prepare `rel-probe` v1.0.0 revision=`8cee66eb27be9c8555f21e9b80e61abca0fc2e88` ready=true dry_run=true')
```

So the same lane answers differently for the same kind of run: the
apply lane's dry run is `planned`, the prepare lane's dry run is
`done`.

**The state is chosen in two places, from two different rules.**

```rust
// src/cli/release.rs, before
let state_label = if plan.healthy() { "done" } else { "blocked" };
```

```rust
// src/cli/release.rs, before
let state_label = if report.dry_run {
    "planned"
} else if report.healthy() {
    "done"
} else {
    "partial"
};
```

Two writers, two private copies of "what did this run do". The
prepare's copy has no dry-run arm at all, which is how a plan reached
`done`.

This is the rule the repair programme is built on: Forge states
something its own gathered evidence does not support. The journal's
evidence — the detail — says `release prepare` and `ready=true`; the
state next to it says the release is done.

## What Changes

- **One rule names what a release run did**, in one place in the
  release model: a plan, a blocked plan, an applied release or an
  applied release that did not complete every stage.
- **The journal state follows that rule.** A preparation records
  `planned` when its checks allow the release and `blocked` when a
  check blocks it. It never records `done`.
- **Both writers read the one rule.** The apply lane's dry run keeps
  recording `planned`, an applied release keeps recording `done` and a
  partial one `partial`; the detail of both rows names the run.

## BFS Impact Map

| Surface | Owner | In this package | Notes |
|---|---|---|---|
| The run rule | `forge` | Yes | `src/release/model.rs`: `ReleaseRun` and its journal state. |
| The release prepare | `forge` | Yes | `src/cli/release.rs`: the journal row follows the rule. |
| The release apply | `forge` | Yes | `src/cli/release.rs`: the journal row reads the same rule. |
| The release contract test | `forge` | Added | `tests/release_plan_verdict_truth_contract.rs`. |
| The `release-plan-verdict-truth` capability | `forge` | Added | this package |
| The `release-publishing` capability | `forge` | Modified | the preparation requirement names the journal rule. |
| The `release-dry-run-evidence-truth` capability | `forge` | Modified | the journal scenario covers the preparation too. |
| The release plan API route | `forge` | Read only | It is a read-only plan view and writes no journal row. |
| The release state file | `forge` | Read only | A prepare writes no release state. |
| The release contract version | `forge` | Read only | Unchanged. |

| Concern | Result |
|---|---|
| Requirements | `requirement.md` §27 owns the release lane; the `release-publishing` capability owns the preparation and the `release-dry-run-evidence-truth` capability owns the journal rule. |
| Contracts | No field is removed or renamed. The journal gains one detail field, `verdict=`. `RELEASE_CONTRACT_VERSION` is unchanged. |
| Callers | `forge release prepare` and `forge release apply`, their journal rows. |
| Persistence | The journal state for a ready preparation becomes `planned`; a blocked preparation keeps `blocked`; the apply lane keeps `done`, `partial` and `planned`. |
| Integrations | None. No adapter, provider or repository is touched. |
| Failures | No new error code. A prepare still exits 0 and a blocked plan still exits 0 with its reason. |
| Tests | A new contract test `tests/release_plan_verdict_truth_contract.rs`; the existing release contract and cross-surface tests stay green. |
| Dependencies | None added. |
| Concerns | Quality: the release journal can no longer be read as "a release happened" when only a plan was computed. Compatibility: a reader of the journal state for a preparation now reads `planned` instead of `done`; the detail gains a `verdict=` field. Security: no new surface; the redaction rule and the evidence handling are unchanged. |

## Capabilities

| Capability | Owner | Type | State | Notes |
|---|---|---|---|---|
| `release-plan-verdict-truth` | forge | Rust | added | this package |
| `release-publishing` | forge | Rust | modified | the preparation requirement names the journal rule. |
| `release-dry-run-evidence-truth` | forge | Rust | modified | the journal scenario covers the preparation too. |

## Non-goals

- **The prepare's output is unchanged.** The plan still prints its
  checks, its readiness and its note.
- **The apply lane's behaviour is unchanged.** A dry run still writes
  nothing and records `planned`; a confirmed release still writes and
  records `done`; an incomplete one still records `partial`.
- **The release state file is not in this package.** A prepare writes
  none.
- **The API's release plan route is not in this package.** It is a
  read-only view and journals nothing.
- **The release contract version is unchanged.**
- **The analytics metric counters are not in this package.** Their
  decision is recorded in the handoff, not here.
