## 1. Contracts

- [x] 1.1 Create `Platform.Quota` contracts for subjects, resources, windows, usage snapshots, decisions, reservations, and lifecycle outcomes.
- [x] 1.2 Add options for maximum amounts, clock behavior, and operation-key validation.
- [x] 1.3 Add tests for opaque identifiers, explanatory decisions, invalid transitions, and idempotent operations.

## 2. Stores and concurrency

- [x] 2.1 Implement a thread-safe in-memory quota store using `IClock` and explicit windows.
- [x] 2.2 Add concurrency tests proving accepted reservations cannot oversubscribe a limit.
- [x] 2.3 Add deterministic testing builders and operation-record inspection helpers.

## 3. Billing and application seams

- [x] 3.1 Define an optional entitlement-to-limit resolver interface without importing plan or invoice entities.
- [x] 3.2 Document application-owned unit conversion, reconciliation, cleanup, and persistence.
- [x] 3.3 Port relevant starter/Crossify reservation tests and behavior without copying product models.
- [x] 3.4 Add package-folder guidance using `Contracts`, `Stores`, `Evaluation`, and `DependencyInjection`.

## 4. Verification

- [x] 4.1 Run targeted tests, full solution build/test, package packing, strict OpenSpec validation, and architecture tests.
- [x] 4.2 Run `git diff --check` and document that no billing plans or migrations are included.
