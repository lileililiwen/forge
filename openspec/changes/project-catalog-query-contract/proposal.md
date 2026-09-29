# Proposal: versioned project catalog and composable query contract

## Why

Forge can already observe selected fleet and inventory sources, but it does
not provide one stable catalog contract for locating projects, combining local
and external observations, or piping filtered results into other commands.
Without that contract, Labrys, Hermora, Hypora, and future repositories remain
dependent on manual context and project-specific commands.

## What Changes

- Define `forge-project-catalog/0.1.0` as the normalized project record
  contract for local, Git, GitHub, workspace-registry, and explicit inventory
  sources.
- Add a Core query model for tags, languages, profile, lifecycle, repository,
  CI state, Compose state, and evidence state.
- Add CLI `forge project list|inspect|tags|languages` with table, JSON, and
  NDJSON output and repeatable filters.
- Preserve source provenance, observation timestamps, unavailable sources, and
  deterministic ordering in every result.
- Keep query operations read-only; remediation and provider writes remain
  separate changes.

## Package Boundary and Split Assessment

This package owns one outcome: a stable, read-only catalog and query contract.
It excludes gap detection, file mutation, semantic generation, GitHub writes,
and MCP/API transport exposure. Those outcomes have independent state,
authorization, and verification boundaries and are separate packages below.

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `project-catalog-query-contract` | Query a normalized project catalog | Forge Rust Core/CLI | `forge-project-catalog/0.1.0` | Existing inventory/fleet readers | Fixture catalog and CLI JSON/NDJSON contract tests |
| `project-evidence-gap-assessment` | Explain missing project evidence | Forge Rust doctor | `forge-project-evidence/0.1.0` | This package | Finding fixtures and doctor parity tests |
| `project-local-remediation-plans` | Plan/apply safe local repairs | Forge Rust remediation | `forge-remediation-plan/0.1.0` | Gap assessment, standard packs | Diff, conflict, rollback tests |
| `project-semantic-description-review` | Review semantic metadata proposals | Forge Rust plus optional agent provider | Proposal records | Gap assessment | Approval and provenance tests |
| `github-project-metadata-adapter` | Observe and optionally review GitHub changes | Forge Rust adapter | GitHub adapter protocol | Catalog and remediation contracts | Mock provider and no-credential tests |
| `project-query-consumer-surfaces` | Expose the query contract to transports | Forge Rust CLI/MCP/API | Shared Core query service | Catalog contract | Cross-surface parity tests |

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Forge registry | `src/registry/` | Existing project identity, observations, and SQLite patterns | Current records are registry-oriented, not a multi-source catalog | Forge | **Extend shared owner** |
| Fleet observation | `src/fleet/`, `fleet-registry-observation` | Read-only external registry projection | No general filter/query contract | Forge | **Extend shared owner** |
| Portable inventory | `src/publish/inventory.rs` | Versioned normalized inventory and explicit source selection | Deployment fields are narrower than catalog fields | Forge | **Adopt** |
| workspace-governance | External registry/adapter | Portfolio policy and declarations | Must remain optional and root-portable | workspace-governance | **Adapt through a generic adapter** |

## BFS Impact Map

| Surface | Impact |
|---|---|
| Catalog | New normalized records, source provenance, and observation freshness |
| Query | New filters for tags, language, lifecycle, profile, CI, Compose, and evidence |
| CLI | New `forge project` read commands with stable machine output |
| Persistence | No mandatory schema migration; source snapshots may remain provider-owned |
| Integrations | Local/Git/GitHub/workspace providers are optional and isolated |
| Failure | Invalid records, duplicate identities, unavailable sources, and empty results are explicit |
| Security | No implicit parent-directory scan, credential display, or provider write |
| Unaffected | Deployment execution, semantic descriptions, file remediation, and GitHub mutation |

## Capabilities

- `project-catalog-query-contract`: normalized project catalog and composable
  read-only query surface.

## Non-goals

- Automatically importing every discovered project into the Forge registry.
- Editing repositories or GitHub metadata.
- Treating GitHub tags as portfolio classification tags.
- Declaring a project healthy, deployable, or production-ready from metadata.

Source: requirement.md §7, §32, §36.
