# Proposal: The public manifest is produced in a shape the consumer rejects

## Why

Forge is the producer of the document a separate static Hugo site consumes.
That consumer validates the document against the pinned schema
`platform.public-portfolio-manifest/1.0.0`
(`/home/paul/code/platform-contracts/schemas/public-portfolio-manifest.schema.json`,
digest `07a3c47769da8923998273cda602ddffb195f983e3dcf344fb1d86540c6bc986`).
Three fields Forge emits do not satisfy it, so the first real export fails the
consumer's build.

This is reproduced, not hypothesised. Running the consumer's own checker,
`python3 /home/paul/code/lileililiwen.github.io/scripts/validate_manifest.py`,
against a Forge-shaped document reports:

```
schema_family must be 'platform.public-portfolio-manifest'; got 'public-portfolio-manifest'
schema_version must match ^[0-9]+\.[0-9]+\.[0-9]+$; got 1
manifest_revision must match ^[A-Za-z0-9][A-Za-z0-9._:-]{0,63}$; got 4
```

| Field | Schema requires | Forge emits | Source |
|---|---|---|---|
| `schema_family` | string, enum `["platform.public-portfolio-manifest"]` | `"public-portfolio-manifest"` | `src/portfolio/share/mod.rs:69` (`MANIFEST_SCHEMA_FAMILY`) |
| `schema_version` | string, `^[0-9]+\.[0-9]+\.[0-9]+$` | JSON number `1` | `src/portfolio/share/mod.rs:72` (`MANIFEST_SCHEMA_VERSION: u32 = 1`), used at `src/portfolio/share/manifest.rs:66,75,100,146` |
| `manifest_revision` | string, 1..64 chars, `^[A-Za-z0-9][A-Za-z0-9._:-]{0,63}$` | JSON number (`u32`) | `src/portfolio/share/manifest.rs:67,72,102,148` |

The cause is a version of the upstream contract that did not exist when the
archived change `2026-09-28-portfolio-share-publish` pinned the family. That
change's own design recorded the state honestly: *"`platform-contracts` still
has `public-portfolio-manifest` as an unimplemented proposal (0 tasks, no
`schemas/public-portfolio-manifest.schema.json` on disk). Forge pins the family
name `public-portfolio-manifest` and major version 1."* Forge pinned a name and
an integer where the contract now requires a qualified name and a semantic
version. There was no schema on disk to fail against, so the pin was never
checked; the contract has since shipped, and the pin is now measurably wrong.

`contracts/schemas/public-portfolio-manifest.schema.json` is absent from
Forge's own mirror, so nothing inside this repository could have caught it
either.

## What Changes

- `MANIFEST_SCHEMA_FAMILY` becomes the full contract name
  `platform.public-portfolio-manifest`.
- `MANIFEST_SCHEMA_VERSION` becomes the semantic version string `"1.0.0"`
  instead of the integer major `1`.
- The **serialized** `manifest_revision` becomes a string, derived
  deterministically from the existing integer revision as `rev_<revision>`.
- The **internal** revision stays an integer everywhere it already is: the
  `manifest_revision INTEGER` column (`src/registry/share/mod.rs:88`), the
  `i64` approval/audit/publication/report types
  (`src/portfolio/publication.rs:52`, `src/portfolio/share/audit_types.rs:29`,
  `src/portfolio/share/publish.rs:24,191`), and
  `build_manifest(records, manifest_revision: u32)`. **No database schema
  change, no persistence type change, no audit-trail change.**
- A test pins the new wire shape and a second test validates a real
  Forge-produced document against the pinned schema with `jsonschema`.

## BFS Impact Map

