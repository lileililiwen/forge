# Design: adopt `platform.gate-result` 1.0.0

## 1. Ownership and boundary

| Path | Change |
|---|---|
| `src/contract/mod.rs` | **the whole change**: `PLATFORM_FAMILY_VERSIONS` + `platform_family_version`, `contract_discriminator`, `map_gate_result` (replaces `map_gate_aggregate`), `map_gate_check_state` (replaces the inline `match`), `emit_gate_result`'s check loop, `validate_envelope`'s major check. Still pure at the mapping layer: no clock, no filesystem, no database, no manifest, no new dependency. |
| `src/contract/contract_tests.rs` | mapping rows rewritten for the 1.0.0 vocabulary and extended to the check level; new version-registry, discriminator and schema-vocabulary proofs; the two major-version tests rewritten. |
| `src/cli/functions_13.rs` | `forge contract inspect`'s human line stops printing a literal `/0.1.0`. One format string. |
| `contracts/**` | written only by `scripts/sync-contracts.mjs`. |

The emitter MUST NOT:

- add a dependency (no JSON-Schema validator crate, no regex crate);
- touch `src/gate/**` — `GateAggregate`, `GateCheckState`,
  `GateEvidence` and `classify_aggregate` are Forge's own vocabulary and
  this change only changes how they are *rendered*;
- add an MCP tool, API route, portal section, journal row or generated
  project file;
- write to any project tree (`emit_gate_result` stays read-only);
- validate its own output beyond what `validate_envelope` already does —
  it asserts the mapping table and refuses, it does not validate.

## 2. The upstream vocabulary, quoted

From `platform-contracts` `schemas/gate-result.schema.json` at `5e8a6ca`:

> `result` … `passed`: the gate ran and every asserted check passed.
> `failed`: an attempted check reported a failure. `errored`: a check could
> not be evaluated to a verdict. `skipped`: a check was proven not to
> apply; this is an exemption, not an absence of evidence.
> `review_required`: a human decision is still open and no one has decided
> the outcome. `unbound`: the check was asserted and no command was ever
> bound to it, so nothing ran; this is a configuration defect and is never
> an exemption. **A consumer MUST NOT read `skipped` and `unbound` alike,
> and MUST NOT read `review_required` as a decision already taken.**

> The vocabulary is shared by `result` and `checks[].status` and each value
> carries the same meaning at both levels.

That last sentence is the constraint that makes Forge's mapping
symmetric: the same function must serve both levels, and the sibling
(`driftwatchdog`, `platform-gate-result-major-adoption` §3) applies the
same rule — one total function, both levels.

## 3. Status mapping (normative)

Two hard rules, applied to every row:

- **R1 — no unobserved assertion.** A rendered value may only state a
  claim Forge actually observed. `errored` is never produced, because
  Forge's gate plane has no source for it: it does not run the checks
  (the sibling does) and it never observes an adapter-level evaluation
  failure — `src/gate/mod.rs` documents that a parseable document is
  evidence *whatever the exit code*, and an unparseable document, a
  timeout or a spawn failure is `GateOutcome::Unavailable`, which never
  becomes evidence and therefore never reaches a contract payload.
- **R2 — refuse rather than force.** Where 1.0.0 has no honest value the
  record is refused with `ContractUnmappable` (rendered as
  `ForgeError::ContractInvalid` naming the family and the value). `errored`
  is never reused as a catch-all.

### 3a. `result` — `map_gate_result(&GateEvidence)`

