# Tasks

## 1. BFS — Baseline and impact coverage

- [x] Map every requirement and scenario to registry, authorization, persistence, manifest, adapter, audit, and tests.
- [x] Confirm the `platform-contracts` manifest version and the existing Forge operation/idempotency interfaces.
- [x] Confirm the implementation-handoff boundary: Forge owns intent; projects own runtime/auth; Hugo owns rendering.

## 2. DFS — Requirement-by-requirement implementation

- [x] Add migration and domain model for share records, allowlisted surfaces, revision, approval, and publication audit.
- [x] Add validation for canonical HTTPS URLs, allowed surface paths, privacy fields, status evidence, and duplicate identities.
- [x] Add authenticated preview and admin approval endpoints/CLI commands.
- [x] Add deterministic manifest serialization, SHA-256 hash, schema validation, and local export.
- [x] Add generic publisher adapter with idempotency, timeout, safe errors, and reconciliation state.
- [x] Add unit, integration, authorization, golden, malformed-input, and adapter contract tests.

## 3. BFS — Cross-surface regression and completeness

- [x] Verify existing publish operations and project inventory are unchanged.
- [x] Verify no private field, local path, internal port, credential, admin URL, or raw analytics can enter the manifest.
- [x] Verify empty, stale, unknown, retry, duplicate, and partial-publication behavior across HTTP and CLI.
- [x] Verify the audit trail is sufficient to explain what was public, when, by whom, and from which revision.

## 4. Verification

- [x] Run formatter, linter, unit/integration tests, manifest fixture validation, and strict OpenSpec validation.
- [x] Record unavailable external GitHub/Cloudflare evidence as blocked adapter verification, not as a pass.
- [x] Do not mark this planning package implemented until runtime evidence and a reviewed publication artifact exist.
