## 1. Package setup

- [x] 1.1 Create `Platform.Web.Versioning` and add centrally managed Asp.Versioning dependencies.
- [x] 1.2 Add the package to the solution and architecture package inventory.
- [x] 1.3 Define options and XML-documented registration extensions.

## 2. Implementation

- [x] 2.1 Implement configurable API-versioning registration with opt-in defaults.
- [x] 2.2 Implement API Explorer grouping and OpenAPI integration guidance.
- [x] 2.3 Ensure repeated registration and consumer overrides are deterministic.

## 3. Tests and documentation

- [x] 3.1 Add unit and TestServer tests for opt-in behavior, defaults, readers, and grouping.
- [x] 3.2 Add architecture tests for dependency direction and forbidden application references.
- [x] 3.3 Document migration from the starter versioning extension and rollback behavior.

## 4. Verification

- [x] 4.1 Run package tests and the full platform suite.
- [x] 4.2 Run packed consumer conformance and strict OpenSpec validation.
- [x] 4.3 Run `git diff --check`.