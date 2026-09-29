# Proposal: project query consumer surfaces

## Why

The catalog and filter engine should be usable by Forge itself, other CLIs,
automation, MCP clients, and HTTP consumers without each caller reimplementing
project discovery or interpreting table output. A shared Core service gives all
surfaces identical results and failure semantics.

## What Changes

- Expose the catalog query contract through CLI, MCP, and HTTP API surfaces.
- Keep CLI table/JSON/NDJSON output stable for shell pipelines and automation.
- Add transport-neutral pagination, sorting, filtering, source selection, and
  observation freshness parameters.
- Preserve authorization and project-scope checks across every transport.
- Return typed errors and equivalent result envelopes across CLI, MCP, and API.

## Package Boundary and Split Assessment

This package owns transport exposure only. It does not redefine catalog fields,
filters, provider behavior, remediation, or semantic approval. The query
engine remains in Forge Core and the transports remain thin adapters.

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `project-query-consumer-surfaces` | Expose one query service consistently | Forge Rust CLI/MCP/API | Shared Core query service | Catalog query contract | Cross-surface parity tests |
| `project-catalog-query-contract` | Own records and filters | Forge Rust Core | Catalog contract | Existing inventory/fleet readers | Catalog fixtures |
| `project-evidence-gap-assessment` | Supply optional gap fields | Forge Rust doctor | Evidence contract | Catalog contract | Finding parity tests |

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Forge API | `src/api/` | Existing request parsing, auth, and project-scoped responses | Needs catalog query endpoint and shared validation | Forge | **Extend shared owner** |
| Forge MCP | `src/mcp/` | Existing JSON-RPC tool registration and error mapping | Must consume Core query service, not duplicate logic | Forge | **Extend shared owner** |
| Forge CLI | `src/main.rs` | Existing output formats and typed command errors | Needs composable NDJSON and filter flags | Forge | **Extend shared owner** |
| Other CLIs | External consumers | JSON/NDJSON process boundary | Version and compatibility must be explicit | External consumers | **Adapt through a generic adapter** |

## BFS Impact Map

| Surface | Impact |
|---|---|
| CLI | `forge project list|inspect|tags|languages` and pipeline-safe output |
| MCP | Project query/list/inspect tools using the same Core service |
| HTTP | Read-only project catalog endpoint with query parameters |
| Contract | Stable serialization, error envelope, ordering, and pagination |
| Failure | Invalid filters, unauthorized scope, unavailable provider, stale source, empty result |
| Security | Project scoping and redaction must match existing transport policy |
| Tests | Golden output and cross-surface equivalence fixtures |
| Unaffected | Remediation writes, GitHub mutation, deployment, and semantic review |

## Capabilities

- `project-query-consumer-surfaces`: transport adapters for the stable project
  catalog query contract.

## Non-goals

- Adding a second query implementation per transport.
- Making HTTP or MCP required for standalone Forge CLI use.
- Mutating projects or external providers from read-only query calls.
- Returning table output as the machine contract.

Source: requirement.md §32, §35, §36, §41.
