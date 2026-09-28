# Design: Portfolio share and publication

## Implementation boundary

Repository `/home/paul/code/forge`; Rust service and SQLite persistence. Inspect
the existing project registry, operation/idempotency, authorization, audit,
publish-provider, and liveness modules. Add a portfolio-share domain module,
HTTP/CLI orchestration, migrations, and provider contract tests. Do not change
product repositories or Hugo templates.

## Language and runtime

Rust using the repository's existing toolchain, SQLite, HTTP API, and CLI.
Use existing formatter, linter, unit/integration test commands, and strict
OpenSpec validation. The default publisher writes a local artifact; a GitHub
Pages adapter is optional and credential-injected.

## Ownership and shared code

Forge owns share state and publication operations. `platform-contracts` owns the
serialized manifest schema. Projects own their public route behavior and auth.
No shared runtime library is added. The adapter receives a manifest and
publishes it; it cannot query a product database.

## Behavioral model

| State | Allowed transition | Rule |
|---|---|---|
| `draft` | `validated` | all required fields and allowlisted URLs pass |
| `validated` | `approved` | authorized admin explicitly approves exact revision |
| `approved` | `published` | idempotent publisher accepts exact manifest hash |
| `published` | `superseded` | later approved revision replaces it |
| any | `rejected` | validation or authorization failure is recorded without publication |

Each record has stable project identity, title, summary, category, source URL,
optional demo URL, optional public surface labels, visibility, featured flag,
status evidence, manifest revision, and timestamps. A manifest is sorted by
stable project ID, canonicalized, hashed with SHA-256, and published only when
the approval hash equals the export hash.

Authorization is existing Forge project/admin authorization. Approval and
publication require admin permission. Repeating the same operation key and
manifest hash returns the original result. A changed manifest requires a new
approval.

## Contract and compatibility

The provider input is the versioned `public-portfolio-manifest` schema. The
provider result includes `operation_id`, `manifest_sha256`, target, status,
published revision, and safe error code. Unknown manifest fields are rejected
by Forge before publication; the adapter never receives private fields.

Backward compatibility: no existing project or publish operation changes. A
project without a share record is omitted, not auto-discovered or auto-listed.

## Failure and boundary policy

- Missing required metadata, non-HTTPS public URL, duplicate project, private
  surface, or secret-like value: reject and persist findings.
- Unauthorized preview/approval/publish: deny; do not reveal private records.
- Liveness unavailable: publish status as `unknown` only if the admin approves
  that explicit evidence state; never infer `online`.
- Publisher timeout: retain approved manifest, mark attempt failed, allow safe
  retry with the same operation key.
- Partial external publication: report `unknown`, do not claim success, and
  require reconciliation before a new revision is marked published.
- Empty approved catalog: valid and publishable with an explicit empty reason.

## Verification oracle

Tests must prove: private records are excluded; admin URLs and secrets are
rejected; canonical ordering and hash are stable; exact approval is required;
retries are idempotent; changed revisions require reapproval; adapter failures
do not create false success; audit records contain actor/revision/hash/result;
and a golden manifest validates against `platform-contracts` fixtures.

## Decision ledger

- Forge is the source of truth for publication intent; GitHub Pages is a
  consumer and rendering target.
- A local export is the default so Forge remains useful without GitHub access.
- GitHub App/token credentials are an adapter concern and are never stored in
  the manifest or project records.
- Analytics and monetization are intentionally deferred to a separate package.
- No unresolved architecture blocker remains for this package; exact adapter
  credential provisioning is an implementation/deployment concern and must be
  documented before enabling that adapter.

## Refinement history (implementation, 2026-09-29)

Three decisions were fixed during implementation. Each is recorded here because
each changes a line the original design left open.

1. **The hashed manifest body excludes `generated_at`.** The consumed contract
   requires `generated_at` on the document, but the change also requires "the
   same canonical bytes and SHA-256 for the same approved records". Those two
   rules conflict if the emission time is inside the hash. Resolution: the
   hashed part is `ManifestBody` (`schema_family`, `schema_version`,
   `manifest_revision`, `projects[]`) and the published document is
   `PublicPortfolioManifest`, which adds `generated_at` and `manifest_sha256`.
   A retry under the same operation key re-renders the bytes of the *first*
   attempt (its emission time is stored on the publication row), so a retry
   reconciles rather than publishing a document that differs only in a
   timestamp.
2. **No record is ever stored as an unvalidated `draft`.** The state table
   above lists `draft` as the entry state, but Forge validates a write before
   it persists it, so an invalid record is never stored. The persisted
   vocabulary is `validated | approved | published | superseded | rejected`.
   A `rejected` state is reachable: when an admin's edit to an *existing*
   record fails validation, the record moves to `rejected` and leaves the
   manifest rather than continuing to serve its previous, now-unrepresentable
   content. The refusal reason is persisted as a finding either way.
3. **Publication attempts are append-only rows keyed by `operation_key`.**
   The registry's `reserve_idempotent_operation` is reused conceptually but not
   literally: a share publication needs its own audit row shape (revision,
   hash, target, publisher outcome, emission time), so the uniqueness lives on
   `portfolio_share_publications.operation_key`. The share domain writes **no**
   `operations` journal row, which keeps the publish journal unchanged.

## Blocked verification (recorded, not passed)

- `platform-contracts` still has `public-portfolio-manifest` as an unimplemented
  proposal (0 tasks, no `schemas/public-portfolio-manifest.schema.json` on
  disk). Forge pins the family name `public-portfolio-manifest` and major
  version `1` from that proposal's design and enforces the field set itself.
  The cross-repo schema-fixture validation is therefore **blocked**, not
  passed; it becomes available when that package is implemented and archived.
- The GitHub Pages adapter is implemented as a contract
  (`forge-portfolio-share-adapter/0.1.0`) and exercised with local executable
  stubs. No credential-injected adapter was run against GitHub, so **no
  external publication is claimed**.