| Forge record | Forge's own documentation of that state | 0.1.0 | 1.0.0 | Evidence and rejected alternatives |
|---|---|---|---|---|
| `Passed` | `classify_aggregate`: `"PASS" if exit_success` — the gate ran and reported a pass | `passed` | `passed` | The 1.0.0 text for `passed` is the same claim. No alternative exists and none is needed. |
| `Failed` | `src/gate/model.rs`: "`failed` and `unknown` keep **a non-blocking failure** and an unclassifiable document honestly separated from `blocked`" | `errored` | **`failed`** | 1.0.0 `failed` = "an attempted check reported a failure" — Forge's own definition. **Rejected `errored`** ("a check could not be evaluated to a verdict"): the sibling produced a verdict, so R1 forbids it. *This is a third lossy coercion the adoption corrects; the task brief named only the two `Unknown`/`Unresolved` rows, and this row fails the same R1 test.* |
| `Blocked` **with** a `Fail` check row | the sibling's `blocked` bit, with a required check that reported a failure | `failed` | `failed` | A definite, evaluated failure was observed; identical to today's output. **Rejected `review_required`**: nothing about a failing required check is an open human decision. **Rejected `errored`**: the check ran to a verdict. |
| `Blocked` **with no `Fail` row** (any number of `Unresolved`, `Skip`, `NotApplicable`, `Pass` rows, or no rows at all) | the sibling's `blocked` bit with no check row attributing a failure to it | `failed` — **asserted a check failure Forge never observed** | **`review_required`** | Forge observed a gate that did not proceed and **no attempted failure**: no row says a check failed, no row says a check could not be evaluated, and no row says a command was never bound. The one claim 1.0.0 can make honestly is that nobody has decided the outcome — which is also exactly the shape the sibling produces here (a required check with no result row, a required-but-inapplicable check with no declared exemption, or a pending review; `driftwatchdog` `src/gate/aggregate.rs:100-137`, `:169-187`). **Rejected `failed`**: R1. **Rejected `unbound`**: Forge never inspects the sibling's check binding. **Rejected `errored`**: R1 and R2. |
| `Unknown` | `src/gate/model.rs`: "an unclassifiable document"; `classify_aggregate` routes `REVIEW_REQUIRED`, a `PASS` riding a failure exit, and every unrecognized status here | `errored` — asserted an evaluation failure Forge never observed | **`review_required`** | Forge deliberately declined to decide: the note it persists says the state "is not in the known vocabulary and never counts as pass". "A human decision is still open and no one has decided the outcome" is the same statement in the contract's vocabulary. **Rejected `errored`** (R1 — the task brief's *most serious defect*). **Rejected `failed` / `unbound`**: both assert an outcome. |

### 3a-bis. Where a refusal would have been, and why there is none

The brief's hard constraint says to refuse when 1.0.0 has no honest
value. For `platform.gate-result` that branch is unreachable, and the
analysis is worth recording rather than leaving to a future reader:

| Candidate case | Honest 1.0.0 value | Why it is honest |
|---|---|---|
| Blocked, all rows `Pass`, required check produced no `results[]` row | `review_required` | Forge observed a block and no failure. The sibling put the missing required check in `pending_reviews` — a human must resolve it (`aggregate.rs:100-107`). Nothing was decided. |
| Blocked, only `NotApplicable` rows | `review_required` | A required-but-inapplicable check without a declared exemption *is* a pending review to the sibling (`aggregate.rs:127-133`); Forge observed the inapplicability, not a failure. |
| Blocked, zero rows | `review_required` | Same claim, weaker evidence, same truth: nothing was observed to have failed and the gate did not proceed. |
| A future `GateCheckState` variant | n/a today | Adding a variant is a compile error in `map_gate_check_state`, so a new state cannot be silently defaulted. The `Result` shape is retained so the refusing interface survives until someone adds a row with no honest value — at which point the `Err` arm is written, not guessed. |

Refusing the blocked-without-a-failure case was considered and
**rejected**: it would have introduced a new hard failure on a realistic
emit path (`forge contract emit gate-result` and the admin emit route)
for a record whose only true property is that nothing was decided — a
property `review_required` states without asserting anything untrue.
The `ContractUnmappable` / `ContractInvalid` refusals are untouched and
still fire on the cases they fired on before: unmapped readiness states,
unknown audit-event journal states, a missing evidence record, an
unknown family, an undeclared major, an out-of-enum payload value and a
secret field.

### 3b. `checks[].status` — `map_gate_check_state(GateCheckState)`

