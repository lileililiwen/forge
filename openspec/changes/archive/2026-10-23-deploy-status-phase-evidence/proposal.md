# Change: deploy-status-phase-evidence

## Why

`forge deploy status` answers "did the build, the run, or both fail?"
from the persisted journal. The canonical capability says exactly what
the projection must show:

> #### Scenario: Status projection surfaces revision and identity
>
> - **WHEN** `forge deploy status --queue <id>` is queried for a
>   completed fleet
> - **THEN** every entry exposes its `revision`, `build_status`,
>   `run_status`, and `container_identity` fields through both the JSON
>   document and the human renderer; missing values render as
>   `unknown`, never as a verified success

The projection dropped every field the row did not carry:

```rust
// src/cli/release.rs
let build_label = entry
    .build_status
    .as_deref()
    .map(|b| format!(" build={b}"))
    .unwrap_or_default();
let run_label = entry
    .run_status
    .as_deref()
    .map(|r| format!(" run={r}"))
    .unwrap_or_default();
```

and the JSON document serialised the raw journal row, whose four
evidence fields carry `#[serde(skip_serializing_if = "Option::is_none")]`
— so the fields were **absent**, not named.

A row with no phase evidence therefore rendered exactly like a fully
verified publish. Driven on this host, against a journal row that
carried no phase evidence:

```text
deploy status: project=p1
  #1 p1 publish done
```

and in the JSON document:

```json
{"detail":"shipped 1.2.3","finished_at":"2026-10-23T03:28:12.773582887+00:00",
 "kind":"publish","op_id":1,"project_id":"p1",
 "started_at":"2026-10-23T03:28:12.773582887+00:00","state":"done"}
```

**Nothing named what the row did not prove.** The reader sees a
completed publish with no build, no run, no revision and no container
identity — and no sign that any of it was ever established. The
canonical requirement calls that a verified success, and forbids it.

**The registry's own contract already said what the projection should
do.** The method that persists the row documents it:

```rust
// src/registry/model.rs
/// evidence still get their rows persisted with `NULL` values; the
/// `forge deploy status` projection renders the missing values as
/// `unknown` instead of a verified success.
```

The row is persisted with `NULL` — that part works. The projection
never rendered the `NULL` as `unknown`.

**Three surfaces dropped it the same way.** The CLI's human renderer,
the CLI's JSON document and the API's JSON document
(`GET /v1/admin/projects/{id}/deploy/status`) each serialise or format
the raw row, so each one dropped the same four fields. One rule, three
readers — and no rule at all.

## What Changes

- **One shared projection.** A deploy status entry is projected by one
  function that always exposes `revision`, `build_status`, `run_status`
  and `container_identity`, naming a value the row does not carry
  `unknown`.
- **Every surface reads it.** The CLI's human renderer, the CLI's JSON
  document and the API's JSON document stop dropping the fields.
- **The claims are pinned.** A repository contract check drives the
  real CLI against a journal row with and without phase evidence.

## BFS Impact Map

| Surface | Owner | In this package | Notes |
|---|---|---|---|
| The deploy status projection | `forge` | Yes | One shared projection of a journal row's phase evidence. |
| The CLI's deploy status renderer | `forge` | Yes | `src/cli/release.rs`: the human table and the JSON document. |
| The API's deploy status route | `forge` | Yes | `src/api/admin/history.rs`: `GET /v1/admin/projects/{id}/deploy/status`. |
| The registry's journal row | `forge` | Read only | `OperationEntry` and `record_publish_phase` are unchanged; the row is already persisted with `NULL`. |
| The deploy status contract test | `forge` | Yes | `tests/deploy_status_phase_evidence_contract.rs` (new). |
| The `deploy-status-phase-evidence` capability | `forge` | Added | this package |
| The `forge-publish-observability-revision-containers` capability | `forge` | Modified | The status projection requirement names the rule. |
| The publish command | `forge` | Read only | It persists the evidence; it does not project it. |
| The deploy apply lane | `forge` | Read only | Unchanged. |
| The queue status surface | `forge` | Read only | Unchanged. |
| The `forge-deploy-status/0.2.0` contract version | `forge` | Read only | The document keeps its fields; the four evidence fields become always present. |

| Concern | Result |
|---|---|
| Requirements | `requirement.md` §27 (distribution) and §30 (deployment) own the publish and deploy lanes; the observability capability owns the phase evidence. |
| Contracts | The status document's entries gain four always-present fields (`revision`, `build_status`, `run_status`, `container_identity`); a row that carried none now reads `unknown` there instead of omitting them. No field is removed or renamed, and the contract version is unchanged. |
| Callers | `forge deploy status` (human and JSON), `forge deploy status --watch`, `GET /v1/admin/projects/{id}/deploy/status`, the portal's release/deploy history section. |
| Persistence | Nothing new is persisted. The row is already stored with `NULL`; only the reading changes. |
| Integrations | None. No provider is contacted. |
| Failures | No new error code. |
| Tests | A new `tests/deploy_status_phase_evidence_contract.rs`. |
| Dependencies | None added. |
| Concerns | Quality: the projection shows what the journal proves and names what it does not. Compatibility: the document gains four always-present fields; a reader that checked for their absence must now check for `unknown`, which is what the canonical requirement already demanded. Security: no new surface; the values are the row's own. |

## Capabilities

| Capability | Owner | Type | State | Notes |
|---|---|---|---|---|
| `deploy-status-phase-evidence` | forge | Rust | added | this package |
| `forge-publish-observability-revision-containers` | forge | Rust | modified | the status projection requirement names the rule |

## Non-Goals

- **The journal row is unchanged.** The row is already persisted with
  `NULL` for a value it does not carry; the change is in the reading.
- **The publish and deploy lanes are unchanged.** They persist the
  evidence; they do not project it.
- **The values the row carries are unchanged.** A row with
  `build=succeeded` still renders `build=succeeded`.
- **The aggregate state is unchanged.** The document's `state` stays
  the journal row's own state; only the per-entry evidence is
  projected.
- **The contract version is unchanged.**
