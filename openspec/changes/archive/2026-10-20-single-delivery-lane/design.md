## Implementation Boundary

Owners:

| Surface | Owner | In this package |
|---|---|---|
| The decision "may this project deliver this revision" | `forge` | Yes. `src/gate/ship_gate.rs`. |
| The delivery lane's preflight | `forge` | Yes. `src/delivery/handlers.rs`. |
| The deploy lane's apply step and its plan | `forge` | Yes. `src/deploy/engine/stages.rs`. |
| The Gate's own contract, evidence format and command surface | `forge` | No. Read only. |
| The deploy adapter contract and the target kinds | `forge` | No. Read only. |
| The delivery provider contract | `forge` | No. Read only. |
| The release lane and its check model | `forge` | No. Read only. |
| The publish lane | `forge` | No. Read only. |
| Every sibling module | its owner | No. Not edited. |

## Language and Runtime

- **Forge Core** is Rust, in the root `forge` crate.
- The rule reads one JSON file (the persisted Gate evidence) and compares it
  with the revision. It spawns nothing and writes nothing. The revision itself
  is already captured by the caller: the deploy lane captures it in
  `prepare_deploy` (`src/deploy/engine/stages.rs:48`) and the delivery lane
  captures it in `verify_registered_head`
  (`src/delivery/handlers.rs:108-146`).
- `src/gate/evidence.rs` is at 998 physical lines, one line under the
  repository's 1000-line cap, so the rule goes in a new file rather than in
  that one.

## Ownership and Shared Code

Today the rule exists once, privately, inside the delivery lane:

```
src/delivery/handlers.rs:148  fn require_fresh_passing_gate(project_dir, project_id, revision)
```

It requires, in one combined condition (`handlers.rs:159-165`): a supported
contract, the full profile, freshness for the given revision, a passed
aggregate, and `dry_run == false`. The deploy lane has no equivalent at all.

The change moves the rule into `src/gate/ship_gate.rs` and gives it one
public entry point:

```rust
pub fn evaluate_ship_gate(
    project_dir: &Path,
    project_id: &str,
    revision: &str,
) -> ShipGateStatus
```

`ShipGateStatus` carries `satisfied: bool` and `reason: String`. It is a
value, not an error, because two callers need both halves of it: the plan
reports it, and the apply step refuses on it.

The two call sites:

| Call site | File | Use |
|---|---|---|
| Deployment plan | `src/deploy/engine/stages.rs::prepare_deploy` | Reports the status and sets the plan's readiness. |
| Deployment apply | `src/deploy/engine/stages.rs::apply_deploy` | Refuses when the status is not satisfied and the run is not a dry run. |
| Delivery preflight | `src/delivery/handlers.rs::run_preflight` | Refuses when the status is not satisfied. |

The delivery lane's private function and its private revision check stay where
they are: the revision check compares the working tree's HEAD with the
*registered* revision, which is a registry rule, not a gate rule. Only the
evidence rule moves.

## User Experience and Interface

No verb is added and no verb changes shape.

`forge deploy apply` on a project without Gate evidence now refuses with
`error[deploy-invalid]` before it spawns the adapter. The refusal names the
project and tells the owner what to run:

```
error[deploy-invalid]: deploy invalid: Forge Gate evidence is absent for `my-app`; run `forge gate --profile full` from the project directory and retry
```

`forge deploy plan` stays a read-only preview and now states whether the
project may deliver:

```
ready: no
note: deploy plan not ready: Forge Gate evidence is absent for `my-app`; run `forge gate --profile full` from the project directory and retry
```

`forge deploy apply --dry-run` is a rehearsal: it contacts no target and
therefore needs no evidence. It is never refused by the rule.

The delivery lane's refusal text changes shape slightly: the shared rule
produces one wording, so the "delivery: " prefix and the "retry preflight"
tail are gone. The reason itself is unchanged, and no test asserts on that
text.

## Behavioral Model

