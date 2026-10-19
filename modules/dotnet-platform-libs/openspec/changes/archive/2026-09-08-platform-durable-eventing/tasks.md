## 1. Package and contract setup

- [x] 1.1 Create `Platform.Eventing.Contracts` with outbox/inbox records, state enums, claim results, store interfaces, and options.
- [x] 1.2 Add project metadata, package references, XML documentation, and architecture rules for the framework-neutral contract package.
- [x] 1.3 Add contract tests for validation, metadata preservation, state transitions, retry bounds, and duplicate decisions.

## 2. EF Core adapter

- [x] 2.1 Create `Platform.Eventing.EfCore` using the repository’s package-folder convention: `Persistence`, `Dispatch`, `Telemetry`, and `DependencyInjection`.
- [x] 2.2 Implement application-configurable EF model mappings without owning an application DbContext or migrations.
- [x] 2.3 Implement atomic outbox/inbox claim, lease expiry, completion, retry, and dead-letter operations.
- [x] 2.4 Add SQLite and concurrent independent-context tests for claim and duplicate behavior.

## 3. Dispatch and adoption

- [x] 3.1 Implement a hosted dispatcher seam that uses `IClock`, cancellation, bounded batches, and safe logging.
- [x] 3.2 Port relevant starter dispatcher behavior and tests after removing FSH-specific dependencies.
- [x] 3.3 Document transport adapters, handler idempotency, migration ownership, and rollback.
- [x] 3.4 Add starter composition and sample adoption guidance without enabling durable persistence by default.

## 4. Verification

- [x] 4.1 Run targeted tests, full solution build/test, and package packing.
- [x] 4.2 Run strict OpenSpec validation and architecture tests.
- [x] 4.3 Run `git diff --check` and record evidence in `HANDOFF.md` only when implementation is complete.
