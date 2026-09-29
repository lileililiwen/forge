# Design: project-query-consumer-surfaces

Status: implementation-ready planning package. No code is written by this
change.

## Implementation boundary

Repository `forge`, Rust 1.87+, existing CLI/MCP/API transports.

Files to **add**:

- `tests/project_query_surface_contract.rs` — CLI/MCP/API parity fixtures.

Files to **change** (additive only):

| File | Change |
|---|---|
| `src/main.rs` | `forge project list\|inspect\|tags\|languages` gains stable JSON/NDJSON pipeline output and shared filter flags |
| `src/mcp/mod.rs` | register `list_projects`/`inspect_project` catalog tools over the shared Core service |
| `src/api/mod.rs` | add `Route::CatalogQuery` (`GET /v1/projects/catalog`) with query parameters |

Do **not** touch: catalog field definitions or filter semantics (owned by
`project-catalog-query-contract`), provider behaviour, remediation, or semantic
approval. Transports stay thin.

## Language and runtime

Rust 1.87+, `rustfmt` defaults, no new dependency. Commands: `cargo test --test
project_query_surface_contract`, `cargo test --all-targets -- api mcp`, `cargo
clippy --all-targets -- -D warnings`.

## Ownership and shared code

Forge owns the shared Core query service in `src/catalog/`; every transport
**adapts** to it and none re-implements filtering. Authorization and
project-scope checks reuse the existing `authorize()` boundary; a catalog read
that exposes the fleet is admin-gated exactly like the interest reads.

## Behavioral model

All three transports call one function and serialize one `CatalogPage`.

| Transport | Shape |
|---|---|
| CLI | table (human), JSON (contract), NDJSON (one record per line, stable order) |
| MCP | one tool result whose content is the versioned JSON page |
| API | `200` with `{"catalog": <page>}` for a valid session; typed errors otherwise |

Pagination, sorting, filtering, source selection and freshness are parameters
carried identically on every transport; identical inputs produce byte-identical
JSON after normalizing the envelope.

## Contract and compatibility

The catalog contract is owned by `project-catalog-query-contract` and is not
redefined here. Errors reuse the existing mappings: `catalog-invalid` /
`api-invalid` → 400, `unknown-project` → 400, unauthorized → 401, project
mismatch → 403. The API envelope carries `API_CONTRACT_VERSION`; the catalog
payload carries `forge-project-catalog/0.1.0`.

## Failure and boundary policy

| Case | Result |
|---|---|
| Invalid filter via any transport | typed `api-invalid`/`catalog-invalid`, empty stdout on CLI |
| Empty result | `200` with an empty page on API/MCP; empty table on CLI; exit 0 |
| Unauthorized | `401`; the fleet composition is not disclosed |
| Project-scoped mismatch | `403` before any read |
| Transport disagrees with Core | a test failure; the Core service is the single source |

## Verification oracle

`tests/project_query_surface_contract.rs`: for a fixture catalog, assert the
CLI JSON, MCP result and API body describe the same records in the same order
after envelope normalization; assert pagination/filter/sort parameters behave
identically; assert NDJSON lines are a stable order; assert the negative
authorization and invalid-filter cases per transport. No checkbox without its
named test and captured output.

## Decision ledger

- One query implementation in Core; a second per-transport implementation is a
  defect, not a shortcut.
- Table output is never the machine contract.
- HTTP/MCP are optional; standalone CLI use remains complete.
