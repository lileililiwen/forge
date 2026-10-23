# Change: upgrade-fleet-verdict-truth

## Why

`forge upgrade --all` runs one upgrade lane over every registered
project and then states a verdict for the whole fleet. The canonical
capability says what the fleet run owes the operator:

> #### Scenario: Isolated fleet outcomes success
>
> - **WHEN** two eligible projects complete their migrations
> - **THEN** each records its new versions and validation evidence
>
> #### Scenario: Isolated fleet outcomes failure
>
> - **WHEN** one project fails midway through a fleet run
> - **THEN** its partial state and recovery steps are reported without
>   marking the fleet wholly successful

So the fleet reports per-project outcomes, and it does not mark a fleet
wholly successful when a project failed. But the fleet's verdict rule
never asked whether the projects were judged at all:

```rust
// src/upgrade/model.rs
impl FleetReport {
    /// A fleet is healthy only when no project failed or blocked.
    /// Skipped (already satisfied, unavailable, dry-run) is healthy.
    pub fn healthy(&self) -> bool {
        self.failed == 0 && self.blocked == 0
    }
}
```

A project that could not be judged — its registry record was unreadable,
or its directory is gone — was recorded as `skipped` and counted as
healthy. The per-project `skipped` status did not distinguish the two
very different reasons a project is skipped:

| Reason the project was skipped | Judged? | Fleet verdict today |
|---|---|---|
| already satisfied — the plan is a no-op | yes | healthy |
| the project path is unavailable | **no** | **healthy** |
| the project record was unreadable | **no** | **healthy** |

A fleet of nothing but unavailable projects therefore reported itself
healthy. Driven on this host, against a registry whose only project had
its directory removed:

```text
fleet upgrade: 0 success, 0 failure, 0 blocked, 1 skipped (selection: gone-app)
  [skipped] gone-app: skipped: project path unavailable; nothing was changed
verdict: healthy
```

and the command **exited 0**.

**Nothing was upgraded and nothing was judged.** The verdict "healthy"
was the absence of a failure, read as the presence of a success — the
same mistake the release lane made with its DriftWatch check, and the
same mistake the delivery lane made with the provider's health.

The JSON document was worse in one way and better in another. It
published `succeeded: 0`, `failed: 0`, `blocked: 0`, `skipped: 1` and
**no verdict at all**, so a reader of the JSON could not tell a judged
fleet from an unjudged one even by counting. And the exit code — 0,
success — was read from the same `failed == 0 && blocked == 0` rule.

Three surfaces read one wrong rule: the human renderer's `verdict:`
line, the command's exit code, and the JSON document that published no
verdict at all.

## What Changes

- **One typed fleet verdict.** A fleet's verdict is `healthy` only when
  every selected project was judged and none failed or blocked. A
  project that could not be judged makes the verdict `unavailable`,
  never `healthy`. A project that failed or blocked makes it
  `not healthy`.
- **The per-project status tells the truth.** A project whose record
  was unreadable, or whose path was unavailable, is reported as
  `unavailable` — a distinct result from `skipped`, which means the
  fleet judged the project and chose not to change it.
- **Every surface reads the verdict.** The human renderer states it,
  the JSON document publishes it, and the exit code follows it.
- **The claims are pinned.** A repository contract check drives the
  real CLI against a fleet whose project is unavailable.

## BFS Impact Map

| Surface | Owner | In this package | Notes |
|---|---|---|---|
| The fleet verdict rule | `forge` | Yes | One typed verdict, computed once from the entries. |
| The fleet report model | `forge` | Yes | `src/upgrade/model.rs`: the verdict type and the rule. |
| The fleet run | `forge` | Yes | `src/upgrade/engine.rs`: the unjudged per-project status and the verdict. |
| The fleet human renderer | `forge` | Yes | `src/upgrade/engine.rs`: the `verdict:` line and the header counts. |
| The fleet JSON document | `forge` | Yes | `src/cli/feature.rs`: the document gains the verdict. |
| The fleet exit code | `forge` | Yes | `src/cli/feature.rs`: the verdict decides the exit code. |
| The single-project upgrade | `forge` | Read only | `forge upgrade <project>` is unchanged; it fails loudly on an unreadable project. |
| The fleet registry surface | `forge` | Read only | `forge fleet list`/`status`/`inspect` are a different fleet and are unchanged. |
| The upgrade plan | `forge` | Read only | `plan_upgrade` is unchanged. |
| The fleet verdict contract test | `forge` | Yes | `tests/upgrade_fleet_verdict_truth_contract.rs` (new). |
| The `upgrade-fleet-verdict-truth` capability | `forge` | Added | this package |
| The `project-upgrade-orchestration` capability | `forge` | Modified | The fleet outcome requirement names the rule. |

| Concern | Result |
|---|---|
| Requirements | `requirement.md` §27 (distribution) owns the upgrade lane; the `project-upgrade-orchestration` capability owns the fleet outcomes. |
| Contracts | The fleet report gains one always-present field (`verdict`) and one count (`unavailable`); a project that could not be judged changes its per-project `status` from `skipped` to `unavailable`. No field is removed or renamed, and the `0.1.0` contract version is unchanged. |
| Callers | `forge upgrade --all` (human and JSON), the `forge upgrade --all` exit code. |
| Persistence | The journal rows are unchanged: an unavailable project still records a `skipped` operation row, because nothing was applied. |
| Integrations | None. No provider is contacted. |
| Failures | No new error code. The exit code for an unavailable fleet becomes 1, where it was 0. |
| Tests | A new `tests/upgrade_fleet_verdict_truth_contract.rs`. |
| Dependencies | None added. |
| Concerns | Quality: the verdict now distinguishes a judged fleet from an unjudged one. Compatibility: a reader that checked the fleet's `status` values now sees `unavailable` where it saw `skipped`, and a script that relied on the exit code for an unavailable fleet sees 1 instead of 0 — that is the point of the change. Security: no new surface; the entries are the run's own. |

## Capabilities

| Capability | Owner | Type | State | Notes |
|---|---|---|---|---|
| `upgrade-fleet-verdict-truth` | forge | Rust | added | this package |
| `project-upgrade-orchestration` | forge | Rust | modified | the fleet outcome requirement names the rule |

## Non-Goals

- **The single-project upgrade is unchanged.** `forge upgrade <project>`
  already fails loudly on a project it cannot read.
- **A dry-run fleet keeps its verdict.** A dry run judges the plan, not
  the applied state, and its header already says `(dry-run)`. A dry-run
  fleet with an unavailable project still reports `unavailable`.
- **The `skipped` result is unchanged.** A project that was judged and
  needed no change is still `skipped`, and a fleet of such projects is
  still healthy.
- **The plan, the receipts and the manifest are unchanged.** No file is
  written differently.
- **The contract version is unchanged.**
