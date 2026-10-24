# Change: mirror-plan-verdict-truth

## Why

`forge mirror --dry-run` reports a delivery it did not make.

```rust
// src/distribution/engine.rs, before
let report = MirrorReport {
    ...
    note: "plan only; no push was attempted".to_string(),
    healthy: true,
};
```

`plan_mirror` hard-codes `healthy: true`. The report's own note says no
push was attempted; the report's `healthy` field says the run was
healthy. Nothing in the plan was ever written, so nothing established a
delivery.

The boolean cannot say "nothing was attempted". A boolean has two
values, and the code uses `true` for both "every remote received the
ref" and "I did not try". So every surface that reads it has to guess
which one it is holding.

**The same configuration gets two different verdicts.** A mirror
declared on a provider that is planned but not supported produces an
`unavailable` outcome in both runs. The apply reads it as a failure:

```rust
// src/distribution/model.rs, before
pub(super) fn healthy_check(&self) -> bool {
    if self.dry_run {
        return true;
    }
    for outcome in &self.outcomes {
        match outcome.status.as_str() {
            "delivered" | "skipped" | "disabled" => continue,
            _ => return false,
        }
    }
```

The plan ignores the same outcome and reports healthy. The plan is
also the run that reaches the journal first, and it reaches it as a
completed operation:

```rust
// src/cli/gitops.rs, before
let state_label = if report.healthy() { "done" } else { "partial" };
```

So the append-only evidence says a mirror operation completed, with
`healthy=true dry_run=true` in its detail, for a run that pushed
nothing.

This is the rule the repair programme is built on: Forge states
something its own gathered evidence does not support. The mirror
report is the evidence, and the report contradicts itself.

It also puts the two mirror surfaces in conflict. The release lane's
mirror stage now reports a plan as `disabled`, because the release
dry run writes nothing. The standalone mirror command reports the
same plan as healthy and journals it as `done`.

## What Changes

- **One rule judges a mirror run**, in the one place the report is
  built: a typed verdict that says what the run did — `delivered`,
  `partial` or `planned`.
- **A plan is `planned`, never `delivered`.** `plan_mirror` stops
  hard-coding `healthy: true`; the verdict is derived from the run's
  own evidence.
- **`healthy` is true only for `delivered`.** The report keeps the
  field and gains the verdict, so a reader can tell "delivered" from
  "planned" from "partial".
- **The journal records the plan.** A dry run is recorded as
  `planned`, not `done`, and the exit code stays 0: a plan is not a
  failure.
- **The human renderer names the verdict**, so the summary line and
  the JSON document agree.

## BFS Impact Map

| Surface | Owner | In this package | Notes |
|---|---|---|---|
| The verdict rule | `forge` | Yes | `src/distribution/model.rs`: `MirrorVerdict`, `verdict_for_outcomes`, `MirrorReport::verdict_check`. |
| The mirror plan | `forge` | Yes | `src/distribution/engine.rs`: `plan_mirror` no longer hard-codes `healthy: true`. |
| The mirror apply | `forge` | Yes | `src/distribution/engine.rs`: `apply_mirror` derives the verdict from the outcomes. |
| The human renderer | `forge` | Yes | `src/distribution/engine.rs`: `render_report_human` prints the verdict. |
| The mirror CLI | `forge` | Yes | `src/cli/gitops.rs`: the journal state, the journal detail and the exit code follow the verdict. |
| The mirror report | `forge` | Read only | `src/distribution/model.rs`: the fields are not added, removed or renamed; `verdict` is added. |
| The MCP mirror method | `forge` | Read only | `src/mcp/methods.rs` serialises the report, so it carries the verdict. |
| The release mirror stage | `forge` | Read only | `src/release/engine/stages.rs` already reports a plan as `disabled`; it does not read the mirror report's `healthy`. |
| The distribution state file | `forge` | Read only | `.forge/distribution/<project>/state.json` is unchanged. |
| The distribution contract version | `forge` | Read only | `DISTRIBUTION_CONTRACT_VERSION` is unchanged. |
| The `mirror-plan-verdict-truth` capability | `forge` | Added | this package |
| The `repository-distribution` capability | `forge` | Modified | the distribution requirement names the verdict. |

| Concern | Result |
|---|---|
| Requirements | `requirement.md` §29 owns the mirror surface; the `repository-distribution` capability owns the per-remote report. |
| Contracts | One field is added to the report: `verdict`. No field is removed or renamed, and the contract version is unchanged. |
| Callers | `forge mirror` (human and JSON), its exit code and its journal row; the MCP `mirror_project` method; the release lane's mirror stage, which reads the plan's evidence and not its health. |
| Persistence | The distribution state file is unchanged. The registry journal state for a plan becomes `planned`. |
| Integrations | None. No provider is contacted by a plan, before or after. |
| Failures | No new error code. A plan still exits 0; a partial delivery still exits 1. |
| Tests | A new contract test `tests/mirror_plan_verdict_truth_contract.rs`; the existing `tests/distribution_contract.rs` and `tests/distribution_cross_surface.rs` stay green. |
| Dependencies | None added. |
| Concerns | Quality: the report and the journal stop claiming a delivery that did not happen. Compatibility: a reader matching `healthy == true` on a dry run now sees `false` and reads `verdict == "planned"`; a reader matching the journal state `done` for a dry run now sees `planned`. Security: no new surface; no secret is involved. |

## Capabilities

| Capability | Owner | Type | State | Notes |
|---|---|---|---|---|
| `mirror-plan-verdict-truth` | forge | Rust | added | this package |
| `repository-distribution` | forge | Rust | modified | the distribution requirement names the verdict. |

## Non-Goals

- **No probe is added.** The verdict is derived from what the run did, not from a new check of the remotes.
- **The per-remote evidence is unchanged.** The outcomes, their notes, their evidence and their recovery lines stay exactly as they are; only the run-level verdict changes.
- **The release lane's mirror stage is unchanged.** It already reports a plan as `disabled` and does not read the mirror report's health.
- **The MCP method is unchanged.** It returns the report, and the report now carries the verdict.
- **The state file is unchanged.** A plan still writes no delivery state, and a retry still only skips what was delivered.
- **The distribution contract version is unchanged.** No existing field changes meaning: `healthy` already meant "the refs reached their remotes", and a plan never reached a remote.
