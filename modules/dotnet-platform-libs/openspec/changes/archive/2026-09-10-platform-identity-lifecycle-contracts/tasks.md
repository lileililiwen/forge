## 1. Contract design

- [x] 1.1 Inventory existing identity contracts and starter workflows without copying application entities.
- [x] 1.2 Define opaque session/refresh identifiers, lifecycle commands, outcomes, and stable safe error codes.
- [x] 1.3 Define atomic refresh rotation, policy evaluator, and audit-hook interfaces.

## 2. Package implementation

- [x] 2.1 Add framework-neutral lifecycle contracts under `Platform.Identity.Contracts`.
- [x] 2.2 Add optional ASP.NET Core integration seams without replacing consumer authentication schemes.
- [x] 2.3 Add deterministic in-memory/test implementations with explicit non-production status.

## 3. Verification and documentation

- [x] 3.1 Add concurrency tests for refresh rotation and revocation.
- [x] 3.2 Add safe-failure, policy-denial, impersonation, and user-enumeration tests.
- [x] 3.3 Add architecture tests proving no application entities, EF migrations, or test packages enter production dependencies.
- [x] 3.4 Document ownership, adapter migration, rollback, and security requirements.

## 4. Release verification

- [x] 4.1 Run full Release tests and packed consumer conformance.
- [x] 4.2 Run strict OpenSpec validation and API compatibility review.
- [x] 4.3 Run `git diff --check` and record security audit evidence.