| Actor | Block | Boundary |
|---|---|---|
| A project with fresh, passing, full-profile evidence for the delivered revision | The delivery proceeds. | The evidence must not be a rehearsal. |
| A project with no evidence | The delivery is refused and the plan is not ready. | The reason names the project. |
| A project whose only evidence is a dry-run rehearsal | The delivery is refused. | A rehearsal is not evidence. |
| A project whose evidence is bound to another revision | The delivery is refused. | The reason names the observed freshness. |
| A project whose evidence did not pass | The delivery is refused. | The reason names the observed aggregate. |
| A project whose evidence belongs to another project | The delivery is refused. | The reason names the observed project. |
| A project whose evidence came from a profile other than full | The delivery is refused. | The reason names the observed profile. |
| A dry-run deployment | Nothing is refused and nothing is contacted. | The rule is not evaluated as a gate on a rehearsal. |
| The delivery lane's stage and promote verbs | Unchanged. | They inherit the preflight's verdict through the preflight row, as they do today. |
| The release lane | Unchanged. | It binds evidence through its own check model. |
| The publish lane | Unchanged. | It syncs a source tree and builds on the target; there is no local artifact for a local Gate run to verify. |

## Contract and Compatibility

The Gate's evidence format, its contract version, its freshness rule and its
command surface are untouched. The rule reads the same evidence the delivery
lane reads today, with the same conditions.

The plan gains one field. `DeployPlan` (`src/deploy/model.rs:234-243`) grows a
`ship_gate` field that carries the status, so the plan states the verdict
rather than only implying it through the note. The field is additive: a
consumer that ignores it sees the same plan as before. The browser's flat plan
view gains two scalar fields, the verdict and the reason, beside the
readiness it already reported.

The deploy lane's error vocabulary is unchanged: the refusal uses
`ForgeError::DeployInvalid` (`deploy-invalid`). The delivery lane keeps
`ForgeError::DeliveryConflict` (`delivery-conflict`). No new error variant is
introduced, so the API and the MCP surface are unchanged.

## Failure and Boundary Policy

| Situation | Outcome |
|---|---|
| The project directory has no Gate evidence | Refused, with the reason that names the project. |
| The evidence file is unreadable or malformed | Treated as absent. The owner is told to run the Gate. |
| The evidence belongs to another project | Refused. |
| The evidence uses an unsupported contract version | Refused. |
| The evidence came from a profile other than full | Refused. |
| The evidence is a rehearsal record | Refused. |
| The evidence did not pass | Refused. |
| The evidence is bound to another revision | Refused. |
| The deploy target is unsupported (`ssh`) | Refused earlier, by the target check. Unchanged. |
| The deploy request has no `--confirm` | Refused earlier, by the confirm check. Unchanged. |
| The deployment is a dry run | Not refused; nothing is contacted. |
| The delivery lane's registered revision differs from HEAD | Refused earlier, by the registration check. Unchanged. |

## Verification Oracle

```sh
# The Forge Core test suite
cargo test --workspace --all-targets -- \
  --skip rust_scaffold_builds_and_tests_with_native_toolchain \
  --skip rust_scaffold_builds_with_native_toolchain_without_forge \
  --skip declaration_is_inert_for_native_builds_present_and_absent \
  --test-threads=1

# The repository Gate
forge gate --dry-run
forge gate --timeout-secs 1800

# Strict OpenSpec validation and whitespace
openspec validate --all --strict --no-interactive
git diff --check
```

The oracle is the behaviour, not the build:

- A project with no Gate evidence produces a plan that is not ready, whose
  reason names the missing evidence.
- A deploy apply for that project refuses before the adapter is spawned, and
  the project has no delivery state and no journal row for a delivered
  artifact.
- A deploy apply for a project with fresh, passing, full-profile evidence
  proceeds exactly as before.
- A deploy apply with `--dry-run` for a project with no evidence is not
  refused.
- The delivery lane refuses for the same project for the same reason.
- A rehearsal record, a failed record, a stale record and another project's
  record are each refused.

## Implementation Plan

**The rule.** A new `src/gate/ship_gate.rs` with `ShipGateStatus` and
`evaluate_ship_gate`. The conditions are the delivery lane's conditions,
moved: supported contract, full profile, not a rehearsal, the same project,
fresh for the given revision, and a passed aggregate. The rule reuses
`crate::gate::engine::load_latest_evidence`, `evidence_freshness` and
`crate::gate::contract::is_supported_gate_contract`; it does not re-derive
them.

**The registration.** `src/gate/mod.rs` gains the module and the re-export.

**The delivery lane.** `src/delivery/handlers.rs` loses
`require_fresh_passing_gate` and calls the shared rule. Its error variant and
its position in the preflight stay the same.

