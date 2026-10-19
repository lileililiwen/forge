# Proposal: Consumer conformance matrix for the remaining contract families

## Why

The in-tree fixture parity walker covers 4 of the 9 roadmap contract
families (`identity-subject`, `permission-decision`, `tenant-context`,
`audit-event`). The remaining 5 operational families — `readiness`,
`gate-result`, `job-outcome`, `release-evidence`, `capability` — are
validated on the wire only by the generic JSON-Schema path; there are no
typed Rust payloads for 4 of them (`Readiness` already ships) and no
walker tests pinning any of them to the canonical 12-case fixture matrix
(valid, invalid, boundary, unknown-field, redaction, compatibility).
Until this change lands, a consumer emitting a gate result or job outcome
has no typed builder and no machine-checked proof that the Rust validator
agrees with the language-neutral one on those families.

## What Changes

- Four new typed payload modules in `platform-contracts`
  (`gate_result.rs`, `job_outcome.rs`, `release_evidence.rs`,
  `capability.rs`) following the existing payload pattern (serde
  structs, snake_case enums, `Option` for schema-optional fields),
  re-exported from `payload/mod.rs` and `lib.rs`, each with a
  `ContractPayload` impl carrying its canonical family identifier.
- Five new walker tests in
  `crates/platform-contracts/tests/fixture_parity.rs` (one per family,
  reusing the existing `walk_family` helper), bringing parity coverage
  from 4 families to 9 of 9 roadmap families (60 new fixture cases:
  12 per family).
- Per-type serialization round-trip unit tests proving each new typed
  payload serializes to a document the registry-backed validator
  accepts.
- One ADDED requirement in `specs/rust-platform-foundation/spec.md`
  pinning the new payload + walker coverage.
- `HANDOFF.md` evidence + `ROADMAP.md` step-4 status updates at
  implementation time, per the repo's two-commit cadence.

## Package Boundary and Split Assessment

**Single outcome:** every roadmap contract family has a typed Rust
payload and a green 12-case parity walk, so the foundation can claim
9-of-9 roadmap parity.

**Included:** four payload modules + exports + trait impls, five walker
tests, round-trip unit tests, one spec delta, handoff/roadmap status
lines.

**Excluded (split signals):** `platform.public-portfolio-manifest` is a
tenth registry family (major version 1, Forge/Hugo owner, added after
the roadmap scope) and stays out — a separate change can cover it if a
Rust consumer needs it; HTTP/ Axum / SQLx adapters stay deferred per
ROADMAP steps 5–6; no consumer repository edits; no registry, schema, or
validator-policy changes (the generic validator already enforces the
policy for all families).

**Dependencies:** `rust-platform-foundation` (shipped) supplies the
validator, registry loader, and payload pattern. No new external
dependencies — serde/serde_json only, both already in the workspace.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| this repository — `platform-contracts` validator | `crates/platform-contracts/src/validate.rs` (`validate_document`, schema-driven, Draft 2020-12) | generic envelope/version/schema/redaction/compatibility policy already applies to all families | none — new families need no policy change | rust-platform-libs | **adopt** (reuse `walk_family` unchanged) |
| this repository — payload pattern | `crates/platform-contracts/src/payload/{readiness,identity_subject,permission_decision}.rs`, `mod.rs`, `lib.rs` | struct + snake_case enum + `ContractPayload` shape to mirror | 4 families missing | rust-platform-libs | **extend shared owner** (add 4 modules in the same crate) |
| `platform-contracts` sibling repo | `schemas/{readiness,gate-result,job-outcome,release-evidence,capability}.schema.json`, `fixtures/<family>/manifest.json` (12 cases each) | schemas (source of truth) + canonical fixture matrix | none | platform-contracts (language-neutral) | **adopt** (validate against live repo; never copy schemas in-tree) |
| `platform.public-portfolio-manifest` | `schemas/public-portfolio-manifest.schema.json`, `fixtures/public-portfolio-manifest/` | same walker shape would apply | different owner (Forge/Hugo), major version 1, outside roadmap step 4 | platform-contracts | **keep local out of scope** (non-goal; separate change if needed) |
| `trailCrew` / other consumers | path-dependency adoption only | no contract-family code to reuse | consumers emit these families opaquely today | each consumer | **not applicable** (no consumer edit in this change) |

## BFS Impact Map

- **Capabilities:** `platform-contracts` payload surface (+4 types);
  parity walker (4→9 families); `rust-platform-foundation` spec (+1
  requirement).
- **Unaffected:** validator policy, registry loader, envelope,
  `platform-core` / `platform-identity` / `platform-audit` /
  `platform-testing` / `platform-arch-tests` public APIs, CI, toolchain,
  sibling repos. `Readiness` type is reused, not reshaped.
- **Callers:** none in-tree (payloads are leaf types); consumers gain
  typed builders they can adopt in their own changes.
- **Failure/boundary:** all 12 canonical categories per family
  (missing-required, bad-enum, bad-pattern, max/min-length boundary,
  additive vs security-rejected unknown fields, credential redaction,
  major-unsupported vs minor-accepted) are exercised by the sibling
  fixtures through the unchanged generic validator.
- **Compatibility:** additive only — new modules, new tests, one spec
  requirement. No existing type, test, or fixture changes.
- **Security/privacy:** new payloads carry no secret-typed fields;
  redaction coverage comes from the existing
  `redaction.credential_rejected` fixture per family.
- **Ordering:** single active change; `mailing-provider-adapters` and
  `post-mvp-readiness` proposals stay untouched in-tree.

## Capabilities

- Typed, serializable Rust payloads for the 5 remaining operational
  contract families.
- 9-of-9 roadmap fixture parity (48 existing + 60 new cases green).

## Non-goals

- No `public-portfolio-manifest` coverage (tenth family, separate
  owner/version scope).
- No validator, registry, envelope, or schema change.
- No provider/framework adapters (Axum, SQLx, SMTP, OIDC/JWT).
- No consumer or sibling repository edits and no adoption claims.
- No change to `publish = false` or registry strategy.
