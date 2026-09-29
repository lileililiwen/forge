# Design: project-local-remediation-plans

Status: implementation-ready package; implementation authorized by the user.

## Implementation boundary

Repository `forge`, Rust 1.87+, existing generation and standard-pack stack.

Files to **add**:

- `src/remediation/mod.rs` — the versioned plan, action, precondition, scan,
  diff and apply service.
- `tests/remediation_contract.rs`, `tests/remediation_cross_surface.rs`.

Files to **change** (additive only):

| File | Change |
|---|---|
| `src/lib.rs` | `pub mod remediation;` |
| `src/main.rs` | `Remediate` command tree: `scan`, `plan`, `diff`, `apply` with shared `--target`, `--finding`, `--pack`, optional `--plan`, `--confirm`, and global `--format`; previews never open the registry |
| `src/standard/mod.rs` | expose the existing pack renderer/receipt semantics to the remediation builder without changing pack ownership |
| `src/core/mod.rs` | add `remediation-invalid`, `remediation-conflict`, and `remediation-apply-failed` typed errors |

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

The serialized action names are `write_owned_file`, `refresh_manifest_field`,
`install_standard_asset`, `add_doc_link`, and `refresh_ownership_receipt`;
states are `proposed` and the apply outcome is `applied`. The implementation
supports standard asset installation and receipt refresh for automatic
missing-CI findings. Other closed action variants remain rejected. The full
selected pack snapshot is materialized so the receipt cannot claim ownership
of files the plan did not write.

`RemediationPlan` contains `contract`, `plan_id`, `state`, `target` (absolute
path, project id and profile), `finding_id`, `finding_class`, `pack` (id,
version and asset digest), `actions`, and `preconditions`. Each action contains
`kind`, repository-relative `path`, `owner`, `expected_digest`, `content`, and
`source`. Preconditions contain the optional Git revision and one expected
digest/ownership tuple per affected path. Content comes only from the selected
built-in standard pack renderer; no finding or provider value is copied into
file content, and no credential value is generated or persisted.

| Rule | Behaviour |
|---|---|
| Selection | only findings whose `remediation_class == automatic` become actions |
| Ownership | an action may write only a declared pack path under `.standard/`; an existing path is writable only when the current receipt proves Forge ownership and its digest matches; symlink escapes and unowned collisions refuse |
| Idempotency | a plan applied twice with unchanged inputs is a no-op that reports `already applied` |
| Preconditions | target revision and owned-file digests are bound into the plan; a mismatch refuses before any write |
| Apply | re-read and re-render the selected pack, validate target/profile/pack/revision/digests and confirmation, stage every file in a target-local temporary directory, verify staged digests, promote in deterministic path order, restore captured prior bytes on failure, and journal restored/failed rollback paths |

## Contract and compatibility

Plan, diff and outcome are versioned JSON documents; the table is human-facing.
Errors are typed: `remediation-invalid` (malformed plan, missing asset,
incompatible profile, unsupported finding class), `remediation-conflict`
(unowned collision, stale revision or digest), and `remediation-apply-failed`
(promotion or rollback failure). A `scan`/`plan`/`diff` never writes.

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
