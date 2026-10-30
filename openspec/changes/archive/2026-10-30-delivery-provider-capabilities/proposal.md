# Change: delivery-provider-capabilities

## Why

The publish-provider contract declares five operations —
`capabilities`, `preflight`, `publish`, `verify` and `rollback`
(`src/publish/providers/model.rs:73-79`, documented in
`docs/adapter-contracts/publish-provider.md`). Forge constructs two of them.
A repository-wide search for `operation: ProviderOperation::` finds only
`Publish`; the delivery lane adds `Preflight` in
`src/delivery/invoke.rs:43-48`. `Capabilities`, `Verify` and `Rollback` are
declared, documented and unreachable.

Of the three, `capabilities` is the one the contract needs first, because it
is the operation that tells Forge what a provider can honour. Without it every
verb Forge adds later — rollback is the next one — must either be assumed to
exist on every provider or be guarded by configuration Forge cannot check.

Today Forge assumes. `forge delivery stage` resolves the provider, verifies
the preflight confirmation, reserves a journal row and runs `publish`. A
provider that never implemented `publish` fails at that point, after the row
exists, with whatever error its own binary produces. The operator learns the
verb is unsupported from a failed deployment attempt rather than from the
provider's declaration.

The contract also cannot be checked because the declaration has no shape on
the response. `PublishProviderResponse` carries contract, provider,
operation_id, status, health, evidence, recovery and four optional phase
fields — and no field where a provider could say which verbs it serves.

## What Changes

- **`capabilities` becomes a reachable operation.** Forge invokes it once at
  the start of every delivery verb that reaches the publish provider, after the
  local gates and before the journal row is reserved. The probe is
  side-effect-free and carries no `queue_id`.
- **The response gains one additive field.** `capabilities`, an optional list
  of operation names. It is validated like every other additive field: bounded
  count, bounded name shape, no duplicates; a malformed list is refused. Known
  names are read; unknown names are ignored so a newer provider is not refused
  by an older Forge.
- **A declaration gates the verbs that follow.** A provider that declares a
  set is refused for a verb the set does not contain, by name, before the
  requested verb runs and before a journal row exists. A provider that answers
  with no declaration — absent field, non-zero exit, unreadable response — is
  **unknown**, which permits `preflight` and `publish` (the two verbs every
  current provider serves) and permits nothing newer.
- **The declaration is recorded.** Every delivery verb's journal detail gains
  a bounded `provider_capabilities` value so the next verb, and an operator
  reading the row, can see what the provider claimed when the verb ran.
- **The contract version does not move.** `capabilities` was already a
  declared operation and the response field is additive, so
  `forge-publish-provider/0.1.0` still means what it meant. No provider is
  required to implement anything new; a provider that does not is unknown, not
  broken.
- **No delivery verb is added.** Rollback, verify and traffic changes stay out
  of this change; this is the declaration and the gate they will be built on.

## BFS Impact Map

| Surface | Effect |
|---|---|
| `src/publish/providers/model.rs` | `PublishProviderResponse.capabilities`; `ProviderOperation` gains `as_str`, `parse` and `ALL` |
| `src/publish/providers/capabilities.rs` | new: the declaration type, its interpretation and its refusal wording |
| `src/publish/providers/engine.rs` | `validate_response` validates the new field; `probe_capabilities` runs the operation |
| `src/publish/providers/engine_tests.rs` | the test-local `as_str` impl is replaced by the real one |
| `src/delivery/capabilities.rs` | new: the delivery lane's probe and verb gate |
| `src/delivery/invoke.rs` | `operation_for(verb)`, shared with `build_request` |
| `src/delivery/handlers.rs` | preflight, stage and promote probe, gate and record |
| `docs/adapter-contracts/publish-provider.md` | the operation, the field, unknown vs declared, and the refusal |
| `docs/ci-cd-system-model.md` | the delivery seam states the gate |
| Delivery CLI/API surfaces | unchanged shape; existing tests must keep passing because unknown is permissive for preflight and publish |
| Existing providers | no change required; a provider that ignores the operation is unknown |

Persistence: no schema change. The declaration rides in the existing
`operations.detail` JSON under a new key.

Failure boundaries: the gate runs before `reserve_idempotent_operation`, so a
refused verb writes no row, starts no publish and leaves the delivery phase
untouched. A malformed claim is refused; a failed probe is not a refusal.

## Capabilities

| Capability | State | Notes |
|---|---|---|
| `delivery-provider-capabilities` | added | this package |
| `project-to-production-workflow` | read only | its requirements remain true; the gate is an additional precondition |
| `extension-point-contract` | read only | the version rule this change follows for a response field |
| `decoupled-remote-publish` | read only | its publish lane does not enter the delivery gate |

## Non-Goals

- No `rollback`, `verify` or traffic verb. This change makes the declaration
  that those verbs will be gated by; it does not add them.
- No new CLI command. `capabilities` is invoked by the lane; its answer is
  recorded on the verb it gated. An operator surface for the declaration is a
  later change if it earns one.
- No contract version bump and no required provider work.
- No caching or persistence of the declaration beyond the verb's own row. Each
  verb probes for itself, so a provider that changes its declaration between
  verbs is read as it is now.
- No change to stage/promote sharing `publish` on the wire. They remain one
  provider operation; the queue_id prefix stays the discriminator.
