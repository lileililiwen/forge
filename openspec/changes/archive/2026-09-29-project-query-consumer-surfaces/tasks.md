# Tasks: project-query-consumer-surfaces

Status: planning-only package. Every task is unchecked by design.

## 1. BFS — Baseline and impact coverage

- [x] 1.1 Read `src/api/mod.rs` route/authorization, `src/mcp/mod.rs` tool registration and the `forge project` CLI; record the shared Core entry point and the existing auth/error mappings to reuse.
- [x] 1.2 Confirm the implementation-handoff gate: Rust 1.87+, one Core query service, thin transports, no redefined catalog fields, no new dependency.
- [x] 1.3 Map each requirement and scenario to its transport, parameter, authorization arm, failure case and named test.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Add stable JSON/NDJSON output and shared filters to `forge project list|inspect|tags|languages`.
- [x] 2.2 Register MCP catalog tools over the Core service with typed error mapping.
- [x] 2.3 Add `GET /v1/projects/catalog` with shared query parameters and the `authorize()` boundary.
- [x] 2.4 Ensure every transport serializes the Core `CatalogPage` unchanged.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Prove CLI/MCP/API parity for records, ordering, pagination and filters.
- [x] 3.2 Exercise invalid-filter, unauthorized, project-mismatch and empty-result boundaries on each transport.
- [x] 3.3 Prove no second filtering implementation exists and the table layout is not load-bearing.

## 4. Verification

- [x] 4.1 Run `cargo fmt/build/clippy/test`, the new parity suite, `openspec validate --all --strict --no-interactive`, the name preflight and `git diff --check`; record the exact output.
- [x] 4.2 Record that no provider was contacted and no query mutated state; read-only parity is the whole claim.

## Evidence

### Cross-surface parity (3.1, 3.2)

`tests/project_query_surface_contract.rs` runs eight cases against the
built binary on local fixtures only (no network, no model, no
registry mutation):

- `empty_catalog_returns_empty_page_on_every_transport`
- `cli_mcp_api_describe_the_same_records_in_the_same_order`
- `cli_mcp_api_describe_the_same_records_for_inspect`
- `ndjson_lines_are_in_deterministic_core_order`
- `filter_parameters_are_carried_identically_across_transports`
- `invalid_filter_is_a_typed_refusal_on_every_transport`
- `unauthorized_caller_sees_no_fleet_on_the_api`
- `unknown_project_returns_unknown_project_on_every_transport`

`cargo test --test project_query_surface_contract -- --test-threads=1`
→ 8 passed; 0 failed.

`cargo test --test mcp_contract` → 15 passed; 0 failed (4 cases
updated to the new `result["catalog"]["records"]` shape and
`project_id` field).

`cargo test --test api_contract` → 11 passed; 0 failed.

### Single filtering implementation (3.3)

`rg "fn apply\(|fn filter\(|from_pairs" src/catalog` →
`src/catalog/query.rs`, `src/catalog/record.rs`, `src/catalog/mod.rs`.
No transport re-implements filtering: `src/api/mod.rs`
(`handle_catalog_query` + `catalog_filter_pairs_from`) and
`src/mcp/mod.rs` (`mcp_list_projects` + `catalog_filter_pairs_from_mcp`)
both delegate to `catalog::CatalogQuery::from_pairs` /
`catalog::apply`. CLI already wired through `cmd_project_list` /
`cmd_project_inspect` in the catalog service introduction.

### Build / fmt / clippy / test (4.1)

- `cargo build` → `Finished dev profile` (only pre-existing
  warnings on `FleetEntryOutcome::Published` / `Failed.journal`).
- `cargo fmt --all` → clean; pre-existing formatting drift on
  `src/gate/evidence.rs`, `src/portfolio/share/validation.rs`,
  `src/publish/{fleet,mod}.rs`, `src/api/ui/auth.rs`,
  `src/github/{adapter,normalize,mod}.rs` and the related test
  files was reverted with `git checkout --` to preserve the
  baseline per the project rules.
- `cargo clippy --all-targets -- -D warnings` → only the
  pre-existing baseline errors
  (5 in lib, 9 in lib-test, all on the files listed in the
  baseline); verified by `git stash` and re-run. No new clippy
  errors introduced.
- `cargo test --test project_query_surface_contract` →
  8 passed; 0 failed.
- `cargo test --workspace --all-targets --no-fail-fast
   -- --skip rust_scaffold_builds_and_tests_with_native_toolchain` →
  one pre-existing baseline failure
  (`fleet_online_routes_to_local_listener_when_alethefy_is_up`,
  sandbox listener restriction, fails on the stashed baseline
  too); the other two pre-existing flakes were not exercised
  by the changed paths.
- `node scripts/check-openspec-change-names.mjs` →
  `check-openspec-change-names: PASS`.
- `openspec validate --all --strict --no-interactive` →
  `Totals: 61 passed, 0 failed (61 items)`.
- `git diff --check` → no output (clean).

### No provider contact, no state mutation (4.2)

The new parity suite and all existing contract suites run the
built `forge` binary against local SQLite registries and local
file fixtures. No `FORGE_GITHUB_BIN`, `OPENAI_API_KEY`,
`ANTHROPIC_API_KEY`, `HTTP_PROXY`, `HTTPS_PROXY`, or
`ALL_PROXY` environment variables are set in the test command
(`clean_cmd()` strips them defensively). All assertions are
read-side: `list`, `inspect`, `mcp tools/call`, `http GET`,
identity `build-challenge` / `complete-auth` (test-only OIDC
flow that mints a session in a local tempdir). No
`register`, `add-feature`, `apply-deployment`, `share-*`,
`interest-*`, `publish-*`, `release-*`, or `git push` command
is exercised. The registry is opened read-only by the catalog
service and the API handlers; no migration or schema change
is shipped.
