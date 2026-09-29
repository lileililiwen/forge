# Design: project-local-remediation-plans

Status: implementation-ready planning package. No code is written by this
change.

## Implementation boundary

Repository `forge`, Rust 1.87+, existing generation and standard-pack stack.

Files to **add**:

- `src/remediation/mod.rs` — module root and re-exports.
- `src/remediation/plan.rs` — the versioned plan, action, precondition and
  ownership-receipt shapes.
- `src/remediation/apply.rs` — the apply engine (stage → verify → promote,
  rollback on failure).
- `tests/remediation_contract.rs`, `tests/remediation_cross_surface.rs`.

Files to **change** (additive only):

| File | Change |
|---|---|
| `src/lib.rs` | `pub mod remediation;` |
| `src/main.rs` | `Remediate` command tree: `scan`, `plan`, `diff`, `apply` with `--target`, `--finding`, `--confirm`, `--format` |
| `src/standard/mod.rs` | consume pack-owned CI/Compose assets as the only template source |

Do **not** touch: semantic generation, GitHub transport, deployment execution,
or the standard pack's own ownership semantics.

## Language and runtime

Rust 1.87+, `rustfmt` defaults, no new dependency (`serde`, `serde_json`,
`chrono` present). Commands: `cargo test --lib -- remediation`,
`cargo test --test remediation_contract`, `cargo clippy --all-targets -- -D
warnings`.

## Ownership and shared code

Forge owns the plan and apply engine. The package **extends** `src/standard/`
for versioned owned assets and **adopts** `src/generate/`'s
stage/collision/cleanup discipline rather than re-implementing atomic writes. It
**adapts** `src/spec/mod.rs` routing only for finding selection; semantic
repairs are deferred to `project-semantic-description-review`. No sibling
repository is written implicitly.

## Behavioral model

`forge-remediation-plan/0.1.0`.

```rust
pub enum ActionKind { WriteOwnedFile, RefreshManifestField, InstallStandardAsset, AddDocLink }
pub enum PlanState { Proposed, Confirmed, Applied, Failed, RolledBack }
pub enum RemediationClass { Automatic, Semantic, Manual }
```

| Rule | Behaviour |
|---|---|
| Selection | only findings whose `remediation_class == automatic` become actions |
| Ownership | an action may write only a path Forge owns (declared in the pack receipt / `.project.json`); an unowned collision refuses |
| Idempotency | a plan applied twice with unchanged inputs is a no-op that reports `already applied` |
| Preconditions | target revision and owned-file digests are bound into the plan; a mismatch refuses before any write |
| Apply | stage to a temp path, verify digests, then promote; on any failure restore prior bytes |

## Contract and compatibility

Plan, diff and outcome are versioned JSON documents; the table is human-facing.
Errors are typed: `remediation-invalid` (malformed plan, missing asset,
incompatible profile), `remediation-conflict` (unowned collision, stale
revision). A `--dry-run`/`plan`/`diff` never writes.

## Failure and boundary policy

| Case | Result |
|---|---|
| No automatic finding selected | empty plan, exit 0, nothing written |
| Semantic/manual finding selected | refused with `remediation-invalid` naming the class; routed elsewhere |
| Missing standard asset | `remediation-invalid`, names the pack/version |
| Unowned file collision | `remediation-conflict`, no write, prior bytes preserved |
| Stale target revision | `remediation-conflict`, plan refused before apply |
| Partial write failure | prior bytes restored; `Failed`/`RolledBack` outcome recorded with rollback info |
| Secret generation | never; a needed secret is referenced, not created or printed |

## Verification oracle

`tests/remediation_contract.rs`: plan/diff/apply, idempotent re-apply, digest
and revision preconditions, unowned-collision refusal, rollback-on-failure,
standard-asset selection by profile; `tests/remediation_cross_surface.rs`:
`plan`/`diff` write nothing, no secret is generated or disclosed, no remote
mutation, and the registry journal records intent and outcome without replacing
project source. No checkbox without its named test and captured output.

## Decision ledger

- No blind overwrite: ownership must be provable before a write, and the
  receipt identifies the owner.
- No fake CI/Compose: an asset must come from a selected standard pack version.
- Builds/tests/deployments are never run as a substitute for evidence.
