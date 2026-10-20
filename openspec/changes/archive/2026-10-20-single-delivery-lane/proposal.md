## Why

The same action had two opposite evidence policies.

`forge delivery` refuses to ship without fresh, passing, full-profile Forge
Gate evidence bound to the revision it is about to deliver
(`src/delivery/handlers.rs:148-179`, called from the preflight at
`src/delivery/handlers.rs:200`). `forge deploy` applies a built artifact to a
target — the Docker Compose host, the SSH host, the Jenkins job, the Mac
runtime — with no evidence at all: `apply_deploy` (`src/deploy/engine/stages.rs:86`)
validates the request, checks `--confirm`, prepares the plan and then spawns
the adapter (`src/deploy/engine/stages.rs:103-109`, the process spawn at
`src/deploy/engine/stages.rs:326`).

So the owner who verifies a project and then deploys it gets a refusal from
one lane and a successful delivery from the other, and the one that actually
puts a build on a host is the one that checks nothing. `src/publish/` is the
same story with a third vocabulary: it has no reference to the gate anywhere.

A trunk is a spine with one rule per step. "Deliver" had two rules.

## What Changes

- **One rule, one owner.** A new `src/gate/ship_gate.rs` owns the decision
  "may this project deliver this revision". It is the only implementation of
  that decision in the repository.
- **The delivery lane stops owning it.** Its private copy is deleted and the
  shared rule is called instead. Its behaviour and its error text stay the
  same.
- **The deploy lane adopts it.** A deploy that is not a dry run evaluates the
  shared rule before it spawns the adapter, and refuses with the deploy lane's
  error vocabulary when the rule is not satisfied.
- **The plan reports it.** A deployment plan states whether the rule is
  satisfied and is not ready when it is not, so the owner sees the blocker
  before the apply step.
- **Nothing else moves.** The gate's own contract, the deploy adapter
  contract, the delivery provider contract, the release lane, the publish lane
  and every sibling module are untouched.

## Package Boundary and Split Assessment

| Package | Module | Platform/Stack | Status | Decision |
|---|---|---|---|---|
| `single-delivery-lane` | `forge` | Rust (Forge Core) | implemented | This package. One delivery rule with one owner. |
| `extension-point-contract` | `forge` | Rust (Forge Core) | planned | Later. One descriptor and one contract for every extension kind, including the deploy executor. |
| `release-gate-mandatory` | `forge` | Rust (Forge Core) | planned | Later. The release lane binds doctor/test/DriftWatch/gate evidence through its own check model, and the manifest chooses which kinds apply. Making the Gate mandatory in the release path changes the release contract. |
| `deploy-target-ssh` | `forge` | Rust (Forge Core) | planned | Later. A new adapter behind the existing deploy-executor contract. |

One package, one owner. The delivery rule belongs to the gate module because
it is a rule about gate evidence; the deploy and delivery lanes are its
callers.

## Sibling and Shared Architecture Reconnaissance

| Neighbor | Shared surface | Boundary decision |
|---|---|---|
| `forge/modules/workspace-governance` | The gate runtime and the DriftWatch evidence the `driftwatch` check reads. | Read only. The rule reads persisted evidence; it does not run a gate. |
| `forge/modules/dotnet-platform-libs` | Nothing in this change. | Not edited. |
| `forge/modules/rust-platform-libs` | Nothing in this change. | Not edited. |
| `forge/modules/identity-providers` | Nothing in this change. | Not edited. |
| The `platform-contracts` contract set | The gate evidence contract and the deploy executor contract are local to this repository. | Not edited. |

## User Experience and Interface Impact

Before:

```
$ forge deploy apply ./my-app --target compose --confirm
apply: delivered
```

The artifact reached the host. No evidence was consulted, because none was
required.

After:

```
$ forge deploy apply ./my-app --target compose --confirm
error[deploy-invalid]: deploy invalid: Forge Gate evidence is absent for `my-app`; run `forge gate --profile full` from the project directory and retry
```

Nothing reached the host, no delivery state was written and no operation row
records a delivered artifact.

The plan tells the owner first:

```
$ forge deploy plan ./my-app --target compose
...
ready: no
note: deploy plan is not ready: <the refusal reason>
```

A project that has run `forge gate --profile full` and committed nothing since
is unaffected: the plan is ready and the apply proceeds exactly as before.

## BFS Impact Map

- **`src/gate/ship_gate.rs`** (new) — the shared rule and its refusal type.
- **`src/gate/mod.rs`** — the module registration and the re-export.
- **`src/delivery/handlers.rs`** — the private copy is deleted; the shared rule
  is called. Behaviour and error text unchanged.
- **`src/deploy/engine/stages.rs`** — the deploy apply step evaluates the rule
  before the first mutation.
- **`src/deploy/engine/model.rs` or `src/deploy/prepare.rs`** — the plan
  reports the rule.
- **`src/api/admin/deploy.rs`** — the flat, path-free plan view the browser
  reads carries the rule's verdict and its reason, beside the readiness it
  already reported.
- **`src/api/admin/deploy_views.rs`** (new) — the path-free plan and report
  views move here. The route file was at 998 physical lines before this
  change and the repository caps a source file at 1000, so the views could
  not grow in place.
- **Test surfaces** — the deploy contract fixtures that exercise a real apply
  need Gate evidence; the delivery fixtures already have it.
- **Unchanged on purpose** — the gate's own contract and evidence format, the
  deploy adapter contract, the delivery provider contract, the release lane,
  the publish lane, the `forge.yaml` contract, and every sibling module.

## Capabilities

- New capability: `single-delivery-lane`.
- No existing capability changes. The `adapter-deployment`, `forge-native-gate`
  and delivery contracts keep their current requirements; this capability adds
  the rule the deploy lane was missing.

## Non-goals

- No change to the Gate's own contract, its evidence format or its command
  surface.
- No change to the deploy adapter contract, the target kinds or the health
  observation.
- No change to the delivery provider contract or the delivery verbs.
- No change to the release lane. Its checks are its own contract.
- No change to the publish lane. It syncs a source tree and builds on the
  target, so there is no local artifact for a local Gate run to verify, and it
  is being superseded by the artifact-only deploy lane.
- No new CLI verb and no change to any verb's shape.
- No new error variant. The deploy lane refuses with `deploy-invalid` and the
  delivery lane keeps `delivery-conflict`.
- No change to any sibling module.
