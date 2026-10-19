# Design: Forge-owned platform schema set

## 1. Implementation boundary

Work stays in `/home/paul/code/forge` on `main`.

- `contracts/schemas/` is the maintained source for six Forge-consumed family
  schemas and the common envelope/schema registry documents.
- `contracts/registry.json` remains the authority for supported major versions
  of the five envelope families. `contracts/family-support.json` is the
  authority for Forge's supported and unsupported family list, schema path,
  and unsupported reason.
- `contracts/manifest.json` hashes every retained contract file. Replace
  `scripts/sync-contracts.mjs` with an offline manifest builder/verifier over
  Forge-maintained files and the in-tree governance vocabulary.
- `scripts/package.sh` includes the `contracts/` tree under
  `share/forge/contracts`; `src/contract/mod.rs::contracts_dir` resolves the
  explicit `FORGE_CONTRACTS_DIR`, then installed assets beside the executable,
  then the source-tree path used in development.
- `src/contract/mod.rs` owns typed family metadata and validation behavior;
  `src/cli/functions_13.rs` and `src/api/admin/assurance.rs` expose it.
- `.github/workflows/ci.yml` and `scripts/release-check.sh` invoke the same
  offline contract check. Delete `scripts/contract-parity.sh` and its tests.
- `tests/manifest_wire_contract.rs` validates published portfolio output
  against `contracts/schemas/public-portfolio-manifest.schema.json` on every
  normal test run.

Do not edit the sibling `platform-contracts` checkout, its origin, or sibling
consumers. Do not change retained wire shapes or add a production schema
validator dependency.

## 2. Language and runtime

Forge Core and CLI use Rust 2021 with the repository's Rust 1.88 MSRV. Contract
metadata and projections remain in Rust. Schemas and registries are JSON Schema
2020-12 / JSON data. CI and release scripts remain POSIX shell and GitHub
Actions YAML. Existing commands are `cargo test`, `scripts/release-check.sh`,
`forge contract list|inspect|emit|validate`, and the authenticated assurance
API/browser tests.

Schema tests use Python `jsonschema==4.25.1` from a new root
`requirements-dev.txt`. CI installs that pinned development dependency before
the Rust suite; `scripts/release-check.sh` fails with the exact install command
when it is unavailable. The validator is not part of the Forge runtime or
binary. It validates Draft 2020-12 schemas and positive/negative canonical
fixtures offline.

## 3. Ownership and shared code

Forge owns six consumed schemas: `platform.gate-result`,
`platform.readiness`, `platform.release-evidence`, `platform.capability`,
`platform.audit-event`, and `platform.public-portfolio-manifest`. The first
five are emitted or validated through the platform envelope; the portfolio
manifest is produced by Forge's public portfolio publisher and is validated
against its own schema.

Forge does not claim the broader Platform Contracts catalog. It explicitly
marks `platform.identity-subject`, `platform.permission-decision`,
`platform.tenant-context`, and `platform.job-outcome` unsupported. The first
three have no Forge producer/consumer. Job outcome currently has a schema
lookup and unavailable UI message but no source record or emitter; the family
is therefore unsupported until Forge implements a real source record and
end-to-end validator/emitter behavior.

The checked-in schema bytes are the initial baseline. From this change onward,
Forge reviews schema changes under its own OpenSpec and compatibility policy;
`platform-contracts` is a read-only reconnaissance source, not a build-time,
test-time, release-time, or runtime dependency. No shared library extraction is
needed: the contract data is language-neutral, and Forge's Rust adapters remain
local to Forge.

Keep the Rust contract module below the repository's 1,000-line ceiling by
placing the existing CLI/API inventory table in a dedicated module. The new
catalog and asset checks must not push this foundational boundary past the
mandatory source-size Gate.

Remove the old required `platform-contract-copy` interface from Forge's own
`.project.json`; it represented the in-tree contract manifest as a sibling
provider copy. The contract asset manifest now accounts for its files, and the
remaining secret vocabulary interface names Forge as its provider.

## 4. User experience and interface

Actors are Forge maintainers using the CLI and authenticated assurance
workbench. Keep the current CLI and route entry points.

`forge contract list` and `GET /v1/admin/contracts` report the existing Forge
contract inventory plus platform family support records. Supported records
include family, version, and schema path. Unsupported records include family,
`status: unsupported`, and a safe reason. `inspect` on an unsupported family
returns a typed unsupported error. `emit` and `validate` reject unsupported
families before reading or writing a target. Unknown identifiers retain a
separate unknown-family error.

The assurance list presents supported and unsupported sections with explicit
status text; unsupported items are not offered as emit actions. The detail
view for a supported family keeps its schema and version. For an unsupported
family it shows its reason and status without schema bytes or emit controls.
Loading, empty, API failure, authentication expiry, and success states retain
the existing assurance workbench patterns. Use semantic status text, keyboard
navigation, and existing responsive card/table behavior. No new route,
navigation entry, or unauthenticated access is added. Verify the real browser
journey at desktop and narrow viewport widths and run the existing accessibility
assertions.

## 5. Behavioral model

