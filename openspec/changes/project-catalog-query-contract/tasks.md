# Tasks: project-catalog-query-contract

Status: planning-only package. Every task is unchecked by design; nothing is
implemented or archived by this package.

## 1. BFS — Baseline and impact coverage

- [ ] 1.1 Read `src/publish/inventory.rs`, `src/fleet/`, `src/registry/mod.rs` and the `forge list` command; record the existing source readers, normalized shapes and the exact reuse boundary.
- [ ] 1.2 Confirm the implementation-handoff gate: Rust 1.87+, module paths `src/catalog/*`, CLI `forge project …`, no new dependency, no schema change.
- [ ] 1.3 Map each requirement and scenario to its source adapters, query predicates, callers, failure cases and named tests.

## 2. DFS — Requirement-by-requirement implementation

- [ ] 2.1 Implement `CatalogRecord`, `SourceKind`, `EvidenceState` and provenance with a closed field set and refusal of unknown fields.
- [ ] 2.2 Implement source adapters over local registry, Git, workspace registry and explicit inventory; unreadable sources produce `unavailable` provenance.
- [ ] 2.3 Implement the pure query model: AND filters, OR within a predicate, stable ordering by `(project_id, source)`, and deterministic pagination.
- [ ] 2.4 Add `forge project list|inspect|tags|languages` with `--format table|json|ndjson` and repeatable filters; redact credentials on every path.

## 3. BFS — Cross-surface regression and completeness

- [ ] 3.1 Prove determinism across repeated reads and source permutations.
- [ ] 3.2 Prove the query is read-only: no registry byte, table or journal row changes.
- [ ] 3.3 Exercise empty, stale, unavailable, duplicate-id, redaction and unknown-filter boundaries.

## 4. Verification

- [ ] 4.1 Run `cargo fmt/build/clippy/test`, the two new suites, `openspec validate --all --strict --no-interactive`, the name preflight and `git diff --check`; record the exact output.
- [ ] 4.2 Record that no provider was contacted and no project was mutated; GitHub observation remains a separate package.
