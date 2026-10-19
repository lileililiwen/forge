# Design: consumer-conformance-matrix

## 1. Implementation boundary

Exact repository/project: `rust-platform-libs`, Rust workspace, crate
`platform-contracts` only, plus one spec delta file.

Files to inspect/change:

- `crates/platform-contracts/src/payload/gate_result.rs` (new),
  `job_outcome.rs` (new), `release_evidence.rs` (new),
  `capability.rs` (new) — mirror
  `crates/platform-contracts/src/payload/readiness.rs` and
  `identity_subject.rs`.
- `crates/platform-contracts/src/payload/mod.rs` — add 4 `pub mod`
  lines, 4 re-export lines, 4 `ContractPayload` impls.
- `crates/platform-contracts/src/lib.rs` — extend the existing
  `pub use payload::{...}` list with the new type names.
- `crates/platform-contracts/tests/fixture_parity.rs` — add 5 walker
  tests reusing `walk_family`; no helper changes.
- Round-trip unit tests: inline `#[cfg(test)]` modules in each new
  payload file (serialize a minimal value → `validate_payload` against
  a registry loaded from the live repo path is integration-only, so
  instead assert serde round-trip plus required-field presence; the
  walker tests own the schema proof).
- `openspec/changes/consumer-conformance-matrix/specs/rust-platform-foundation/spec.md`
  (new delta), `HANDOFF.md`, `ROADMAP.md` (status lines only).

Must NOT change: `validate.rs`, `registry.rs`, `envelope.rs`, existing
payload modules, any other crate's `src/`, `Cargo.toml` /
`Cargo.lock`, schemas or fixtures in the sibling `platform-contracts`
repo, `mailing-provider-adapters` / `post-mvp-readiness` proposals.

## 2. Language and runtime

Rust, edition 2021, pinned toolchain `1.95.0` via `rust-toolchain.toml`
(declared MSRV 1.75 untouched). Dependencies: `serde` (derive) and
`serde_json` only — both already in `[workspace.dependencies]`. No new
crate dependencies, no features, no async.

Commands: `cargo test -p platform-contracts`,
`cargo test --workspace`, `cargo fmt --all -- --check`,
`cargo clippy --workspace --all-targets -- -D warnings`,
`node scripts/check-openspec-change-names.mjs`,
`openspec validate --changes --strict --no-interactive`,
`openspec validate --specs --strict --no-interactive`.
Fixture walkers resolve the sibling repo via `PLATFORM_CONTRACTS_DIR`
or the `../platform-contracts` fallback (present in this environment).

## 3. Ownership and shared code

