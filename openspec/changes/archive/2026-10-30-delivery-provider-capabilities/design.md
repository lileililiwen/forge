## Implementation Boundary

Owners:

| Surface | Owner | In this package |
|---|---|---|
| The provider response shape and its validation | `forge` | Yes. `src/publish/providers/model.rs`, `engine.rs`. |
| The capability declaration and its refusal wording | `forge` | Yes. `src/publish/providers/capabilities.rs`. |
| The delivery lane's probe, gate and recorded detail | `forge` | Yes. `src/delivery/`. |
| The provider contract document | `forge` | Yes. `docs/adapter-contracts/publish-provider.md`. |
| The CI/CD system model's delivery seam | `forge` | Yes. `docs/ci-cd-system-model.md`. |
| The provider binaries themselves (OpenPanel, Jenkins, siblings) | their owners | No. A provider may ignore the operation. |
| The `operations` schema | `forge` | No. The declaration rides in the existing `detail` JSON. |
| The stage/promote wire discriminator | `forge` | No. Both stay `publish`; `queue_id` stays the discriminator. |
| The delivery phase vocabulary | `forge` | No. A refused verb writes no row, so no phase moves. |

## The Reachability Problem

`ProviderOperation` has five rows. The delivery lane builds one of them for
`Preflight` and one for `Stage`, `Promote` and `Hermora` — all three share
`Publish`. Nothing constructs `Capabilities`, `Verify` or `Rollback`, and
`validate_response` has no way to read a declaration because the response type
has no field for one.

The gap is therefore two-sided: no request is sent, and no answer could be
recorded if one were. This change closes both sides for `capabilities` and
leaves `verify` and `rollback` declared and unreachable on purpose, so the next
change has one operation to wire, not a vocabulary to invent.

## The Declaration

```rust
pub enum CapabilityDeclaration {
    /// The provider answered and declared these verbs.
    Declared(BTreeSet<ProviderOperation>),
    /// The provider made no readable claim.
    Unknown { reason: String },
}

impl CapabilityDeclaration {
    pub fn supports(&self, operation: ProviderOperation) -> bool;
    pub fn summary(&self) -> String;
    pub fn detail(&self) -> serde_json::Value;
}
```

`Unknown` is not an error. There are three ways to reach it, and all three are
ordinary for a provider written before this change:

| Situation | Reading |
|---|---|
| the response carries no `capabilities` field | unknown |
| the provider exits non-zero on the probe | unknown |
| the provider answers something that is not a provider response | unknown |

`Unknown` permits `preflight` and `publish` only. Those are the two verbs every
provider in the contract has served since `0.1.0`, and refusing them would turn
a missing optional declaration into a broken deployment. It permits nothing
newer: a future `rollback` on an unknown provider is refused, not attempted.

`Declared` is the provider's own statement and is enforced literally. An empty
list is a declaration of nothing, and it refuses `stage` and `promote`; that is
the honest reading of a provider that answered the operation and named no verb.

A malformed list is neither. The response parsed as a provider response, so the
provider made a checkable claim, and a claim Forge cannot read is refused in
the same spirit as an unreadable adapter self-identification
(`extension-point-contract`). The malformed cases are bounded and enumerated:

- more than 16 names;
- a name that is empty, longer than 32 characters, or outside
  `[a-z0-9_-]`;
- the same name twice.

Unknown names inside an otherwise valid list are ignored, not refused: a
provider that declares `{"publish","preflight","preview"}` is read as
`{publish, preflight}` by this Forge, and a later Forge that knows `preview`
reads it without the provider changing.

## Probe, Gate, Record

The order inside each delivery verb is fixed:

1. local gates that already exist (ship gate, revision match, confirmation);
2. resolve the provider;
3. **probe `capabilities`**;
4. **gate the verb**;
5. reserve the operation;
6. invoke the requested verb;
7. record the response and the declaration together.

Steps 3 and 4 sit before step 5 on purpose. A refused verb must leave no
journal row, no phase and no provider side effect — the same rule the deploy
lane follows for a missing ship gate. The probe itself is a provider process,
so "no provider ran" is not true for a refusal; "the requested verb never ran"
is, and that is the claim the spec makes.

The probe is not gated and not recorded as its own operation. It has no
`queue_id`, it must be side-effect-free by contract, and its only output is the
declaration attached to the verb that asked for it.

Each verb probes for itself rather than reading the previous verb's row. A
provider can change its declaration between preflight and stage; a gate that
reads a stale declaration would enforce a claim the provider no longer makes.
The cost is one extra provider process per delivery verb, bounded by the same
timeout every provider call already has.

## Refusal Wording

One function owns the sentence, so stage and promote cannot drift:

```text
delivery stage: publish provider `openpanel` does not declare the `publish`
capability (declared: preflight); refused before the requested verb ran
```

The stage/promote verb name is interpolated, the provider id is the one that
was resolved, and the declared set is rendered from `ProviderOperation::as_str`
in a stable order. `Unknown` never produces this sentence, because it permits
the two verbs that can currently reach it.

## Compatibility

- **Wire request unchanged.** `operation: capabilities` was already legal; the
  request shape is the existing one. `queue_id` is absent, which the contract
  already allows.
- **Wire response additive.** `capabilities` is `Option<Vec<String>>` with
  `skip_serializing_if`, so a provider that never sends it is unchanged and a
  provider that sends it is readable by this Forge. `validate_response` checks
  it whenever present, so a malformed declaration cannot reach the journal.
- **Contract version stays `0.1.0`.** The operation was declared; the field is
  additive under the contract's own rule for additive fields.
- **Existing tests keep their outcomes.** The stubs in `tests/delivery_contract.rs`
  and `tests/delivery_provider_health_truth_contract.rs` answer no
  `capabilities` field, so they are unknown, and unknown permits preflight and
  publish exactly as before.

## Open Decisions

None material. Two decisions were made and are recorded rather than left
implicit:

- **probe per verb, not per chain**, for the freshness reason above;
- **unknown permits preflight and publish**, because those two are the
  contract's existing floor and a declaration is optional.

## Verification

The change is verified by:

- unit tests for the declaration's three readings, the four malformed cases and
  the per-verb permission table;
- CLI contract tests with provider stubs that declare a set, declare nothing,
  answer nothing, refuse the probe, and emit a malformed list;
- the existing delivery suites, unchanged, as the compatibility oracle;
- the repository suite and the repository Gate.
