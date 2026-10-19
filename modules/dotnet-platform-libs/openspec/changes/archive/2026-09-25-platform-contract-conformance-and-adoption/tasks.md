# Tasks: .NET contract conformance and adoption

## 1. BFS — Baseline and impact coverage

- [x] Inventory current platform package boundaries, architecture tests,
  public API baseline, release checks, and adoption diagnostics.
- [x] Map existing identity/admin/audit/release contracts to the shared
  language-neutral schemas and identify incompatible semantics.
- [x] Add fixture and consumer-conformance skeletons before implementation.

## 2. DFS — Requirement-by-requirement implementation

- [x] Add shared-contract fixture validation without runtime repository
  coupling.
- [x] Add adoption diagnostics for package, registration, test-boundary, and
  application-owned adapter status.
- [x] Add architecture and public-API coverage for new conformance surfaces.
- [x] Document staged adoption and rollback for representative .NET products.

## 3. BFS — Cross-surface regression and completeness

- [x] Re-run package dependency-direction, test-only reference, API-baseline,
  sample-matrix, and packed-consumer checks.
- [x] Verify identity/admin packages remain opt-in and do not own application
  users, roles, migrations, or provider credentials.
- [x] Verify release evidence remains distinct from build/test success.

## 4. Verification

- [x] `./scripts/quality-gate.sh`.
- [x] `./scripts/conformance.sh`.
- [x] `./scripts/check-public-api.sh`.
- [x] `./scripts/audit-packages.sh`.
- [x] `openspec validate --changes --strict --no-interactive`.
