# Tasks: project-catalog-query-contract

Status: implemented, verified and archived on 2026-09-29 as
`2026-09-29-project-catalog-query-contract`. Every task below is checked
with its recorded evidence; the four ADDED requirements were promoted into
[openspec/specs/project-catalog-query-contract/spec.md](../../../openspec/specs/project-catalog-query-contract/spec.md).

## 1. BFS — Baseline and impact coverage

- [x] 1.1 Read `src/publish/inventory.rs`, `src/fleet/`, `src/registry/mod.rs` and the `forge list` command; record the existing source readers, normalized shapes and the exact reuse boundary.
  Evidence: `fleet::observe` (`src/fleet/mod.rs`) is the workspace-registry reader; `inventory::load_local`/`classify` (`src/publish/inventory.rs`) is the inventory reader; `Registry::list` (`src/registry/mod.rs:1050`) is the local reader. The catalog wraps all three and adds a declared-Git reader; it re-reads no file itself and never re-implements an existing reader.
- [x] 1.2 Confirm the implementation-handoff gate: Rust 1.87+, module paths `src/catalog/*`, CLI `forge project …`, no new dependency, no schema change.
  Evidence: `src/catalog/{mod,record,query,source}.rs` added; `forge project list|inspect|tags|languages` added to `src/main.rs`; `Cargo.toml` unmodified (no new dependency); no registry table or migration added — `Registry::open_read_only` is a new read-only constructor, not a migration.
- [x] 1.3 Map each requirement and scenario to its source adapters, query predicates, callers, failure cases and named tests.
  Evidence: mapping is `src/catalog/record.rs` (Requirement 1 records/provenance, closed `RECORD_KEYS`), `src/catalog/query.rs` (Requirement 2 filters/order/pagination), `src/catalog/source.rs` (Requirement 3 empty/stale/unavailable), `src/catalog/mod.rs` (Requirement 4 rendering, `redact_credentials`). Callers: `cmd_project` in `src/main.rs`. Named tests: `tests/catalog_contract.rs` (17) and `tests/catalog_cross_surface.rs` (9).

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Implement `CatalogRecord`, `SourceKind`, `EvidenceState` and provenance with a closed field set and refusal of unknown fields.
  Evidence: `record.rs` defines `RECORD_KEYS` (15 fields) and `CatalogRecord::from_value` refuses any key outside the set; `freshness_from` derives `Freshness` from `observed_at` at read time; `SourceKind` and `EvidenceState` are closed kebab-case enums. `the_record_field_set_is_closed` and `parses_a_well_formed_record` assert this.
- [x] 2.2 Implement source adapters over local registry, Git, workspace registry and explicit inventory; unreadable sources produce `unavailable` provenance.
  Evidence: `source.rs::collect` dispatches `SourceKind::Local` (read-only `Registry`), `Git` (declared working tree, `git` argument array), `WorkspaceRegistry` (`fleet::observe`), `Inventory` (`inventory::load_local`) and `Github` (unavailable — adapter owned by the next package). Each unreadable path pushes `SourceStatus { state: "unavailable", reason }` and no record. Asserted by `an_unreadable_source_is_named_and_the_others_still_report`, `the_git_source_reads_only_a_declared_working_tree`, `an_unselected_or_unconfigured_source_is_never_zero_rows_read_as_an_answer`.
- [x] 2.3 Implement the pure query model: AND filters, OR within a predicate, stable ordering by `(project_id, source)`, and deterministic pagination.
  Evidence: `query.rs::filter`/`apply` are pure; `CatalogQuery::from_pairs` validates keys. Asserted by `filters_compose_as_and_and_repeated_values_as_or`, `ordering_is_stable_by_project_then_source_whatever_the_selection_order`, `pagination_is_deterministic_and_walks_every_record_once` and the `query.rs` unit tests.
- [x] 2.4 Add `forge project list|inspect|tags|languages` with `--format table|json|ndjson` and repeatable filters; redact credentials on every path.
  Evidence: `ProjectCommands` and `CatalogFilterArgs` in `src/main.rs`; every record field passes through `clean_field` (`redact_credentials`). Asserted by `a_credential_shaped_value_never_reaches_any_output` and `the_catalog_advertises_its_grammar_without_creating_a_registry`.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Prove determinism across repeated reads and source permutations.
  Evidence: `ordering_is_stable_by_project_then_source_whatever_the_selection_order` (permuted `--source` order yields identical records) and `repeated_reads_of_unchanged_sources_are_byte_identical` (two reads differ only in the page's query instant).
- [x] 3.2 Prove the query is read-only: no registry byte, table or journal row changes.
  Evidence: `a_catalog_query_writes_no_registry_byte_table_or_journal_row` compares the database file bytes, `sqlite_master` table set, `projects` count and `operations` count before and after five catalog commands; `the_catalog_adds_no_table_to_the_registry` and `a_read_never_creates_the_registry_it_looked_for` complete the boundary.
- [x] 3.3 Exercise empty, stale, unavailable, duplicate-id, redaction and unknown-filter boundaries.
  Evidence: `an_empty_catalog_is_an_empty_page_with_exit_zero`, `a_stale_observation_is_reported_as_stale_not_as_current`, `an_unavailable_source_is_never_presented_as_a_healthy_one`, `sources_combine_without_silent_merging`, `a_credential_shaped_value_never_reaches_any_output`, `malformed_filters_and_bounds_are_typed_refusals_with_empty_stdout`.

## 4. Verification

- [x] 4.1 Run `cargo fmt/build/clippy/test`, the two new suites, `openspec validate --all --strict --no-interactive`, the name preflight and `git diff --check`; record the exact output.
  Evidence (2026-09-29): `cargo fmt --all -- --check` clean for every touched file (the pre-change baseline drift in `src/gate/evidence.rs`, `src/portfolio/share/validation.rs`, `src/publish/fleet.rs`, `tests/gate_contract.rs`, `tests/gate_cross_surface.rs`, `tests/publish_queue_status_contract.rs` was preserved by reverting the incidental `cargo fmt` edits with `git checkout --`). `cargo build` PASS. `cargo clippy --all-targets -- -D warnings` identical to the recorded 12-location baseline, zero new. `cargo test --workspace --all-targets --no-fail-fast -- --skip rust_scaffold_builds_and_tests_with_native_toolchain`: 84 result groups, 1888 passed, 1 failed — `fleet_online_routes_to_local_listener_when_alethefy_is_up`, proven pre-existing by `git stash push -u -- src tests` reproducing it on the untouched baseline (sandbox listener restriction, unrelated `fleet online` path). New supervised suites: 17 `catalog_contract`, 9 `catalog_cross_surface`, and the `src/catalog` unit tests. `node scripts/check-openspec-change-names.mjs` PASS; `openspec validate --all --strict --no-interactive` 57 passed, 0 failed (57 items); `git diff --check` and `git diff --cached --check` PASS.
- [x] 4.2 Record that no provider was contacted and no project was mutated; GitHub observation remains a separate package.
  Evidence (2026-09-29): no network call exists in `src/catalog/`. The Git source reads a local working tree through the `git` argument array; the workspace and inventory sources read local files; the local source opens SQLite read-only. Selecting `--source github` returns an explicit `unavailable` status naming the owning package and invents no record (`an_unselected_or_unconfigured_source_is_never_zero_rows_read_as_an_answer`). No project, registry row or journal row is created or changed by any catalog command.