| `GateCheckState` | Forge's own documentation of that state | 0.1.0 | 1.0.0 | Evidence and rejected alternatives |
|---|---|---|---|---|
| `Pass` | `from_raw("PASS")` | `passed` | `passed` | Unchanged. |
| `Fail` | `from_raw("FAIL")` | `failed` | `failed` | Unchanged; "an attempted check reported a failure". |
| `Skip` | `from_raw("SKIP")` | `skipped` | `skipped` | **Retained, with the caveat recorded.** The packaged runtime (`SUPPORTED_GATE_RUNTIMES = ["driftwatchdog"]`) emits no `SKIP` status — its statuses are `PASS`, `FAIL`, `NOT_APPLICABLE`, `REVIEW_REQUIRED`, `UNBOUND` — so this arm is unreachable in practice and exists so a closed vocabulary stays closed. It maps to `skipped`, which asserts "proven not to apply". |
| `NotApplicable` | `from_raw("NOT_APPLICABLE")` | `skipped` | `skipped` | **Re-examined, kept.** 1.0.0 `skipped` = "a check was proven not to apply; this is an exemption, not an absence of evidence" — a word-for-word match for the state name and the only value that distinguishes a proven exemption from `unbound`. `unbound` is rejected because Forge observed an inapplicable result, not an absent binding. |
| `Unresolved` | `src/gate/model.rs`: "Unknown or unexpected state strings become `unresolved` — **never `pass`**" | `errored` — asserted the check errored | **`review_required`** | The task brief's *most serious defect*: the state exists precisely so Forge does not assert an outcome it did not observe, and `errored` ("a check could not be evaluated to a verdict") is precisely such an assertion. `review_required` states the true claim — nobody has decided — and is fail-closed. **Rejected `errored`** (R1). **Rejected `unbound`**: no binding source; a `UNBOUND` row arrives here and would be over-reported. **Rejected `skipped`**: an exemption was never proven; that is the conflation 1.0.0 explicitly forbids. The raw status survives in the redacted, bounded `detail` string (`src/gate/engine.rs:413-423`). |

### 3c. Why `Blocked` is decomposed at all

`GateAggregate::Blocked` is a **superposition**: `classify_aggregate`
short-circuits on the sibling's `blocked` bit before it ever looks at the
status string (`src/gate/engine.rs:461-464`), so it absorbs two different
claims from the sibling —

1. `status: FAIL, blocked: true` — a required check reported a failure
   (`driftwatchdog` `src/gate/aggregate.rs:169-177`), and
2. `status: REVIEW_REQUIRED, blocked: true` — required reviews are pending
   and `review_required_blocks` is set (`aggregate.rs:178-187`, default
   **`true`** at `aggregate.rs:55`).

Neither a flat `failed` nor a flat `review_required` is honest for both.
A flat `failed` — today's behaviour — asserts "an attempted check reported
a failure" for a run where nothing failed; R1 forbids it. A flat
`review_required` asserts "no one has decided the outcome" for a run where
a check failed and was recorded; R1 forbids that too. The observed
per-check rows are the evidence that separates them, and they are part of
the same `GateEvidence` record `emit_gate_result` already reads, so the
decomposition costs nothing, changes no persisted field, and touches
nothing in `src/gate/`.

The decomposition is two branches, not three: **an observed `Fail` row
means `failed`; no observed `Fail` row means `review_required`.** The
check-level `Unresolved` state is not consulted for the root value, because
it is not what distinguishes the two sibling claims — `driftwatchdog`
raises `blocked` for a pending review regardless of whether the pending
item is an out-of-vocabulary `REVIEW_REQUIRED` or a required check that
produced no row at all, and Forge renders both as "undecided".

### 3d. `unbound` is never produced, and that is stated

Forge has no `unbound` source: `GateCheckState` is a closed five-value
vocabulary and its `from_raw` collapses `UNBOUND` into `Unresolved`
(Non-goal: widening it is a `src/gate/` change). Such a row therefore
renders `review_required`, which under-claims rather than over-claims —
it says a human must act, not that the gate failed and not that no
command was bound. The proof that the value stays named rather than
silently vanishing is a test, mirroring the sibling's own
"`errored` is never produced" test in §3 of its design: the mapping table
test asserts no `GateCheckState` maps to `unbound`, while a second
assertion proves the value is still in the vendored 1.x schema enum.

## 4. Field and version mapping (normative)

| Platform field | Source | Constraint enforced |
|---|---|---|
| `contract` (envelope) | `PLATFORM_FAMILY_VERSIONS` lookup | `CONTRACT_PATTERN`; the only place the version is expressed, because the schema's `$id` (`https://platform.contracts/gate-result.schema.json`) carries no version |
| `gate_id` | `<project_id>/<runtime>`, unchanged | unchanged |
| `pipeline` | `evidence.runtime`, unchanged | unchanged |
| `evaluated_at` | `evidence.observed_at`, unchanged | unchanged |
| `result` | §3a | 1.0.0 `result` enum |
| `subject_ref` | `evidence.revision`, unchanged | unchanged |
| `checks[].check_id` | `c.id`, unchanged | unchanged |
| `checks[].status` | §3b | 1.0.0 `checks[].status` enum |
| `checks[].detail` | `c.note` after `redact_credentials`, unchanged | unchanged |

