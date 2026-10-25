# Change: publish-container-identity-evidence

## Why

The publish lane invented a container for a deployment that never
happened.

```
$ forge --registry /tmp/pub/registry.db publish --folder /tmp/pub/demo \
    --provider legacy --revision 0123456789abcdef0123456789abcdef01234567
# the provider reported: status=failed, health=unhealthy, no container_identity
$ sqlite3 /tmp/pub/registry.db \
    "SELECT state, container_identity FROM operations WHERE kind = 'publish'"
failed|forge-demo-0123456789ab
```

The row said `failed` and carried a container identity for a publish
that failed. Nothing was deployed; the identity was composed by Forge
from the revision alone.

The composition was unconditional:

```rust
// src/cli/publish.rs, before
let container_identity = response.container_identity.clone().unwrap_or_else(|| {
    forge::publish::providers::compose_project_name(project_id, &phase_revision)
});
```

The canonical capability already draws the line, and it draws it the
other way from the code:

> **Scenario: Provider without container_identity synthesizes it from
> revision** — **WHEN** a provider returns a **successful** response
> without a `container_identity` field — **THEN** Forge persists the row
> with the canonical `forge-<project>-<sha12>` identity computed from
> the recorded revision so legacy providers are not silently absent from
> observability.

The synthesis is authorised for a successful response only. The code
composes one for every response.

The provider contract's own field says the same:

```rust
// src/publish/providers/model.rs
/// Bounded container / Compose project identity the provider
/// actually deployed ... absent when the provider cannot prove the
/// identity.
pub container_identity: Option<String>,
```

And the fleet lane already reads that rule, in its own words:

```rust
// src/cli/fleet.rs
// The container identity is the provider's own.
// The lane never composes one: a run that published nothing has no container.
```

So the fleet lane and the manual lanes disagree: the fleet lane never
composes, the manual lanes always do. The manual lanes are the ones
that claim a container nobody reported.

This is the programme's rule class: Forge states something its own
gathered evidence does not support.

## What Changes

- **One rule names the container identity a response proves.** The
  provider's own identity is the truth and is always kept. A legacy
  provider that published but reported no identity gets the canonical
  identity composed from the recorded revision. A response that
  published nothing carries no identity at all.
- **Every publish surface reads that rule.** The CLI publish lane, the
  sibling-cwd publish lane, the operator route and the admin route all
  read it, so no surface composes a container of its own.

## BFS Impact Map

| Surface | Owner | In this package | Notes |
|---|---|---|---|
| The CLI publish lane | `forge` | Yes | `src/cli/publish.rs`: the row's identity comes from the rule. |
| The sibling-cwd publish lane | `forge` | Yes | `src/cli/publish.rs`: the row's identity comes from the rule. |
| The operator publish route | `forge` | Yes | `src/api/handlers_core.rs`: the row and the response read the rule. |
| The admin publish route | `forge` | Yes | `src/api/admin/publish.rs`: the row and the response read the rule. |
| The publish provider contract | `forge` | Yes | `src/publish/providers/mod.rs`: the rule and its vocabulary. |
| The fleet publish lane | `forge` | Read only | Unchanged: it already keeps only the provider's own identity, which the rule agrees with. |
| The delivery lane | `forge` | Read only | Unchanged: it writes no container identity at all. |
| The status projection | `forge` | Read only | Unchanged: a row without an identity renders as `unknown`. |
| The registry | `forge` | Read only | No schema change; one row's `container_identity` column becomes `NULL`. |

| Concern | Result |
|---|---|
| Requirements | `requirement.md` §33 owns the release and publish workflow; the `forge-publish-observability-revision-containers` capability owns the container identity. |
| Contracts | No field is removed or renamed. The `container_identity` field of a publish response carries `null` when the provider published nothing; the projection renders `unknown`. |
| Callers | `forge publish`, `forge publish --cwd`, `POST /v1/projects/{id}/publish`, `POST /v1/admin/projects/{id}/publish`. |
| Persistence | The publish journal row's `container_identity` column is `NULL` for a response that published nothing. |
| Integrations | None. No adapter, provider or repository is touched. |
| Failures | No new error code. A failed publish still exits the way it did and still journals the provider's own status. |
| Tests | A new contract test `tests/publish_container_identity_evidence_contract.rs`; the existing publish tests stay green. |
| Dependencies | None added. |

## Capabilities

| Capability | Owner | Type | State | Notes |
|---|---|---|---|---|
| `forge-publish-observability-revision-containers` | forge | Rust | modified | the identity requirement states that a response that published nothing carries no container. |

No new capability: the rule already exists and is already promoted. The
code did not implement the line the capability drew.

## Non-goals

- **The synthesis for a successful response is unchanged.** A legacy
  provider that published and reported no identity still gets the
  canonical `forge-<project>-<sha12>` identity, so it is not silently
  absent from observability.
- **The provider's own identity is always kept.** A provider that
  reports an identity keeps it, whatever its status says.
- **The fleet lane is unchanged.** It never composed an identity, which
  is what the rule says.
- **The status projection is unchanged.** A row without an identity
  renders as `unknown`, never as a verified success.
- **No journal row is rewritten.** Rows written before this change stay
  as they are.
