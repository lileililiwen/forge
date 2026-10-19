## 1. Contract and state model

- [x] 1.1 Define lifecycle operation, step descriptor, status snapshot, failure classification, and retry semantics.
- [x] 1.2 Define application-owned catalog, state-store, migration, seed, and tenant-scope callback interfaces.
- [x] 1.3 Define stable idempotency keys and operator retry semantics.

## 2. Implementation

- [x] 2.1 Add framework-neutral tenant lifecycle contracts and in-memory state implementation.
- [x] 2.2 Add orchestration with ordered execution, resume, cancellation, safe failure mapping, and scope restoration.
- [x] 2.3 Add provider-neutral readiness/status adapter (HTTP endpoints) and integration with the persistence multitenancy scope.

## 3. Tests and documentation

- [x] 3.1 Test ordering, resume, duplicate retry, failure classification, and cancellation.
- [x] 3.2 Test tenant-scope isolation and no-scope cleanup.
- [x] 3.3 Add architecture tests proving no tenant entity, migration, provider, or application reference is introduced.
- [x] 3.4 Document ownership and migration from the starter provisioning workflow.

## 4. Verification

- [x] 4.1 Run package, integration, and consumer conformance tests.
- [x] 4.2 Run strict OpenSpec validation, API compatibility, and `git diff --check`.