**The deploy lane.** `src/deploy/engine/stages.rs`:
- `prepare_deploy` evaluates the rule for the captured revision, stores the
  status on the plan and folds it into the plan's readiness and note.
- `apply_deploy` refuses when the plan's status is not satisfied and the run
  is not a dry run, before `invoke_adapter` is reached.

**The plan contract.** `src/deploy/model.rs` gains the `ship_gate` field on
`DeployPlan`.

**The browser view.** `src/api/admin/deploy.rs` builds a flat, path-free view
of the plan for the browser. It already reported `ready`, which now follows
the rule. The view also carries the rule's verdict and its reason, so the
browser can say why a deployment is blocked. The reason contains no path: it
names the project and the command to run.

That file was at 998 physical lines before this change and the repository caps
a source file at 1000, so the view could not grow in place. The plan view, the
report view and the typed deploy error move to a new
`src/api/admin/deploy_views.rs`, which is a move, not a rewrite: the bodies are
unchanged apart from the two new fields, and the redaction discipline now
reads in one place.

**The tests.** The deploy contract fixtures that exercise a real apply need
Gate evidence. The pattern already exists: `tests/delivery_contract.rs:185`
writes a `.forge/gate/<id>/evidence.json` record. The new tests cover the
seven refusal cases, the satisfied case, the dry run, and the delivery lane's
parity with the deploy lane.

## Decision Ledger

- **Resolved:** the rule lives in the gate module, not in the delivery lane.
  It is a rule about gate evidence, and the delivery lane happened to be
  where it was first needed. The deploy lane is now a second caller of a
  module that already owns the evidence contract.
- **Resolved:** the rule returns a status, not an error. The plan needs to
  report the verdict and the apply needs to refuse on it; a status serves
  both, and each lane keeps its own error variant.
- **Resolved:** the deploy lane refuses with `deploy-invalid` and no new
  error variant is added. A new variant would touch the error contract, the
  API surface and the MCP surface for no gain.
- **Resolved:** a dry run is not gated. It contacts no target. Gating a
  rehearsal would stop the owner from previewing a deployment.
- **Resolved:** the publish lane is not touched. It syncs a source tree and
  builds on the target, so there is no local artifact for a local Gate run to
  verify, and its own contract is a different one. It is being superseded by
  the artifact-only deploy lane.
- **Resolved:** the release lane is not touched. Its checks are its own
  contract, and making the Gate mandatory there is a separate change that
  changes the release contract and every release fixture.
- **Implementation authorized (2026-10-20):** the owner directed the trunk
  repair programme and authorized this package in the approved order. The
  authorized surfaces are the files listed in the proposal.

## Requirement Traceability

| Requirement | Proposal outcome | UX flow | Design decision | Implementation boundary | Success/failure/boundary scenarios | Tasks | Verification oracle |
|---|---|---|---|---|---|---|---|
| One owner decides whether a project may be delivered | The rule moves into the gate module and is called by every lane | The two lanes refuse for the same reason | One function, one owner, a status value | `src/gate/ship_gate.rs`, `src/gate/mod.rs`, `src/delivery/handlers.rs` | The lanes refuse for the same reason; the rule has a single owner | 2.1–2.3 | Engine tests, the repository suite |
| A delivery requires fresh, passing, full-profile Gate evidence | The deploy lane adopts the rule the delivery lane already had | The refusal names the project and the command to run | The delivery lane's conditions, moved unchanged | `src/gate/ship_gate.rs` | Absent, rehearsal, stale, failed, another project, partial profile | 2.4, 3.1 | Engine tests, a real run |
| The refusal precedes every mutation | The apply step checks the rule before it spawns the adapter | The artifact does not reach the host | The check sits before the first side effect | `src/deploy/engine/stages.rs` | A refused delivery changes nothing; a rehearsal is not a delivery | 2.5, 3.2 | Engine tests, a real run |
| The deployment plan reports the delivery rule | The plan carries the status and folds it into its readiness | The owner sees the blocker at plan time | The status is a value both callers use | `src/deploy/engine/stages.rs`, `src/deploy/model.rs` | The plan names an unsatisfied rule; the plan reports a satisfied one | 2.6, 3.3 | Engine tests, a real run |