### The version-declaration shape

The declaration is the **existing envelope `contract` field**, whose value
becomes `platform.gate-result/1.0.0`. It is not a new field: Forge's
envelope already carries `contract`, and the only thing wrong with it is
that the version inside it was a literal that no longer described the
document. `additionalProperties: true` on the 1.0.0 schema would also
permit a *second* spelling, but a second spelling would give every future
consumer a branch, and the sibling already emits `contract` as a root
property for the same concept.

The schema's `$id` is version-free, so the registry/discriminator is the
only place a version can be expressed — which is why the version is
**registered**, not interpolated:

```rust
pub const PLATFORM_FAMILY_VERSIONS: &[(&str, &str)] = &[
    ("platform.audit-event", "0.1.0"),
    ("platform.capability", "0.1.0"),
    ("platform.gate-result", "1.0.0"),
    ("platform.job-outcome", "0.1.0"),
    ("platform.readiness", "0.1.0"),
    ("platform.release-evidence", "0.1.0"),
];
```

`contract_discriminator` reads it, so a family bump is a reviewed table
row rather than a string edit. A test proves the table covers exactly
`supported_families()`, with no duplicate family, and that every value
matches `CONTRACT_PATTERN`.

`build_envelope` keeps its signature. Its `unwrap_or_else` fallback is
retained so the builder stays total for a family nobody registered, and
is bound to a named `UNREGISTERED_FAMILY_VERSION` constant with a comment
saying no Forge emission path reaches it — the coverage test above is what
makes that claim checkable rather than hopeful.

### Validator: accept the majors the source still serves

`validate_envelope` previously refused any version not starting with
`0.`, which rejected `platform.gate-result/1.0.0` — the sibling's own
document. It now reads the vendored `contracts/registry.json` and refuses
a major the family's `supported_major_versions` does not list. For
`platform.gate-result` that is `[0, 1]`, so a 0.1.0 document (still
served, `narrowing.state == "declared"`) and a 1.0.0 document both
validate, and a `2.0.0` document refuses.

The vendored registry is the authority rather than a second hardcoded Rust
list, because it is the thing the source publishes, it is already
digest-pinned by `verify_manifest_digests`, and the parity gate compares
it byte-for-byte against the source. It fails **closed**: an unreadable,
unparseable registry or a missing family record is a refusal naming the
file, never a skipped check.

## 5. Process and integration boundaries

- `scripts/sync-contracts.mjs` is the only writer of `contracts/`. It
  resolves the platform source from `PLATFORM_CONTRACTS_DIR` or the
  sibling checkout, and the governance vocabulary from `GOVERNANCE_DIR`.
  The sibling `workspace-governance` checkout has moved past the revision
  the mirror records (`0301ee8…`), and its `vocabulary.json` has grown
  `roles`, `stacks`, `standalone_statuses`,
  `integration_required_states` and `readme_claim_surfaces` and dropped
  the `placeholder` / `stub` markers. Vendoring that here would pull an
  unrelated vocabulary change into a gate-result change. The sync is
  therefore run with `GOVERNANCE_DIR` pointed at a checkout of
  `workspace-governance@0301ee8…`, whose `vocabulary.json` is byte-identical
  (sha256 `95d1f4d1…`) to the file already vendored, so that file's bytes
  and its manifest record are provably unchanged. Nothing is hand-edited;
  the script still writes every file.
- The platform side of the diff is exactly the gate-result family:
  `contracts/schemas/gate-result.schema.json` and the
  `contracts["platform.gate-result"]` record inside
  `contracts/registry.json`. Every other family record is byte-identical
  before and after (measured with `diff`), and the files
  `scripts/sync-contracts.mjs` copies that upstream has not touched since
  the mirror's pinned revision are unchanged.
- `scripts/contract-parity.sh` is the acceptance check: it compares every
  retained file byte-for-byte against the resolved source and every
  published family digest against the source's own record.

