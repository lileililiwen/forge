## 1. BFS — Baseline and impact coverage

- [x] 1.1 Map every retained contract family to its Forge producer/consumer, CLI/API surface, schema, fixtures, manifest entry, and tests; mark inactive families and their refusal paths.
- [x] 1.2 Confirm the implementation handoff: Rust 2021 / MSRV 1.88, JSON Schema 2020-12, pinned Python `jsonschema` development dependency, no production validator dependency, and no sibling edits.
- [x] 1.3 Add baseline tests for retained-family bytes, unsupported-family status, unknown-family distinction, authenticated list/detail behavior, and offline release/CI execution.
- [x] 1.4 Reconcile the modified requirements with current CLI and assurance API response shapes and record compatibility assertions.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Add Forge-owned support catalog and unsupported-family reasons; remove identity-subject, permission-decision, tenant-context, and job-outcome schema entries/assets from the Forge support set.
- [x] 2.2 Refactor Rust family lookup, version coverage, validation, emission refusals, CLI list/inspect, and assurance API to use the same supported/unsupported catalog.
- [x] 2.3 Update assurance workbench rendering to show supported and unsupported families, safe reasons, and available actions with existing auth, keyboard, responsive, and accessibility behavior.
- [x] 2.4 Replace upstream sync/provenance behavior with deterministic offline Forge manifest generation and verification; include all retained schemas, support data, fixtures, and vocabularies.
- [x] 2.5 Package contract assets under `share/forge/contracts`; resolve installed assets beside the executable and prove list/inspect work from an unrelated directory.
- [x] 2.6 Add Draft 2020-12 positive and negative fixtures for every retained family, pin `jsonschema==4.25.1` as a development dependency, and validate portfolio publisher output in the ordinary test suite.
- [x] 2.7 Remove the parity script, pinned sibling checkout job, and sibling-path release-check behavior; wire the offline contract check into local release checks and required CI.
- [x] 2.8 Update README, architecture, release readiness, canonical OpenSpecs, and roadmap language to state Forge's maintained subset and unsupported boundary.
- [x] 2.9 Keep the contract module within Forge's 1,000-line source limit by placing the contract inventory in a focused module; verify the repository source-size check.
- [x] 2.10 Remove the stale sibling-provider declaration for Forge's own contract manifest; make retained-copy accounting use the Forge asset manifest.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Verify each supported family across registry, schema, manifest, typed producer/consumer, CLI, API, and web assurance UI; verify each unsupported family across catalog, CLI refusal, and API/UI state.
- [x] 3.2 Verify malformed, missing, duplicate, unsafe-path, extra-file, tampered-digest, invalid-schema, invalid-fixture, unauthenticated, unknown-family, and unsupported-family boundaries.
- [x] 3.3 Verify legacy CLI/API fields, retained schema bytes, emitted discriminators, wire documents, read-only behavior, and portfolio manifest bytes remain compatible.
- [x] 3.4 Verify the packaged archive includes every contract asset and installed resolution works without repository paths or environment overrides.
- [x] 3.5 Search the repository for active source-resolution, parity-script, workflow-fetch, and release-check references; update only active code/docs/spec requirements, preserving historical archive records.
- [x] 3.6 Run the authenticated browser flow at desktop and 320px widths and the applicable accessibility assertions for supported and unsupported family states.

## 4. Verification

- [x] 4.1 Run focused Rust contract/CLI/API tests, schema fixture tests, portfolio publisher tests, script syntax checks, and all affected browser tests.
- [x] 4.2 Run the complete applicable Rust test suite and release check from a checkout without `../platform-contracts` and with `PLATFORM_CONTRACTS_DIR` unset.
- [x] 4.3 Run OpenSpec name preflight, strict validation, change status/instructions, and `git diff --check`; inspect all new assets and staged changes.
- [x] 4.4 Run `forge gate --dry-run`, then full Forge Gate with a bounded timeout; record the actual verdict in HANDOFF.
- [x] 4.5 Review every impact-map row and requirement scenario, promote canonical specs without `--skip-specs`, archive the completed change, and commit implementation/archive followed by HANDOFF only.

### Verification exception

- `scripts/check-tests.sh` passed the complete applicable workspace test suite.
- `scripts/release-check.sh --gate-profile rust-web` was attempted and stopped
  at repository-wide `cargo fmt --check`. Its 12 reported files are outside
  this change; every Rust file changed here passes a targeted `rustfmt --check`.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` was
  attempted and stopped on existing warnings in `crates/cargo-audit-boost`;
  no changed Forge contract or API source was named by Clippy. Remediation is
  a separate formatting/Clippy cleanup of the imported cargo-* modules.