Project-local extension of the shared owner: the new payload types live
in `platform-contracts` (the crate that already owns the payload
pattern and the validator), exported through its existing `pub use`
surface. Consumers (`openaccounting`, `rust-ecommerce`, `hookit`,
`openpanel`, `vowra`, future Rust services) adopt the builders in their
own OpenSpec changes; this change edits no consumer. No sibling
`common`/`manager` extraction applies — the capability already has its
owner. Naming follows the existing file-per-family convention
(`gate_result.rs` for `platform.gate-result`, etc.) and enum variants
follow the schema's lowercase values via `#[serde(rename_all =
"snake_case")]`.

## 4. Behavioral model

| Family | Required fields (schema) | Optional fields | Enums (schema values) |
|---|---|---|---|
| `platform.gate-result` → `GateResult` | `gate_id`, `pipeline`, `evaluated_at`, `result` | `subject_ref`, `checks: Vec<GateCheck{check_id, status, detail?}>`, `evidence_refs`, `tenant_id` | `GateResultValue`: `passed`, `failed`, `errored`, `skipped` (reused for check status) |
| `platform.job-outcome` → `JobOutcome` | `job_id`, `job_type`, `started_at`, `result` | `finished_at`, `items_processed: u64`, `items_failed: u64`, `tenant_id` | `JobResult`: `succeeded`, `failed`, `partial`, `cancelled` |
| `platform.release-evidence` → `ReleaseEvidence` | `release_id`, `service`, `version`, `released_at`, `evidence_kind` | `artifact_refs: Vec<String>`, `gate_refs: Vec<String>`, `tenant_id` | `EvidenceKind`: `deploy`, `rollback`, `dry_run` |
| `platform.capability` → `Capability` | `capability_id`, `service`, `version`, `declared_at`, `stability` | `contract_refs: Vec<String>`, `description` | `Stability`: `stable`, `experimental`, `deprecated` |
| `platform.readiness` → `Readiness` | already shipped | already shipped | already shipped |

Invariants: every struct derives `Debug, Clone, PartialEq, Eq,
Serialize, Deserialize`; schema-optional fields are `Option` /
`skip_serializing_if`; timestamps stay raw `String` (RFC3339) per the
payload-module convention; `family()` returns the exact
`platform.<name>` registry id. Types are permissive builders — the JSON
Schema remains the acceptance authority (invalid values are still
representable as Rust values and rejected by the validator, matching
the existing 4 families).

## 5. Contract and compatibility

Wire shape is fixed by the sibling schemas (source of truth; never
copied in-tree). Field names, required sets, enum spellings, and
patterns above are transcribed 1:1 from
`schemas/<family>.schema.json` current versions (`0.1.0`, major `0`).
Serde mapping: `snake_case` rename covers every enum value including
`dry_run` (`DryRun`) and `not_ready` (existing). `contract_refs`
entries keep their `platform.<family>/<semver>` string form (no parsed
type — validation owns the format check). Backward compatibility:
purely additive crate surface; `additionalProperties: true` in every
schema means old readers tolerate new producer fields. No migration,
no env keys, no version bump of existing contracts.

## 6. Failure and boundary policy

No new failure policy — the unchanged generic validator decides every
case, and each family's 12 canonical fixtures pin the behavior:

- `invalid.missing_required` → `payload.missing_required`.
- `invalid.bad_enum` → `payload.enum`; `invalid.bad_pattern` →
  `payload.pattern`.
- `boundary.max_string_length` valid; `boundary.over_string_length` →
  `payload.max_length`.
- `unknown-field.allowed_additive` valid (tolerated);
  `unknown-field.security_rejected` → `compatibility.security_field_added`
  warning, still valid.
- `redaction.credential_rejected` → `redaction.secret_field` error.
- `compatibility.major_unsupported` → `version.unsupported_major`
  error; `compatibility.minor_accepted` valid.
- `valid.optional` → valid + `compatibility.security_field_added`
  warning (security-namespaced producer addition).

Malformed Rust-side input cannot occur (all fields are strings/enums);
schema-violating values serialize fine and fail validation loudly, by
design of the permissive-builder convention.

## 7. Verification oracle

- `cargo test -p platform-contracts` — 5 new walker tests green: each
  walks its family's 12-fixture manifest via the unchanged
  `walk_family` helper against the live sibling repo (60 cases total);
  any mismatch prints `family/file: expected=… got valid=…`
  with codes and fails the test.
- Round-trip unit tests in each new payload module: build the minimal
  valid value (mirroring `valid.basic.json`), `serde_json::to_value`
  → assert required keys present and `serde_json::from_value`
  round-trips to the original value.
- Regression: `cargo test --workspace` fully green (existing 90+ tests
  plus new ones, incl. `platform-arch-tests` and
  `readme_documented_families_all_covered`); `cargo fmt --all --
  --check` clean; `cargo clippy --workspace --all-targets --
  -D warnings` clean.
- `node scripts/check-openspec-change-names.mjs` ok;
  `openspec validate --changes --strict --no-interactive` and
  `openspec validate --specs --strict --no-interactive` pass.
- `git status` shows no edits outside the §1 file list.

A task checkbox may be ticked only when its named command output is
recorded; environment-blocked checks are recorded as BLOCKED with the
exact command, never as passing.

## 8. Decision ledger

- **Assumption:** sibling `platform-contracts` schemas/fixtures for the
  5 families are stable (`0.1.0`, 12 fixtures each, verified this
  session). If a fixture count differs at implementation time, the
  walker (manifest-driven) still passes; record the actual count in
  HANDOFF evidence.
- **Resolved — `public-portfolio-manifest` excluded:** tenth registry
  family, major version 1, Forge (Rust) / Hugo (Go) owner, outside
  ROADMAP step 4. Separate change if a Rust consumer needs it.
- **Resolved — no validator change:** the generic schema +
  redaction + compatibility path already handles all 5 families; the
  first walker run confirms this before any payload code is written
  (BFS phase).
- **Resolved — `Readiness` reused as-is:** already shipped with
  `ContractPayload`; only its walker test is added.
- **Deferred:** Axum/SQLx/provider adapters (ROADMAP steps 5–6),
  consumer adoption (each consumer's own change).
- **Blockers:** none. Sibling repo reachable at `../platform-contracts`
  in this environment.