### One coupled side effect of the sync, recorded rather than hand-patched

The sync also **added a manifest entry for the tracked file
`contracts/vocabulary/README.md`**, which `contracts/manifest.json` has
never listed (it was committed after the last sync). The script's
`collect()` walk treats every non-manifest file under `contracts/` as
vendored, so the entry records `source: "platform-contracts"` and the
platform revision — while the README is Forge's own documentation of the
consumed vocabulary, not vendored bytes.

It is kept as the script wrote it, because the alternatives are worse:
hand-editing the manifest is exactly what the brief forbids, and deleting
the file before the run and restoring it afterwards would smuggle the same
hand-edit in through the back door. The entry is harmless in practice —
its sha256 is correct, `verify_manifest_digests` now covers a file it
previously ignored, and `contract-parity.sh` excludes `contracts/vocabulary/`
from the parity decision — but the provenance label is imprecise. The
remediation path is to teach `collect()` a Forge-owned source value for
files it did not copy, which is a `scripts/` change with its own scope and
is deliberately not made here.

## 6. Security and privacy boundaries

- Unchanged: notes stay `redact_credentials`-redacted and character-bounded
  by the gate plane before they reach `detail`; the payload still passes
  through `payload_contains_secret_field` before the envelope is built.
- The vendored schema and registry are untrusted input re-checked by
  `verify_manifest_digests`; `validate_envelope` reads the registry through
  the same `contracts_dir()` resolution as the schema, and an unreadable
  registry refuses instead of skipping.
- No new process, filesystem, network, credential or authorization
  boundary. No MCP tool, API route or portal section is added.
- `subject_ref` and `tenant_id` remain the family's
  `security_sensitive_fields` in the registry record; Forge still emits
  `subject_ref` from the bound revision and never emits `tenant_id`.

## 7. Compatibility

- **Breaking, by design and by publication:** a Forge-emitted gate-result
  envelope now declares `platform.gate-result/1.0.0` and may carry
  `review_required`. This is the published major boundary the upstream
  narrowing block anticipates ("A consumer MUST emit 1.0.0 or later").
  The alternative — declaring `0.1.0` on a document whose `result` is not
  in the 0.1.0 enum — is a false claim, not compatibility.
- **Unchanged bytes:** every other family's envelope, the gate evidence
  record, `forge gate` / `forge gate --dry-run` output, the journal,
  generated project trees, MCP tools, the API route table and portal
  sections. `GATE_CONTRACT_VERSION` stays `"0.1.0"`: it versions Forge's
  own persisted `GateEvidence` record, not the platform family, and
  bumping it would invalidate every persisted evidence file.
- **Read-only:** `forge contract emit` writes nothing; the change adds no
  migration and no journal row.

## 8. Verification

- `cargo test --lib contract` — the mapping tables (every
  `GateAggregate` shape, every `GateCheckState`), the refusal, the version
  registry coverage/discriminator/pattern pins, an emit-level proof that
  every value the mapping can produce is inside the *vendored* schema's
  `result` and `checks[].status` enums (so a future upstream bump fails a
  test rather than shipping an invalid document), and the rewritten
  major-validation tests.
- `cargo test --lib` — no other module depends on the changed signatures.
- `scripts/contract-parity.sh`, the offline
  `manifest_digests_match` digest test, the name checker,
  `openspec validate --all --strict --no-interactive`, `git diff --check`.
- `cargo build`, `cargo fmt --check`, `cargo clippy` (default targets).

## 9. Open decisions recorded, not silently fixed

- `validate_payload_against_schema` checks top-level `required` and
  top-level `enum` only. Nested `checks[]` item enums and every string
  `pattern` are unenforced for all families, and Forge's `gate_id`
  (`<project>/<runtime>`) does not satisfy the schema's
  `^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$`. Both are pre-existing, family-
  independent, and outside this change's scope; the vocabulary proof in
  §8 is written as a test so the gap is measured rather than assumed away.
- `GateCheckState` cannot express `unbound` (§3d). Widening it is a
  `src/gate/` change with its own spec and is deferred.
- The narrowing block stays `state: "declared"` upstream. Moving it to
  `enforced` is upstream's decision, taken once every declaring consumer
  has migrated; this change is one of those consumers and does not touch
  the block's state.