| Surface | State today | Action |
|---|---|---|
| `src/portfolio/share/mod.rs:69` | `"public-portfolio-manifest"` | change value |
| `src/portfolio/share/mod.rs:72` | `MANIFEST_SCHEMA_VERSION: u32 = 1` | change type and value to `&str = "1.0.0"` |
| `src/portfolio/share/manifest.rs:66,100` | `schema_version: u32` | `String` |
| `src/portfolio/share/manifest.rs:67,102` | `manifest_revision: u32` | `String` |
| `src/portfolio/share/manifest.rs:72-78` | `ManifestBody::new(u32, ..)` | keep the `u32` parameter, encode inside |
| `src/portfolio/share/manifest.rs:75,146` | constant / copy sites | follow the new types |
| `src/portfolio/share/manifest.rs:85-92` | `canonical_json` / `sha256` | **unchanged** — a stable string of the same integer keeps an unchanged catalog hashing identically |
| `src/main.rs:10560,10586` | preview human text and JSON, no numeric assumption | follows the type |
| `src/api/mod.rs:2889` | `GET /v1/share/manifest` preview JSON, no numeric assumption | follows the type |
| `src/registry/share/mod.rs:88` | `manifest_revision INTEGER NOT NULL` | **not touched** |
| `src/portfolio/publication.rs:52,162,223` | `i64` report + publish context | **not touched** |
| `src/portfolio/share/audit_types.rs:29` | `i64` audit row | **not touched** |
| `src/portfolio/share/publish.rs:24,191,256` | `i64` adapter envelope | **not touched** |
| `src/portfolio/share/manifest.rs:392-394` | in-module test asserts the numeric shape | updated |
| `tests/portfolio_share_cli_contract.rs:313-314` | asserts `schema_family` / numeric `schema_version` | updated |
| `tests/portfolio_share_cli_contract.rs:876-898` | asserts key set and `schema_family` equality | key set unchanged, stays green |
| `tests/portfolio_share_cross_surface.rs`, `tests/portfolio_share_api_contract.rs` | read `manifest_sha256` only | no numeric assumption, unaffected |
| `tests/portfolio_contract.rs` | own DB fixtures, unrelated `schema_version` column | unaffected |
| `contracts/**` | vendored mirror, owned by the parked change | **not touched** |
| `contracts/schemas/public-portfolio-manifest.schema.json` | **absent from the mirror** | out of scope; the acceptance test reads the sibling checkout read-only |

## Capabilities

- `portfolio-share` — the public artifact is emitted in the contracted shape.

## Non-goals

- **The vendored contract mirror.** `contracts/**`,
  `scripts/contract-parity.sh` and the parked change
  `contract-parity-gate-real-digests` are untouched. That change is a
  different concern and remains in flight.
- **The database, persistence types and audit trail.** The revision stays an
  integer everywhere it is stored, reported or approved.
- **Archiving.** This change is left active with evidence rather than archived.
- **Three further schema mismatches found while mapping the document**, which
  this change does **not** fix and which are recorded as remaining defects
  rather than silently widened scope. Each is reproduced below.
  1. `visibility` — the schema's enum is `["public"]` only; Forge's
     `Visibility::ALL` (`src/portfolio/share/mod.rs:268`) also admits
     `unlisted`. `forge portfolio share set --visibility unlisted` is accepted
     and the published document is then rejected:
     `projects/0/visibility: 'unlisted' is not one of ['public']`.
  2. `status_evidence` — the schema sets `additionalProperties: false` and
     permits only `observed_at`, `source`, `note`; Forge's `EVIDENCE_KEYS`
     (`src/portfolio/share/mod.rs:183`) admits `observed_at`, `source_system`,
     `source_revision`, `state`, `source`. A record with `--evidence
     '{"observed_at":"…","source_system":"…"}'` is published and then rejected:
     `projects/0/status_evidence: Additional properties are not allowed
     ('source_system' was unexpected)`.
  3. `id` — the schema's 64-character ceiling is enforced nowhere in Forge.
     `validate_project_id` (`src/core/mod.rs:577`) already pins the schema's
     *pattern* exactly, so that half is safe, but it has no length bound: a
     70-character project id registers, publishes and is then rejected with
     `projects/0/id: … is too long`.
