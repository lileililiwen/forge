# Change: publish-dry-run-verdict-truth

## Why

`forge publish --dry-run` records a completed publish in the registry
journal.

```rust
// src/publish/pipeline.rs, before
if let Some(reg) = registry {
    let verdict = if request.dry_run || healthy {
        "done"
    } else {
        "failed"
    };
    let _ = reg.record_operation(
        "publish",
        &request.project_id,
        verdict,
        &format!(
            "publish {} summary via {}: healthy={} stages={}",
            request.action.label(),
            adapter.id(),
            healthy,
            stages.len()
        ),
    );
}
```

The row says `done`, and its detail says `healthy=true`, for a run that
published nothing. The report's own summary, in the same function,
says the opposite:

```rust
// src/publish/pipeline.rs, before
let note = if request.dry_run {
    format!("dry-run: {} stage(s) planned, no side effects", stages.len())
} else if healthy {
```

So one document says "no side effects" and its own journal row says
"done, healthy=true". The append-only evidence a later reader consults
claims a publication that never happened.

**The same rule is written three times.** The `dry_run || healthy`
expression appears in the pipeline and again in the fleet lane:

```rust
// src/cli/fleet_exec.rs, before
let verdict = if report.dry_run || report.healthy {
    "done"
} else {
    "failed"
};
```

and the fleet lane's own per-project state follows the report's
boolean:

```rust
// src/cli/fleet.rs, before
let terminal_state = if healthy { "done" } else { "failed" };
```

Three copies of one decision, and the decision cannot say "planned",
because a boolean has only two values. So all three say `done` for a
plan.

This is the rule the repair programme is built on: Forge states
something its own gathered evidence does not support. A dry run's whole
value is that it is safe, and its journal row is the evidence that it
was.

## What Changes

- **One rule judges a publish run**, in the one place the publish
  report is built: a typed verdict that says what the run did —
  `delivered`, `planned` or `failed`.
- **`healthy` is true only for `delivered`.** A plan delivered
  nothing.
- **Every journal writer reads that one verdict.** The pipeline, the
  fleet lane and the fleet exec lane stop each keeping a private copy
  of the `dry_run || healthy` decision, and a plan is recorded as
  `planned`, never as `done`.
- **The exit code follows the verdict.** A plan is a successful plan
  and still exits 0.

## BFS Impact Map

| Surface | Owner | In this package | Notes |
|---|---|---|---|
| The verdict rule | `forge` | Yes | `src/publish/model.rs`: `PublishVerdict` and `verdict_for_report`. |
| The publish report | `forge` | Yes | `src/publish/model.rs`: the report gains `verdict`; `healthy` is derived from it. |
| The publish pipeline | `forge` | Yes | `src/publish/pipeline.rs`: the report's verdict, and the journal state it writes. |
| The publish CLI | `forge` | Yes | `src/cli/publish.rs`: the exit code follows the verdict. |
| The fleet exec lane | `forge` | Yes | `src/cli/fleet_exec.rs`: the journal state follows the verdict. |
| The fleet lane | `forge` | Yes | `src/cli/fleet.rs`: the per-project state and the summary follow the verdict. |
| The release lane's publish stage | `forge` | Read only | It reads the report's note, not its health. |
| The MCP publish method | `forge` | Read only | It serialises the report, so it carries the verdict. |
| The publish contract version | `forge` | Read only | `PUBLISH_CONTRACT_VERSION` is unchanged. |
| The `publish-dry-run-verdict-truth` capability | `forge` | Added | this package |
| The `decoupled-remote-publish` capability | `forge` | Modified | the publish requirement names the verdict. |

| Concern | Result |
|---|---|
| Requirements | `requirement.md` §29 owns the publish surface; the `decoupled-remote-publish` capability owns the publish report and its summary. |
| Contracts | One field is added to the report: `verdict`. No field is removed or renamed, and the contract version is unchanged. |
| Callers | `forge publish` (human and JSON), its exit code and its journal row; `forge fleet publish`, per project; the MCP publish surface. |
| Persistence | The publish report's shape gains one field. The registry journal state for a plan becomes `planned`. |
| Integrations | None. No adapter is changed. |
| Failures | No new error code. A plan still exits 0; a failed publish still exits 1. |
| Tests | A new contract test `tests/publish_dry_run_verdict_truth_contract.rs`; the existing publish and fleet contract tests stay green. |
| Dependencies | None added. |
| Concerns | Quality: the journal stops claiming a publication that did not happen. Compatibility: a reader matching the journal state `done` for a dry run now sees `planned`; a reader matching `healthy == true` on a dry run now sees `false` and reads `verdict == "planned"`. Security: no new surface; no secret is involved. |

## Capabilities

| Capability | Owner | Type | State | Notes |
|---|---|---|---|---|
| `publish-dry-run-verdict-truth` | forge | Rust | added | this package |
| `decoupled-remote-publish` | forge | Rust | modified | the publish requirement names the verdict. |

## Non-Goals

- **No stage behaviour changes.** A dry run still runs the same stages
  in the same order and still contacts no target.
- **The note is unchanged.** Change 30 already fixed the publish
  summary's claim; this change fixes what the journal and the report's
  boolean say about the same run.
- **The deploy lane is not in this package.** `forge deploy` has its
  own report and its own journal rule; it is separate work.
- **The provider branch of the fleet lane keeps its own evidence.** The
  provider's own `health` field is what that branch reads; this change
  governs the manual path, where the publish report is the evidence.
- **The release lane's publish stage is unchanged.** It reads the
  report's note.
- **The publish contract version is unchanged.** `healthy` already
  meant "the stages delivered"; a plan never delivered.
