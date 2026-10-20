## Why

A release could pass with nothing behind it.

`forge release prepare` and `forge release apply` both judged readiness with
the same expression (`src/release/engine/prepare.rs:82`, `src/release/engine/apply.rs:79`):

```rust
checks.iter().filter(|c| c.applicable).all(|c| c.status == CHECK_PASS)
```

A project whose manifest declares no `release.checks` has no applicable check,
so the filter yields an empty set, and `all()` on an empty set is `true`. The
plan then reported `ready: yes` with "0 check(s) passed", and the apply step
walked the commit, tag, push and publication stages on a release that nothing
had verified.

`release.checks` defaults to empty (`src/core/manifest.rs:421`), so every
manifest written before the `project-manifest-completeness` change, and every
hand-written manifest that omits the list, is in this state.

The release contract says the plan "binds ... doctor/test/DriftWatch evidence
before release side effects" and that "a required check fails ... release
execution blocks before tag or publication". A release with no required check
satisfies that wording without ever producing evidence. The wording has to say
what happens when the set of required checks is empty.

## What Changes

- **One rule, one owner.** A new readiness judgement in the release engine
  decides whether a release is ready. It is a single function used by both the
  prepare step and the apply step, so the two cannot drift apart.
- **Ready needs evidence.** A release is ready only when at least one check is
  applicable and every applicable check passes. The empty set is not a pass; it
  is the absence of evidence.
- **The plan says so.** A plan with no applicable check reports `ready: no` and
  names the missing check. It never reports a passing check count.
- **The apply step refuses.** The apply step re-evaluates the captured checks
  and refuses with a release-check failure before the commit stage, so a vacuous
  plan cannot reach a tag.
- **The refusal is actionable.** The reason names the four check kinds
  `release.checks` accepts, so the owner can declare one and re-run.
- **Nothing else moves.** The check kinds, the check semantics, the stage list,
  the manifest contract and the release error codes are untouched.

## Package Boundary and Split Assessment

| Package | Module | Platform/Stack | Status | Decision |
|---|---|---|---|---|
| `release-evidence-non-vacuous` | `forge` | Rust (Forge Core) | implemented | This package. The readiness judgement of the release engine. |
| `single-delivery-lane` | `forge` | Rust (Forge Core) | planned | Later. Folds the four delivery lanes into one and makes the gate mandatory. `forge publish` has no release check coupling at all today, so the publish lane is a separate hole. |
| `project-changelog-bootstrap` | `forge` | Rust (Forge Core) | planned | Later. The scaffold writes no changelog, so a fresh project still needs one before a release can be prepared. |

One package, one owner. The release engine owns the readiness judgement; the
sibling modules own nothing here.

## Sibling and Shared Architecture Reconnaissance

| Neighbor | Shared surface | Boundary decision |
|---|---|---|
| `forge/modules/workspace-governance` | The gate and DriftWatch evidence the `driftwatch` and `gate` checks read. | Read only. Those checks keep their current semantics; this change decides what happens when no check is configured. |
| `forge/modules/dotnet-platform-libs` | Nothing in this change. | Not edited. |
| `forge/modules/rust-platform-libs` | Nothing in this change. | Not edited. |
| `forge/modules/identity-providers` | Nothing in this change. | Not edited. |
| The `platform-contracts` contract set | The release contract is local to this repository. | Not edited. |

## User Experience and Interface Impact

Before:

```
$ forge release prepare --version 1.0.0 --dry-run
ready: yes
checks:
  - doctor disabled applicable=false revision=abc123: check `doctor` is not configured; release does not require it
  - test disabled applicable=false revision=abc123: check `test` is not configured; release does not require it
  - driftwatch disabled applicable=false revision=abc123: check `driftwatch` is not configured; release does not require it
  - gate disabled applicable=false revision=abc123: check `gate` is not configured; release does not require it
summary: release plan ready for `shop`: 0 check(s) passed at revision `abc123`
```

After:

```
$ forge release prepare --version 1.0.0 --dry-run
ready: no
checks:
  - doctor disabled applicable=false revision=abc123: check `doctor` is not configured; release does not require it
  - test disabled applicable=false revision=abc123: check `test` is not configured; release does not require it
  - driftwatch disabled applicable=false revision=abc123: check `driftwatch` is not configured; release does not require it
  - gate disabled applicable=false revision=abc123: check `gate` is not configured; release does not require it
summary: release plan not ready: no applicable release check; declare at least one of doctor, test, driftwatch, gate in release.checks; release execution blocks before tag or publication
```

`forge release apply` on the same project refuses with
`error[release-check-failed]` and creates no tag.

A project that declares a check is unaffected: the plan is ready when that
check passes, and refused when it does not.

## BFS Impact Map

- **`src/release/engine/prepare.rs`** — the readiness judgement, the known check
  kinds, the prepare call site and the plan note.
- **`src/release/engine/apply.rs`** — the apply call site uses the same
  judgement and refuses with the same reason.
- **`src/release/engine/engine_tests.rs`** — the tests that pin the rule.
- **`tests/forge_web_project_release_contract.rs`** — its fixture declared no
  release check and relied on the vacuous pass, so it is given a real check
  pair. Its assertions are unchanged.
- **`src/procedure/catalog.rs`** — the `prepare-release` procedure description
  says the plan refuses when a check is failing or stale; it now also says the
  plan refuses when no check is applicable.
- **Unchanged on purpose** — the release error codes, the check kinds and their
  statuses, the stage list, the manifest contract, the `forge.yaml` schema
  version, the gate and DriftWatch adapters, and every sibling module.

## Capabilities

- New capability: `release-evidence-non-vacuous`.
- No existing capability changes. The `release-publishing` contract keeps its
  current requirements; this capability adds the missing case the contract
  left open.

## Non-goals

- No new check kind. The four kinds stay `doctor`, `test`, `driftwatch` and
  `gate`.
- No change to what a check does, to its statuses, or to the evidence it
  produces.
- No change to the release stages, the stage order, the identity rules or the
  retry semantics.
- No change to the manifest contract. A manifest with an empty `release.checks`
  list still parses; the release it describes is refused.
- No change to `forge publish`. The publish lane has no release check coupling
  today; that is the `single-delivery-lane` package.
- No change to the gate. Making the gate mandatory for delivery is the
  `single-delivery-lane` package.
- No new CLI verb and no change to any verb's shape.
- No change to any sibling module.
