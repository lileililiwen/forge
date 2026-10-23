## Implementation Boundary

| Surface | Owner | In this package |
|---|---|---|
| The deploy status projection | `forge` | Yes. One shared projection of a journal row's phase evidence. |
| The CLI's deploy status renderer | `forge` | Yes. `src/cli/release.rs`. |
| The CLI's deploy status watch document | `forge` | Yes. `src/cli/publish.rs`: the `forge deploy status --watch --queue <id>` document, which builds the same `forge-deploy-status/0.2.0` document and dropped the same four fields. |
| The API's deploy status route | `forge` | Yes. `src/api/admin/history.rs`. |
| The deploy status contract test | `forge` | Yes. `tests/deploy_status_phase_evidence_contract.rs` (new). |
| The `deploy-status-phase-evidence` capability | `forge` | Added. |
| The `forge-publish-observability-revision-containers` capability | `forge` | Modified. |
| The registry's journal row | `forge` | Read only. `src/registry/model.rs`. |
| The publish command | `forge` | Read only. `src/cli/publish.rs`. |
| The deploy apply lane | `forge` | Read only. |
| The queue status surface | `forge` | Read only. |

## The Gap

### Three readers, no rule

The phase evidence a publish row carries is read by three surfaces, and
each one dropped what the row did not carry.

The human renderer:

```rust
// src/cli/release.rs
let build_label = entry.build_status.as_deref().map(|b| format!(" build={b}")).unwrap_or_default();
let run_label = entry.run_status.as_deref().map(|r| format!(" run={r}")).unwrap_or_default();
let identity_label = entry.container_identity.as_deref().map(|c| format!(" container={c}")).unwrap_or_default();
```

The CLI's JSON document and the API's JSON document serialise the raw
journal row, whose evidence fields are skipped when they are absent:

```rust
// src/registry/model.rs
#[serde(skip_serializing_if = "Option::is_none")]
pub revision: Option<String>,
#[serde(skip_serializing_if = "Option::is_none")]
pub build_status: Option<String>,
#[serde(skip_serializing_if = "Option::is_none")]
pub run_status: Option<String>,
#[serde(skip_serializing_if = "Option::is_none")]
pub container_identity: Option<String>,
```

So the field was not `unknown`; it was not there.

### The row already knows the difference

The row distinguishes "the provider said `succeeded`" from "there was no
answer": the first is a value, the second is a `NULL` column. The
projection is where that distinction has to be turned into a statement
for a reader, and that is exactly what it skipped.

## The Rule

One shared projection, and the three surfaces read it.

```rust
// The projection of one journal row's phase evidence.
pub struct PhaseEvidence {
    pub revision: String,
    pub build_status: String,
    pub run_status: String,
    pub container_identity: String,
}
```

A value the row carries is that value; a value it does not carry is
`unknown`. The rule is one sentence, and it is the sentence the
canonical capability and the registry's own doc comment already use.

**Where the projection lives.** Beside the row it projects — the
registry owns the journal row and the meaning of its columns — so the
CLI and the API read the same function. It is a pure function over one
`OperationEntry`; it touches no database and no provider.

**What the surfaces do with it.**

- The CLI's human renderer prints the projected values in the same
  labels it already prints: `revision=`, `build=`, `run=`,
  `container=`. A missing value becomes `unknown` in the same
  position.
- The CLI's JSON document and the API's JSON document expose the four
  fields on every entry. The entry keeps every other field it has
  today; only the four evidence fields change from *absent* to
  *named*.
- The watch document (`forge deploy status --watch --queue <id>`)
  builds the same `forge-deploy-status/0.2.0` document, so it projects
  its entries the same way.

**The values a row carries are untouched.** A row with
`build_status = "succeeded"` still projects `build=succeeded`; nothing
is normalised, defaulted away or re-spelled.

**The aggregate state is untouched.** The document's `state` stays the
journal row's own state. This change is about the per-entry evidence,
which is where the missing values were being dropped.

## The Contract Test

`tests/deploy_status_phase_evidence_contract.rs` drives the real CLI
against a real journal:

- a publish row with no phase evidence → the human renderer prints
  `build=unknown`, `run=unknown`, `revision=unknown` and
  `container=unknown`, and the JSON document exposes the four fields
  as `unknown`;
- a publish row with phase evidence → the human renderer prints the
  row's own values and no `unknown` anywhere, and the JSON document
  exposes the row's own values.

The third case is the regression guard: a row that carries its evidence
must not be flattened into `unknown` by the new rule.

## Failure Boundaries

| Boundary | Behaviour |
|---|---|
| A row with no phase evidence | Every entry names the four values `unknown`. |
| A row with some phase evidence | The values it carries are projected; the ones it does not are `unknown`. |
| A row with no journal rows | The document's `entries` is empty and its `state` is `empty` — unchanged. |
| The project's directory is unreadable | The deploy status route is journal-only and never reads the directory — unchanged. |
| A non-publish row (release, gate, …) | Filtered out of the status document, as today. |

## Risks

- **The status document's entries gain four fields.** A reader that
  checked for their absence must now check for `unknown` — which is
  what the canonical requirement already demanded, and what the
  registry's doc comment already promised.
- **A row that carries no evidence now shows four `unknown` values
  instead of nothing.** That is the point: the reader can see that the
  journal never recorded a build, a run, a revision or a container.
- **Nothing is persisted and nothing is contacted.** The change is in
  the reading of rows that already exist.
