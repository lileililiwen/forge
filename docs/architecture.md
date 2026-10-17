# Architecture and decision boundaries

## Product constraints

Forge starts as a modular monolith. Core owns project identity, manifests, capability contracts, validation and operation outcomes. The product model is independent of stack, AI provider, IDE, cloud and hosting. Generated projects retain ordinary owned source, native build/test commands and no required Forge runtime.

Deterministic assets are preferred at every reuse level: Profile → Feature → Pattern → Capability. Natural language becomes validated Intent, then a reviewable capability graph and pinned plan. Only unresolved glue, business logic and semantic migrations go to AI/spec work. Natural language never becomes raw shell commands.

## Module ownership

| Boundary | Ownership | Consumers |
| --- | --- | --- |
| Core / Registry | Validated project model, schema, identity, state and timestamped observations | All operations |
| Profiles / Features / Components | Versioned assets, contracts, compatibility, provenance and quality evidence | Planner, Generator, Upgrade, Doctor |
| Intent / Planner | Structured intent, graph validation, exact resolution and reviewable steps | CLI, later MCP/API |
| Generator / Upgrade | Deterministic application, ownership records, preconditions and recovery | New, feature lifecycle, upgrades |
| Policies / Doctor | Maturity assessment, local inspection and normalized external policy evidence | CLI, release, later transports |
| Agent / Integrations | Existing PTY manager, DriftWatch, content and analytics adapters | Spec execution, quality and observations |
| Gate Runtime Consumption | Driftwatchdog owns plan resolution, execution, blocking policy, its own run history and exit semantics; Forge owns bounded invocation, revision-bound evidence, journaling and honest surface projection; the sibling's `gate evidence-export` verb produces a versioned document that Forge consumes (vocabulary gate, revision gate, attribution, contradiction rule, atomic persistence, `forge gate evidence` read surface) | CLI (`forge gate`, `forge gate evidence`), Doctor, Release |
| Distribution / Release / Deployment | Explicit external operations and durable per-stage outcomes | CLI, later mature MCP/API |
| CLI / MCP / API / Portal | Input/output transport only; no duplicated business rules | Humans, agents, portal views |

The [requirement.md](../requirement.md) §38 tree is logical guidance; the Core modules above implement it.

## Technology direction

The brief suggests Rust Core/CLI, SQLite initially, YAML manifests and stdio MCP. These are the implemented baseline, recorded with exact toolchain, crate boundaries, schema/parser and SQLite strategy in the foundation ADR ([ADR 0001](adr/0001-foundation-toolchain.md)). PostgreSQL is conditional on demonstrated need. The portal ships as a read-only Core/CLI data surface that points at the SPA as the single interactive browser surface ([portal–SPA](portal-spa.md)); an ASP.NET Core or Next.js graphical portal remains a deferred downstream choice.

Adopt `forge.yaml` as canonical. An explicit legacy import may read `platform.yaml`, but ambiguous coexistence blocks mutation. Schema versions must be supported explicitly. Manifests describe desired infrastructure; observations include source revision, time and actual verification state. Credentials are references to external secret storage, not embedded tokens.

## Mutation and failure model

Validate inputs and compatibility before side effects. Use typed arguments, constrained paths, pinned assets, explicit destinations and project-scoped authority. Preserve user-owned files and unrelated Git changes. Plan destructive or remote operations with exact targets and recovery implications.

Stage file generation and validate before promotion. Reconcile manifest and registry partial writes through operation records. Record per-project and per-provider outcomes for upgrades, releases and deployment. External publication and data migrations are not generally reversible; never advertise universal rollback.

Retries verify input hashes, source revisions and completed stages. Stale plans require review again. Missing tools, unsupported capabilities, inaccessible paths, unavailable integrations and stale observations must not become PASS, healthy or complete.

## Integration boundaries

DriftWatch owns the quality/policy engine; its actual interface is inspected when integrating. The existing PTY manager owns process/session mechanics for OpenCode, Codex and future agents. Existing GitHub analytics and unified content systems retain their identities. The existing Codex/OpenSpec proposal workflow owns bounded semantic-change authoring.

Transport parity is delivered: CLI first, one mature stdio MCP server, an authenticated HTTP API and read-only portal views afterward, all dispatching through the same Core contracts. SSO uses standard OIDC, project-specific sessions and independent authorization; no shared cookies across unrelated apps.

## Plugin boundary declaration

The plugin registry (`forge plugins list`, `forge cap list|inspect`, and
`GET /v1/admin/projects/{id}/plugins`) resolves an id's kind and
capabilities from the workspace's own declaration first, and reports
**which source answered every row** as `source: descriptor | declared |
manifest | builtin`:

| Rank | Source | Meaning |
| --- | --- | --- |
| 1 | `providers.yaml` `plugins:` descriptor | operator configuration for this installation |
| 2 | the inspected project's declaration (`interfaces[]`, then `capabilities`, then `integrations`) | the provider's own statement of its boundary |
| 3 | `kits/manifest.json` `plugins:` | bundled kit metadata |
| 4 | the builtin table in `src/plugins/mod.rs` | last-resort fallback for an id nothing declares |