| Input/state | Validation and transition | Result |
|---|---|---|
| Supported family | Family support record says `supported`; schema path exists; major is present in registry; every asset digest matches | List/inspect/validate/emission proceed under existing family semantics |
| Unsupported family | Family support record says `unsupported` with non-empty reason | List shows status/reason; inspect, emit, and validate return a typed refusal; no target read or mutation occurs |
| Unknown family | No exact family-support record | Bounded unknown-family response; never fall back to a default schema |
| Missing/malformed support record | Required metadata missing, duplicate family, unsafe path, or unsupported status lacks reason | Contract load fails closed; no CLI/API operation reports support |
| Missing/corrupt manifest or asset | Missing file, malformed JSON, missing entry, extra unlisted file, or digest mismatch | Offline check and contract operation fail naming the relative path and expected/actual state |
| Portfolio output | Publisher serializes its typed record; dev validator checks against maintained public manifest schema | Valid document passes; invalid fixture/document names the failing JSON pointer and schema |
| CI/release | Offline validation runs with no sibling checkout or network | Pass only after nonzero required schema/fixture/digest checks; failure blocks release readiness |

Manifest files are sorted by normalized relative path. The manifest records
Forge as owner and a source revision only when available; it never records
`platform-contracts` as a required source. Manifest generation is deterministic
for a fixed tree and excludes only its own self-digest file. The checker rejects
unlisted files under `contracts/` except explicitly documented non-contract
prose; prose files remain listed and hashed as assets if retained.

Installed path resolution derives the package root from
`std::env::current_exe()`: `<package>/bin/forge` uses
`<package>/share/forge/contracts`. The explicit environment override remains
first for tests and operator-managed data. The source-tree fallback is used
only when the installed directory does not exist. Package smoke verification
must inspect the archive for the six schemas, family support catalog, registry,
fixtures, vocabularies, and manifest, then run `forge contract list` from an
unrelated working directory using the extracted binary.

## 6. Contract and compatibility

- Retained schema contents, family discriminators, supported majors, emitted
  envelopes, CLI exit behavior for supported families, and portfolio manifest
  bytes remain compatible.
- `family-support.json` has a versioned top-level format and closed status enum
  (`supported`, `unsupported`). Supported rows require a schema path; platform
  envelope rows require a registered version. Unsupported rows require a
  non-empty human-readable reason and no schema path.
- CLI and API responses expose only relative repository schema paths, never
  absolute paths. Unsupported diagnostics do not echo arbitrary caller text.
- The manifest records per-file SHA-256 and Forge ownership. It is an offline
  integrity record, not proof of external parity or publication.
- Old consumers that parse contract list JSON must continue receiving existing
  `contracts` and `manifest` fields. Add a separate `platform_families` field;
  do not repurpose the existing array or change its row shape.

## 7. Failure and boundary policy

Missing, malformed, duplicate, unsafe, or digest-mismatched metadata is an
error. Unsupported is a typed capability refusal, distinct from unknown. The
web layer maps unsupported to a stable client error and displays the declared
reason; it never returns 500 for an intentionally unsupported family.
Unauthenticated callers remain 401. No network fallback, sibling path lookup,
self-parity comparison, cached alternate schema, or silent skip is permitted.
Manifest generation refuses symlinks and paths escaping `contracts/`.

## 8. Verification oracle

- Rust unit tests: manifest completeness/digests; support catalog schema and
  registry coverage; supported/unsupported/unknown lookup; all five retained
  envelope families and public portfolio manifest valid/invalid fixtures via
  `python3 -m unittest tests.contract_schema_tests`.
- CLI contracts: supported list/inspect/emit/validate unchanged; unsupported
  list state and typed refusal; unknown remains distinct; no file mutation on
  refusal.
- Assurance API/browser: auth boundary, supported and unsupported catalog and
  detail states, emit availability, keyboard operation, responsive layout, and
  accessibility assertions.
- CI/release scripts: offline execution with `PLATFORM_CONTRACTS_DIR` unset and
  a sibling directory absent; tampered/missing/extra assets fail; workflow has
  no source checkout or parity job.
- Package/install: extracted archive contains every contract asset and its
  binary successfully lists and inspects contracts from outside the source
  checkout.
- Portfolio publisher: schema validates generated output and rejects negative
  fixtures on every ordinary test run.
- Dependency setup: `python3 -m pip install -r requirements-dev.txt` installs
  only the pinned schema test validator; production Cargo dependencies do not
  change.
- Run focused Cargo tests, complete applicable Rust suite, shell/Node syntax,
  name preflight, strict OpenSpec validation, `git diff --check`, Gate dry-run,
  and full Forge Gate before archive. Record exact commands/results in HANDOFF.

## 9. Decision ledger

- **Resolved:** Forge owns the active schema subset, matching roadmap order 10
  and the consolidation direction. The sibling remains independent; no edits or
  archival are authorized here.
- **Resolved:** Job outcome is unsupported until it has a real Forge source
  record; retaining a schema alone is not an implemented contract capability.
- **Resolved:** Four families without active Forge behavior are explicitly
  unsupported. They are not deleted from the other project's catalog.
- **Resolved:** JSON Schema validation is test-only and offline. Production
  behavior retains its existing typed/serde validation and schema inspection.
- **Deferred:** Adoption by sibling language platforms or product repositories
  requires their own ordered changes and pinned Forge contract release.
- **No blocker:** No material ownership, runtime, or interface decision remains.
