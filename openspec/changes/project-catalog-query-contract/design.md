# Design: project-catalog-query-contract

Status: implementation-ready planning package. No code is written by this
change; it defines the contract the following packages consume.

## Implementation boundary

Repository `forge`, Rust 1.87+, existing Cargo workspace and SQLite registry.

Files to **add**:

- `src/catalog/mod.rs` — capability module root and re-exports.
- `src/catalog/record.rs` — the normalized `CatalogRecord`, its closed enums
  and source provenance vocabulary.
- `src/catalog/query.rs` — the composable filter/order/pagination model and
  its pure `apply(records, query) -> CatalogPage` function.
- `src/catalog/source.rs` — trait-style adapters over the existing local
  registry, Git, workspace registry and explicit inventory readers.
- `tests/catalog_contract.rs`, `tests/catalog_cross_surface.rs`.

Files to **change** (additive only):

| File | Change |
|---|---|
| `src/lib.rs` | `pub mod catalog;` |
| `src/main.rs` | `Project` command tree: `list`, `inspect`, `tags`, `languages` with `--format table\|json\|ndjson` and repeatable filters |

Do **not** touch: `src/registry/mod.rs` migrations, `src/publish/inventory.rs`
(writer), deployment, MCP/API transports (owned by
`project-query-consumer-surfaces`), generation or publish.

## Language and runtime

Rust 1.87+, `rustfmt` defaults, no new dependency (`serde`, `serde_json`,
`chrono` already present). Local commands:

```text
cargo fmt --all -- --check
cargo build
cargo test --lib -- catalog
cargo test --test catalog_contract
cargo test --test catalog_cross_surface
cargo clippy --all-targets -- -D warnings
```

## Ownership and shared code

Forge owns the catalog contract. The module **adopts** the normalized shape
already used by `src/publish/inventory.rs` and the read-only fleet projection in
`src/fleet/`, wrapping them as catalog *sources* rather than re-reading files
itself. No sibling repository is modified. A future Git host adapter imports
this contract; it does not redefine it.

## Behavioral model

A `CatalogRecord` is generated, not stored. Each source adapter yields records
carrying `source`, `source_revision`, `observed_at` and a `freshness`
(`current|stale|unknown`) derived at read time — never a stored flag.

```rust
pub enum SourceKind { Local, Git, WorkspaceRegistry, Inventory, Github }
pub enum EvidenceState { Present, Absent, Stale, Unavailable, Unverified }
```

Query is a pure function over an already-collected record set:

| Stage | Rule |
|---|---|
| Collect | union of selected sources; a source that cannot be read contributes `Unavailable` provenance, not zero rows |
| Filter | tags, languages, profile, lifecycle, repository, ci, compose, evidence — every predicate is AND; a repeated filter is OR within that predicate |
| Order | stable by `(project_id, source)`; never by observation time |
| Paginate | `limit` + opaque `cursor`; deterministic regardless of source order |

## Contract and compatibility

`forge-project-catalog/0.1.0`. Record fields are the manifest/registry facts
plus provenance; there is no free-form metadata map (an unknown field is
refused, matching the interest store's closed-key discipline). Errors are typed:
`catalog-invalid` for a malformed filter, `unknown-project` for `inspect`.
Table output is human-facing and explicitly **not** the machine contract; JSON
and NDJSON are.

## Failure and boundary policy

| Case | Result |
|---|---|
| No projects registered | empty page, exit 0, explicit `"records": []` |
| Unknown filter key | `catalog-invalid`, empty stdout |
| Unreadable source | records from other sources plus an `unavailable` provenance entry |
| Duplicate id across sources | both retained, disambiguated by source; never merged silently |
| Credential in a source value | redacted via `policy::redact_credentials` before output |
| Parent-directory scan | never performed implicitly; only declared sources are read |

## Verification oracle

`tests/catalog_contract.rs`: fixture-driven filter/order/pagination parity and
NDJSON/JSON equivalence; `tests/catalog_cross_surface.rs`: read-only proof (no
registry byte, table or journal row changes), provenance retention, redaction,
and empty/unavailable boundary cases. No task is checkable without its named
test and a recorded command output.

## Decision ledger

- The catalog is a **projection**, never a second registry: no schema change,
  no migration, and it never imports discovered projects automatically.
- GitHub is one optional source; local-only Forge remains fully functional.
- GitHub topics/release tags and Forge portfolio tags are separate namespaces.
