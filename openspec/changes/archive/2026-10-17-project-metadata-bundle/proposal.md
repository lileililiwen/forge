# Proposal: Generate and search Forge project metadata

## Why

Forge's catalog gives agents a closed, 15-field observation of a project, but it has no project description, keywords, declared capability/interface summary, documentation links, or specification index. The catalog contract intentionally rejects arbitrary metadata. Forge's own `.project.json` already contains a structured `discoverability` block and declared interfaces, while `forge.yaml` carries Forge-managed identity/profile facts. Agents currently must open arbitrary files or invoke separate commands to answer what a project does and which project owns a boundary.

Measured on 2026-10-17: the catalog has exactly 15 record keys (`src/catalog/record.rs:42-58`); no MCP tool reads `.forge/*`; the MCP registry has 18 tools and no project-metadata search. Forge's own `.project.json` already declares description, keywords, topics, audience, use cases, search phrases, interfaces, and capabilities (`.project.json:19-272`).

## What Changes

- Add a deterministic, versioned Forge project-metadata bundle generated for an explicit project directory from its `.project.json`, optional `forge.yaml`, README, and canonical OpenSpec specs.
- Store the generated artifact under `.forge/project-metadata.json` with an ownership receipt. First creation refuses an unowned conflicting target; refresh requires a valid matching receipt and refuses modified owned bytes.
- Add a read-only Core search over the local Forge registry's registered projects and their existing generated bundles; expose it through a CLI command and mature MCP tool. Search results retain project id/path and the metadata bundle's source revision. Missing bundles and unreadable/invalid bundles are reported per project, not silently treated as no matches.
- Keep the existing `forge-project-catalog/0.1.0` record closed and unchanged. Metadata has its own `forge-project-metadata/1.0.0` contract.
- Do not invoke CodeGraph or make its binary, index, workspace root, or MCP server a Forge prerequisite. The installed CodeGraph CLI is independently owned and its source repository/contract is unavailable in this workspace; a CodeGraph adapter is a separate integration package after its configuration and compatibility contract are verified.

## Package Boundary and Split Assessment

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `project-metadata-bundle` | Generate a reviewable per-project metadata artifact and let registered projects be found through Forge CLI/MCP | Forge, Rust Core/CLI/MCP | `.forge/project-metadata.json`, receipt, `forge-project-metadata/1.0.0`, read-only search | Existing `.project.json` declaration, Forge registry, catalog identity only | Build two metadata bundles; exact deterministic bytes; search returns only matching project with its provenance and reports missing/corrupt bundles |
| `codegraph-project-search-adapter` (follow-up; not part of this change) | Let Forge MCP query CodeGraph's symbol index for a selected project | Forge + CodeGraph owners, Rust and CodeGraph runtime | Optional bounded-argv adapter to CodeGraph's versioned CLI/MCP contract | CodeGraph owner contract/config discovery | Search symbol in fixture project with CodeGraph present and report unavailable without it |

These outcomes have separate owners and dependency lifecycles. This change is only the first package; it does not configure or initialize CodeGraph in sibling repositories.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| `workspace-governance` | `/home/paul/code/workspace-governance/schemas/project.schema.json`; `.project.json` `discoverability`, `interfaces[]` | Canonical project description, keywords, audience, use-cases, capability/interface ownership vocabulary | It owns declarations and fleet audit, not generated per-project Forge metadata or Forge's local registry/MCP | Workspace Governance owns declaration schema and registry; Forge owns its generated bundle and consumer API | **adapt through a generic adapter** — read the explicit declaration as input; do not edit sibling schemas or duplicate their authority |
| Forge catalog | `src/catalog/record.rs:42-58`; `openspec/specs/project-catalog-query-contract/spec.md` | Read-only source provenance, project identity, credential redaction, closed contract, source composition | The catalog intentionally lacks semantic descriptions and arbitrary metadata; changing it would break its stable contract | Forge Core owns catalog contract; existing consumers retain v0.1.0 | **keep local** — add a separate metadata contract and use catalog identities without changing catalog records |
| CodeGraph | Installed `codegraph 1.6.2`; `codegraph query --help` and observed JSON output; `/home/paul/code/REPORT-forge-sibling-cooperation.md:184-192` | Existing symbol index, `query -j -p <path> <search>`, independently served MCP | No CodeGraph source repository or published versioned adapter contract was found; configuring or initializing its index is an independent lifecycle | CodeGraph tool owner; no Forge release dependency | **keep local** for this change; an optional adapter is the separately listed follow-up |

No shared library extraction is proposed: the workspace declaration schema already has an owner; this package adds Forge-owned generation, receipt, indexing and search behavior. Forge MUST continue working without any sibling checkout or CodeGraph executable.

## User Experience and Interface Impact

Actors: a developer adopting a project and an AI coding agent searching registered projects. The developer runs `forge project metadata build <path>` to create or refresh the bundle; the tool explains whether it was created, refreshed, refused because the target is unowned/modified, or could not parse a named input. The agent uses the read-only `search_project_metadata` MCP tool (also available as `forge project metadata search`) with a non-empty query and optional project id filter. Results show project id, matching metadata fields, source revision, and a per-project issue for missing or invalid bundles. Empty results are explicit. No web or browser UI changes; CLI help and MCP tool descriptions/argument schemas are the interface. Inputs are typed data, never shell fragments. No CodeGraph setup or hidden workspace discovery occurs.

## BFS Impact Map

| Area | Impact |
|---|---|
| Capability and contracts | New `project-metadata-bundle` capability and independent versioned metadata contract; existing catalog contract stays unchanged |
| Inputs and generation | Explicit project path; read `.project.json`, `forge.yaml` if present, README and OpenSpec specs without traversing parent directories; deterministic normalized content |
| Artifact and persistence | `.forge/project-metadata.json` plus receipt, only in the selected project; no SQLite migration, journal writes, registry mutation, or fleet writes |
| Search | Core reader queries registered project paths from the existing local registry; no implicit workspace registry/source; deterministic ranking and per-project failure state |
| Interfaces/callers | New CLI nested command and one read-only MCP tool; MCP `tools/list` advertises it with the same Core semantics |
| Security/privacy | Bound paths to explicit registered project roots; redact credential-shaped text before serialization; limit input/output size; no subprocess/network |
| Compatibility | Preserve catalog keys, CLI output and existing MCP tools; missing artifacts are not empty matches and never become healthy evidence |
| Tests | Determinism, ownership receipt, conflict refusal, malformed input, symlink/path boundary, stale revision, ranking tie, no-match and per-project failure; CLI/MCP transport parity |
| UI/accessibility | CLI/MCP-only; no browser surface. JSON and human CLI output are both usable; stable names, explicit empty/unavailable states |
| Dependencies | None at runtime beyond Forge; CodeGraph adapter explicitly deferred |

## Capabilities

- `project-metadata-bundle` (new): deterministic metadata artifact generation and registered-project search through CLI and MCP.

## Non-goals

- No modification to any sibling repository, workspace registry, declaration schema, CodeGraph index, or MCP configuration.
- No automatic CodeGraph installation, indexing, synchronization, or process invocation.
- No semantic/vector search, AI-generated descriptions, or ingestion of source-code contents; search is deterministic over declared metadata and explicit documentation/spec labels.
- No change to `CatalogRecord`, its 15-key closed set, catalog filtering, workspace discovery, or project registration behavior.
- No dashboard/page or generated-project lifecycle expansion beyond the explicit metadata build command.
- No new database, background daemon, network request, or mutable global index.