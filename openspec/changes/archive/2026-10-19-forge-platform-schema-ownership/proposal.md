# Proposal: own Forge's active platform schemas

## Why

Forge currently treats a checked-in copy as a mirror of a separate
`platform-contracts` repository. CI and release checks fetch that sibling
source, even though Forge's own commands read the checked-in bytes. The copied
registry also advertises families Forge does not consume, and one producer
contract test is ignored unless the sibling checkout and Python validator are
installed. This makes the local product boundary dependent on another project
and obscures which schema behavior Forge promises.

## What Changes

- Make `contracts/` the maintained source for the platform schema families
  Forge actually uses: Gate result, readiness, release evidence, capability,
  audit event, and public portfolio manifest. Keep the shared envelope and a
  Forge-scoped registry needed by validators and inspection.
- Remove inactive identity-subject, permission-decision, tenant-context, and
  job-outcome schemas from Forge's supported registry and classify those
  families explicitly as unsupported, with a reason and recovery direction.
- Replace upstream revision/provenance claims with an offline Forge-owned
  manifest that hashes every maintained contract asset and verifies without a
  sibling checkout or network.
- Package the maintained contract assets beside released Forge binaries and
  resolve them from the installed executable, so published contract commands
  work outside the source checkout.
- Remove the upstream parity script/job and sibling-path behavior from release
  checks. Replace the ignored portfolio schema test with an always-running
  test against the in-tree schema and fixtures.
- Keep CLI and authenticated assurance browser discovery consistent: supported
  families remain inspectable; unsupported families are listed with an
  explicit status and cannot be emitted or validated as supported.

## Package Boundary and Split Assessment

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `forge-platform-schema-ownership` | Forge contract operations are offline, Forge-maintained, and expose only supported families | Forge; Rust, JSON Schema, JavaScript workflow YAML, POSIX shell | `contracts/` maintained schema set, manifest, CLI/API catalog, release and CI checks | Existing `platform-contract-consumption`, portfolio manifest, Gate projection | All supported-family tests pass without sibling source; unsupported families are explicitly reported; CI/release checks contain no source fetch or parity path |

The work shares one ownership and inventory lifecycle. The CLI, authenticated
assurance API, tests, and CI all read the same family registry and manifest, so
splitting schema ownership from consumer behavior would leave an incoherent
intermediate state. The separate `platform-contracts` repository is outside
this package and will not be modified.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Platform Contracts | `/home/paul/code/platform-contracts/README.md`, `schemas/registry.json`, `manifest.json`, `docs/migration.md` | Language-neutral JSON Schema and fixture conventions; existing Forge mirror contains the exact starting bytes | Its stated owner scope includes families with no Forge consumer; its source checkout is currently a hard CI/release dependency | Platform Contracts remains an independently maintained project; Forge owns only the subset its runtime surfaces promise | **keep local** |
| Forge contract consumer | `src/contract/mod.rs::{supported_families,family_schema_path,validate_envelope}`, `src/portfolio/share/manifest.rs`, `src/api/admin/assurance.rs` | Existing Rust producers, validators, registry-backed CLI and browser projections | Inventory includes `platform.job-outcome` without a source record and mirror includes three unconsumed families | Forge owns its schemas, validation and release behavior with the Forge binary | **extend shared owner** |
| Workspace Governance in Forge | `modules/workspace-governance/vocabulary.json`, `contracts/vocabulary/governance-vocabulary.json` | Existing independently sourced governance vocabulary and module tests | It is not a platform schema family and must not be mislabeled as Platform Contracts data | Forge owns this in-tree module and its vocabulary | **keep local** |

The sibling was inspected read-only. Its README confirms the project owns
language-neutral schemas and fixtures as a separate contract source. Forge's
maintained subset starts from the checked-in copies, but future changes are
reviewed and released in Forge without parity or source-path claims. No sibling
source, runtime, or Git submodule dependency is introduced.

## User Experience and Interface Impact

Forge maintainers use `forge contract list`, `inspect`, `emit`, and `validate`
and the authenticated assurance browser's contract list and inspect views.
Supported families retain their current schemas and projections. Both
surfaces show an explicit unsupported status and reason for excluded families;
emit, inspect-schema, and validate requests for unsupported families return a
bounded actionable refusal. Unknown family IDs retain the current safe generic
unknown response. The browser must keep its existing authentication boundary,
keyboard behavior, responsive layout, and error presentation; the contract
detail view must distinguish unsupported from unknown without echoing unsafe
input. Browser tests cover list, inspect, unsupported, and refusal states.

There is no new route or navigation item. This changes content and states on
the existing contract list/detail surfaces. The existing authenticated
maintainer flow is the actor journey; verification includes the real browser
contract test plus API tests. No new responsive layout is introduced, but
existing narrow layouts and semantic status labels must remain usable.

## BFS Impact Map

| Surface | Impact and invariant |
|---|---|
| Schemas and registry | Keep only the six active families plus the common envelope; preserve current wire bytes for retained families. Explicitly name each unsupported family, reason, and adoption condition. |
| Manifest, packaging and provenance | Hash all retained schemas, registry, fixture assets, and Forge-owned vocabulary. Package the assets beside the executable and resolve them without source paths. Report Forge ownership and asset digests; never claim upstream parity. Missing, malformed, extra, or changed files fail closed. |
| Rust contract module | Make supported/unsupported status a single source used by version lookup, schema lookup, validation, CLI output, and API projection. No family may be advertised as supported without a schema and validator path. |
| Portfolio publisher | Validate generated public manifests against the in-tree maintained schema and canonical fixture set; no ignored or sibling-dependent acceptance test. |
| CLI and web assurance | Preserve supported-family behavior and authentication. Add explicit unsupported catalog/detail state, safe diagnostics, and no emission/validation fallthrough. |
| CI and release checks | Remove platform-contracts checkout, parity script, env override, and optional sibling-path parity. Run offline manifest/schema/fixture checks as required checks. |
| Compatibility | Retained JSON wire shapes and discriminators stay byte compatible. Removed families are a deliberate Forge support boundary and return typed unsupported responses. No sibling files change. |
| Security and failure | Hash mismatch, malformed schema/manifest, invalid fixture, or unsupported operation fails closed; no fallback to network, embedded alternate copy, or self-parity. Secret-field redaction remains applied to emitted audit data. |
| Tests | Rust contract/API tests, public portfolio producer validation, CLI and web contracts, script syntax, workflow/release-check contracts, and all-family manifest digest checks. |

## Capabilities

- `platform-contract-consumption` (Forge ownership and supported-family boundary)
- `artifact-and-ci-baseline` (offline schema validation in the required check graph)

## Non-goals

- Editing, archiving, publishing, or changing the policy of `/home/paul/code/platform-contracts`.
- Migrating .NET or Rust platform libraries or sibling product consumers.
- Changing the wire shape or version of any retained family.
- Implementing identity, authorization, tenant, or job-outcome producers.
- Adding a generic JSON Schema runtime dependency to Forge production code.
- Adding new contract routes or an unauthenticated schema browser.
- Publishing Forge or contract assets as a separate package in this change.
