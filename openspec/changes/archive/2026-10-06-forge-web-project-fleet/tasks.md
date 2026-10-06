# Tasks: forge-web-project-fleet

## 1. BFS — Baseline and impact coverage

- [x] 1.1 Map current admin project DTO, registry fleet loader, portable inventory/fleet readers, frontend list states and source configuration.
- [x] 1.2 Add fixture matrix for local, external, Forge-self, overlap, stale, malformed and unavailable sources; record baseline focused checks.
- [x] 1.3 Verify normalized DTO, explicit source selection, path redaction and conflict semantics match this design/spec.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Implement bounded adapters for the explicitly configured local inventory and fleet registry formats.
- [x] 2.2 Implement stable identity normalization, Forge-self row, provenance, freshness, partial-source and conflict states.
- [x] 2.3 Expose the versioned authenticated JSON fleet contract with compatibility handling.
- [x] 2.4 Update standalone browser fleet list, filters, source/status badges and self/project navigation.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Verify existing registered-project fields and project details remain compatible with current frontend callers.
- [x] 3.2 Verify no filesystem scanning, path disclosure, unauthorized reads or external writes; test empty and partial source combinations.
- [x] 3.3 Verify Forge appears exactly once when registered and unregistered and ambiguous records cannot initiate mutations.

## 4. Verification

- [x] 4.1 Run `cargo fmt --check`, `cargo check --all-targets`, focused fleet/API tests and OpenSpec strict validation.
- [x] 4.2 Run API plus standalone Rust web server and browser smoke for full, empty, stale and unavailable source states; record exact evidence.
