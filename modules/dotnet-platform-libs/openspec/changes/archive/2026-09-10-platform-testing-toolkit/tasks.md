## 1. Inventory and package boundaries

- [x] 1.1 Inventory existing testing-support projects and remove duplicated helper concepts from the proposed scope.
- [x] 1.2 Define core, ASP.NET Core, and provider/Testcontainers package boundaries.
- [x] 1.3 Add architecture rules for all testing-support packages.

## 2. Deterministic core fixtures

- [x] 2.1 Add resettable clock/context fixtures and cleanup helpers.
- [x] 2.2 Add recorded fake eventing, cache, storage, quota, and identity lifecycle components where contracts exist.
- [x] 2.3 Add configurable transient/permanent failure injection with safe diagnostics.

## 3. Optional integration fixtures

- [x] 3.1 Add a minimal TestServer host builder over public web contracts.
- [ ] 3.2 Add optional Testcontainers fixtures without production references.
- [x] 3.3 Document Docker-gated and environment-blocked verification semantics.

## 4. Verification

- [x] 4.1 Add unit tests for reset, isolation, recording, and failure injection.
- [x] 4.2 Migrate one existing platform test project to validate ergonomics.
- [x] 4.3 Run architecture tests, packed consumer conformance, strict OpenSpec validation, and `git diff --check`.