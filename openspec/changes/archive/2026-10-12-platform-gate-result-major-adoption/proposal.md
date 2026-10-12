# Proposal: adopt `platform.gate-result` 1.0.0

## Why

`platform-contracts` bumped `platform.gate-result` from `0.1.0` to `1.0.0`
(`gate-result-major-review-state`, commits `3cfd310`, `5e8a6ca`). The
`result` and `checks[].status` enums grew symmetrically from four to six
values:

```
passed | failed | errored | skipped | review_required | unbound
```

`schemas/registry.json` now records `current_version: "1.0.0"` and
`supported_major_versions: [0, 1]` for this family, with a `narrowing`
block in state `declared` that will drop major 0 once every declaring
consumer has migrated. `driftwatchdog` has already adopted 1.0.0
(`platform-gate-result-major-adoption`, commits `2221f63`, `cf823dc`) and
was verified live to emit `contract: "platform.gate-result/1.0.0"`.

Forge is itself a **producer** of this contract and has not migrated.
Measured drift on `main` (`scripts/contract-parity.sh`, exit 1):

- `contracts/schemas/gate-result.schema.json` is at sha256 `55750b59…`
  (the 0.1.0 four-value enum); the source publishes `49e7583c…`.
- `contracts/registry.json` is at sha256 `090a97bb…`; the source publishes
  `963272540…`. The whole-file delta is exactly one record —
  `contracts["platform.gate-result"]` — gaining
  `supported_major_versions: [0, 1]`, `current_version: "1.0.0"`, a
  rewritten description and the `narrowing` block. No other family's
  record differs.
- Family digest `platform.gate-result` fails against the source's own
  record for the same reason.

Three separate defects follow from staying on 0.1.0:

1. **A dishonest mapping (`src/contract/mod.rs`).** Under the four-value
   enum `emit_gate_result` had two coercions that assert outcomes Forge
   never observed. `GateCheckState::Unresolved` → `errored` says the check
   errored, but `GateCheckState` exists *precisely so Forge does not
   assert an outcome it did not observe* ("Unknown or unexpected state
   strings become `unresolved` — never `pass`", `src/gate/model.rs:169`).
   `GateAggregate::Unknown` → `errored` repeats the same defect at the
   aggregate level, where "unclassifiable document" is Forge's own
   documented meaning. A third coercion, `GateAggregate::Failed` →
   `errored`, says a check "could not be evaluated to a verdict" when
   Forge's own documentation of `Failed` is "a non-blocking failure" —
   i.e. an attempted check *did* report a failure. 1.0.0's
   `review_required` ("a human decision is still open and no one has
   decided the outcome") is the honest home for both unclassifiable
   states, and 1.0.0's `failed` ("an attempted check reported a failure")
   is the honest home for `Failed`.
2. **A lying version declaration.** `contract_discriminator` hardcodes
   `format!("{family}/0.1.0")` for every family, so after adopting the
   new vocabulary Forge would emit a document *labelled* `0.1.0` whose
   `result` value is not in the 0.1.0 enum. A consumer could not tell a
   0.1.0 document from a 1.0.0 one. `forge contract inspect` compounds it
   by printing `contract: {family}/0.1.0` from a literal
   (`src/cli/functions_13.rs:765`). `validate_envelope` closes the door in
   the other direction: `if !version.starts_with("0.")` refuses
   `platform.gate-result/1.0.0`, so `forge contract validate` rejects the
   sibling's own 1.0.0 document.
3. **A stale vendored schema.** The mirror is pinned to the 0.1.0 bytes
   and fails the offline parity gate, so every Forge release ships a
   contract mirror the source has already superseded.

## What Changes

- **Corrected, total status mapping (Part 1).** `map_gate_aggregate` is
  replaced by `map_gate_result(&GateEvidence)`, which renders the root
  `result`, and the inline per-check `match` in `emit_gate_result` is
  replaced by the tested `map_gate_check_state(GateCheckState)`. Both
  keep the `Result<&'static str, ContractUnmappable>` refusal shape, and
  no row asserts an outcome Forge did not observe. `GateAggregate::Blocked`
  is decomposed on the observed check rows rather than mapped flat, because
  Forge's aggregate is a superposition of two different sibling claims
  (see `design.md` §3c). The analysis of where a refusal *would* be
  required — and why no gate-result row needs one — is recorded in
  `design.md` §3a-bis rather than left implicit.
- **Registered version declaration (Part 2).** A new
  `PLATFORM_FAMILY_VERSIONS` table registers the wire version Forge emits
  per family (`platform.gate-result` → `1.0.0`; every other family stays
  `0.1.0`). `contract_discriminator` reads it instead of a literal,
  `forge contract inspect` prints the registered version, and
  `validate_envelope` accepts any major the family's
  `supported_major_versions` declares while still refusing an undeclared
  one. No literal `"0.1.0"` survives in a version position.
- **Re-vendored schema (Part 3).** `contracts/` is re-synced through
  `scripts/sync-contracts.mjs` — the only sanctioned writer — and
  `scripts/contract-parity.sh` is run against the resolved source. No
  vendored byte and no manifest record is hand-edited.
- **Canonical spec update.** `platform-contract-consumption` keeps its
  refusing-mapping requirement and gains an explicit
  version-declaration requirement plus scenarios pinning the new
  vocabulary, the refusal, and the 1.0.0 discriminator.

## BFS Impact Map

- **Requirements:** `platform-contract-consumption` — "Refusing status
  mapping" (scenario rewritten: the old `passed` / `failed` / `errored`
  triple is no longer what Forge emits), "Read-only contract projection
  surface" (the emit scenario names `platform.gate-result/0.1.0`),
  plus a new "Declared contract version per family" requirement.
  `gate-runtime-evidence` is deliberately **not** amended: it describes
  Forge's own evidence record, whose shape is unchanged.
- **Concepts/vocabulary:** `GateAggregate`, `GateCheckState`,
  `GateEvidence`, `classify_aggregate` are **unchanged** — they are
  Forge's own vocabulary and remain correct. Only the projection onto the
  platform contract changes.
- **Modules:** `src/contract/mod.rs` (mapping, version registry,
  discriminator, validator), `src/contract/contract_tests.rs` (unit
  tests), `src/cli/functions_13.rs` (`forge contract inspect` human
  line). No new module, no new route, no new command.
- **Contracts/data:** `contracts/schemas/gate-result.schema.json`,
  `contracts/registry.json` and `contracts/manifest.json` (source
  revision + per-file sha256 + `synced_at`) are rewritten by the sync
  script. `GateEvidence` JSON, the journal, `.forge/**`, generated
  project trees, MCP tools, API route table and portal sections are all
  untouched.
- **Callers:** `forge contract emit gate-result <target>`
  (`src/cli/functions_13.rs:779`) and the read-only admin route
  `GET /v1/admin/contracts/emit?family=platform.gate-result`
  (`src/api/admin/assurance.rs:742`) — both go through
  `emit_gate_result`, so both inherit the change with no edit.
  `forge contract validate <file>` and `forge contract inspect <family>`
  change behaviour; `forge contract list` does not.
- **Persistence:** none. `emit_gate_result` reads the latest persisted
  evidence and writes nothing; the change adds no field to the evidence
  record and no migration.
- **Integrations:** `driftwatchdog` remains the gate runtime and the
  producer of the *evidence* document Forge consumes; nothing in
  `src/gate/` changes, so the consumption path (`forge gate`,
  `--dry-run`, `GateEvidence` normalisation from `gate --format json`) is
  byte-identical. `platform-contracts` is consumed read-only through
  `scripts/sync-contracts.mjs` / `scripts/contract-parity.sh`.
- **Failures:** no new refusal surface. `ContractUnmappable` /
  `ContractInvalid` still fire on exactly the cases they fired on before —
  unmapped readiness and audit-event states, a missing evidence record, an
  unknown family, an undeclared major, an out-of-enum payload value and a
  secret field — plus one new fail-closed refusal: a vendored registry that
  cannot answer a family's supported majors. `design.md` §3a-bis records
  the case-by-case analysis of where a gate-result refusal would have been
  needed and why none is.
- **Tests:** `cargo test --lib contract` (rewritten mapping rows, new
  check-state rows, new version-registry rows, new emit-level
  schema-vocabulary proof, rewritten major-validation tests), plus
  `cargo test --lib` and `cargo test --test api_contract` for the
  read-only admin route, `scripts/contract-parity.sh`, the name checker
  and strict validation.
- **Compatibility:** consumers pinned to a 0.1.0 gate-result document
  must read the `contract` discriminator; that is the intended, published
  major boundary (`narrowing.state == "declared"`), and the alternative —
  emitting a document labelled `0.1.0` that contains a value the 0.1.0
  enum forbids — is not a compatibility posture, it is a false claim.
  Every other family's emitted bytes are unchanged.
- **Concerns:** `quality.md` (each new behaviour gets success, failure
  and boundary evidence; unknown/unavailable/stale are not healthy);
  `security.md` (the vendored schema is untrusted input re-checked by
  digest, notes stay redacted and bounded, no new process or filesystem
  boundary).

## Capabilities

- **`platform-contract-consumption`** (modified): declared contract
  version per family; honest, total status mapping onto the 1.0.0
  vocabulary with an explicit refusal; re-vendored mirror in parity.

## Non-goals

- **Migrating any other contract family.** `platform.readiness`,
  `platform.audit-event`, `platform.job-outcome`,
  `platform.release-evidence`, `platform.capability` and
  `platform.public-portfolio-manifest` keep their 0.1.0 registration and
  their vendored bytes. The `contracts/` diff is limited to
  `schemas/gate-result.schema.json`, the `platform.gate-result` record
  inside `registry.json`, and the manifest records that follow from
  re-vendoring.
- **Adding `Unbound` to `GateCheckState`.** A sibling `UNBOUND` row
  still collapses into Forge's `unresolved` because `GateCheckState` is
  Forge's own closed vocabulary and this change may not alter it. The raw
  status survives in the redacted, bounded `detail` string.
- **Re-syncing `contracts/vocabulary/governance-vocabulary.json`.** That
  file belongs to `workspace-governance`, whose sibling checkout has moved
  on; re-vendoring it here would pull an unrelated vocabulary change into
  a gate-result change. The sync runs with `GOVERNANCE_DIR` pinned at the
  revision the mirror already records, so the file's bytes and its
  recorded revision are untouched.
- **Extending the payload validator to nested `checks[]` enums or to
  string patterns.** `validate_payload_against_schema` checks top-level
  `required` and top-level `enum` only. That is pre-existing and applies
  to every family; the vocabulary proof is therefore a test that reads
  the vendored schema directly rather than a validator change. (It also
  means the `gate_id` pattern `^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$` is
  not enforced today even though Forge builds `gate_id` as
  `<project>/<runtime>` — a pre-existing, separately-owned gap, recorded
  here rather than silently repaired.)
- **Changing the driftwatchdog consumption path** (`src/gate/**`).
- **Re-vendoring `platform-contracts` fixtures or the consumer-record
  schema.** They are not vendored today and no Forge surface needs them.
- Phase: hardening of an already-shipped producer surface. No publish, no
  push, no release.