A lower rank never overrides a higher one; a field the answering rank does
not state is *filled* from the next rank that states one, which is what
keeps a declared row behaviourally equal to the builtin row for the same
id. An id nothing declares keeps resolving through the builtin table,
unchanged.

The declaration format is `workspace-governance`'s
(`schemas/project.schema.json`). Forge reads a project's `.project.json`
from the directory it inspects, fetches nothing and writes no declaration
into another project, so the no-sibling invariant holds. A declaration is
**readable or refused**: an unparseable file, a `mechanism` outside the
closed set (`invoke|consume|observe|sibling`), a version-less `contract` on
a non-`sibling` interface, or a `consume`/`observe` entry without a
`digest` each refuse with the stable code `plugin-boundary-refused`, naming
the file or the field, and nothing is resolved from that declaration. A
configured descriptor that *contradicts* what the project declares refuses
by name rather than choosing one. An absent or empty `interfaces` array is
valid and silent.

### Forge's own declaration

`forge/.project.json` declares Forge's boundaries with contract versions
derived from the versioned-surface inventory (`CONTRACTS` in
`src/contract/mod.rs`, the adapter contract constants and
`PLATFORM_FAMILY_VERSIONS`) rather than restated by hand;
`tests/plugins_declaration_contract.rs` asserts the shipped declaration
equals that derivation. A row the inventory registers with a wire
discriminator is named by it, a `platform_family` row by
`<family>/<platform_family_version>`, and any other row by its canonical
spec id with the row's registered version.

- **Provides** (Forge owns the contract and offers the entry):
  `gate-result` (`platform.gate-result/1.0.0`, `forge gate --format json`),
  `contract-envelope` (`platform-contract-consumption/0.1.0`,
  `forge contract emit <family> <target>`), `mcp-surface`
  (`mature-mcp-surface/0.1.0`, `forge mcp serve`), `publish-provider`
  (`forge-publish-provider/0.1.0`, `./adapters/<provider-id>`),
  `deploy-executor` (`forge-deploy-executor/0.1.0`, the reference executor's
  bounded argv) and `checker-document` (`forge-checker/0.1.0`, observing the
  canonical checker-emission spec).
- **Consumes** (siblings own these): `policy-checker-report`
  (`driftwatch-checker/0.1.0`, driftwatchdog),
  `governance-observation` (`governance-provider-contract/0.1.0`,
  workspace-governance), and the retained copies below.
- **Integrations.** `ariadex` and `sisyphusfy` are declared as optional
  integrations with an `evidence_ref`, and **not** as `interfaces[]`:
  neither runtime has a contract version on either side (they are invoked by
  argv and their status documents are discriminated by shape), and the
  schema requires a versioned contract for every non-`sibling` mechanism —
  declaring them as interfaces would mean inventing one. The entry keeps the
  boundary and its missing version visible to governance, and both ids
  resolve as `declared` in the plugin registry. The *gate* evidence-export
  document is in the same position: it is versioned by a bare
  `schema_version` integer, so it is reached through driftwatchdog's
  versioned `driftwatch-checker/0.1.0` check boundary instead.
- **Ownership of the adapter contracts.** `publish-provider` and
  `deploy-executor` are declared `provides`, not `consumes`: both contract
  constants are declared in this repository and an adapter *implements*
  them, while a consumption must name the single provider it consumes from
  — which no project is, because every installation configures its own
  adapters.
- **Retained copies** are declared `consume` edges, each naming provider,
  contract, version and the digest of the bytes it keeps. The two
  manifest-pinned trees are named by their pin document
  (`contracts/manifest.json`, `kits/manifest.json`, digest pinned), and the
  test proves every file under those trees is recorded in the pin document
  the edge names — so no copy is unaccounted for. The un-manifested
  `scripts/governance/**` copies, the `scripts/vocabulary.json` list and
  both copies of the governance vocabulary are each declared with their own
  sha256; the *third* copy of the secret-field vocabulary
  (`contracts/vocabulary/secret-field-substrings.json`) is one declared
  edge against `platform-contracts` instead of an undeclared sibling copy.
  Where an artifact publishes no release version, the declaration names the
  contract after the interface id and records the artifact's own schema
  version (`1` → `1.0.0`); it never asserts a release the provider does not
  publish, and the kit feed — whose pin document does record a release
  version — is named `dotnet-platform-libs/0.2.0`.

## Evidence

Test deterministic output and repeatability, compatibility rejection, path confinement, partial writes, stale plans and per-project/provider failures. Run generated-project native builds for each advertised profile. Integration fixtures prove contracts; successful real adapter runs establish runtime integration. Neither OpenSpec validation nor build-only skeletons establish product completion.
