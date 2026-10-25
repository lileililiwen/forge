# Change: fleet-publish-summary-verdict-truth

## Why

The fleet publish summary counted a plan as a success, and counted a
delivery as nothing.

```
$ forge --registry /tmp/fleet/registry.db publish fleet \
    --inventory /tmp/fleet/inventory.json --provider probe --dry-run
publish alpha: provider=probe
fleet publish: 1/1 publishable succeeded, 0 held back, 0 skipped (1 in inventory:...)
```

That run was a dry run. It contacted nobody, deployed nothing, and the
lane's own per-project row said `verdict: planned` with `healthy:
false`. The summary said "succeeded" anyway.

The JSON document says the same thing in numbers:

```json
"fleet": {
  "publishable": 1,
  "success": 1,
  "failed": 0,
  "projects": [{"project": "alpha", "healthy": false, "verdict": "planned"}]
}
```

The summary contradicts the rows it summarises.

The counters were a binary, not the verdict:

```rust
// src/cli/fleet.rs, before
// A plan is not a failure: the fleet counts it
// as neither, and it never trips `--fail-fast`.
if verdict == PublishVerdict::Failed {
    failure_count += 1;
    ...
} else {
    success_count += 1;
}
```

The comment says the fleet counts a plan "as neither". The code counts
it as a success. The code's own rule and its own implementation
disagree.

The same binary is in the other two fleet lanes, and it misses the
other side of the truth: the parallel lane has no `else` branch at
all, so a delivery there is never counted as a success.

This is the programme's rule class: Forge states something its own
gathered evidence does not support — the summary claims deliveries the
lane's own rows deny.

## What Changes

- **The summary counts the verdict.** A delivery is a success, a plan
  is a plan, a failure is a failure. The counters come from the run's
  verdict, so a plan is never counted as a success and a delivery is
  never counted as nothing.
- **The summary names the plans.** The JSON document gains a `planned`
  counter and the human line names the planned runs.

## BFS Impact Map

| Surface | Owner | In this package | Notes |
|---|---|---|---|
| The fleet publish summary | `forge` | Yes | `src/cli/fleet.rs`: the counters come from the verdict. |
| The fleet publish verdict rule | `forge` | Read only | `src/cli/fleet.rs` (`fleet_project_verdict`) and `src/publish/model.rs` (`verdict_for_report`) are unchanged. |
| The per-project rows | `forge` | Read only | Unchanged: the verdict, the health and the journal row already tell the truth. |
| The journal rows | `forge` | Read only | Unchanged: a plan still journals `planned`. |
| The fail-fast behaviour | `forge` | Read only | Unchanged: a plan never trips `--fail-fast`, a failure still does. |
| The fleet web envelope | `forge` | Read only | Unchanged: it is a different document with its own counts. |
| The upgrade fleet lane | `forge` | Read only | Unchanged: it already counts each status separately. |
| The manual publish lanes | `forge` | Read only | Unchanged (change 48 owns them). |

| Concern | Result |
|---|---|
| Requirements | `requirement.md` §33 owns the release and publish workflow; the `publish-fleet-dry-run-verdict-truth` capability owns the fleet lane's verdict rule. |
| Contracts | No field is removed or renamed. The JSON document's `success` counter keeps its name and now counts deliveries only, and a `planned` counter is added. |
| Callers | `forge publish fleet`. |
| Persistence | No journal row changes. |
| Integrations | None. No adapter, provider or repository is touched. |
| Failures | No new error code. The exit status still reflects the failure count. |
| Tests | A new contract test `tests/fleet_publish_summary_verdict_truth_contract.rs`; the existing fleet tests stay green. |
| Dependencies | None added. |

## Capabilities

| Capability | Owner | Type | State | Notes |
|---|---|---|---|---|
| `publish-fleet-dry-run-verdict-truth` | forge | Rust | modified | the requirement states that the summary counts the verdict. |

No new capability: the rule already exists and is already promoted. The
summary's counters did not read it.

## Non-goals

- **The per-project rows are unchanged.** They already name the verdict,
  the health and the journal state.
- **The journal rows are unchanged.** A plan still journals `planned`.
- **The fail-fast behaviour is unchanged.** A plan never trips
  `--fail-fast`; a failure still does.
- **The exit status is unchanged.** The command still fails when a run
  failed.
- **The upgrade fleet lane is unchanged.** It already counts each
  status separately.
