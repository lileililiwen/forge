# Tasks: project-query-consumer-surfaces

Status: planning-only package. Every task is unchecked by design.

## 1. BFS — Baseline and impact coverage

- [ ] 1.1 Read `src/api/mod.rs` route/authorization, `src/mcp/mod.rs` tool registration and the `forge project` CLI; record the shared Core entry point and the existing auth/error mappings to reuse.
- [ ] 1.2 Confirm the implementation-handoff gate: Rust 1.87+, one Core query service, thin transports, no redefined catalog fields, no new dependency.
- [ ] 1.3 Map each requirement and scenario to its transport, parameter, authorization arm, failure case and named test.

## 2. DFS — Requirement-by-requirement implementation

- [ ] 2.1 Add stable JSON/NDJSON output and shared filters to `forge project list|inspect|tags|languages`.
- [ ] 2.2 Register MCP catalog tools over the Core service with typed error mapping.
- [ ] 2.3 Add `GET /v1/projects/catalog` with shared query parameters and the `authorize()` boundary.
- [ ] 2.4 Ensure every transport serializes the Core `CatalogPage` unchanged.

## 3. BFS — Cross-surface regression and completeness

- [ ] 3.1 Prove CLI/MCP/API parity for records, ordering, pagination and filters.
- [ ] 3.2 Exercise invalid-filter, unauthorized, project-mismatch and empty-result boundaries on each transport.
- [ ] 3.3 Prove no second filtering implementation exists and the table layout is not load-bearing.

## 4. Verification

- [ ] 4.1 Run `cargo fmt/build/clippy/test`, the new parity suite, `openspec validate --all --strict --no-interactive`, the name preflight and `git diff --check`; record the exact output.
- [ ] 4.2 Record that no provider was contacted and no query mutated state; read-only parity is the whole claim.
