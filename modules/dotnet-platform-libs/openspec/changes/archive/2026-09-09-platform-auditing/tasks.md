## 1. Contracts

- [x] 1.1 Create `Platform.Auditing.Contracts` with normalized events, scopes, sink, masking, enrichment, and failure-policy contracts.
- [x] 1.2 Define bounded metadata and safe default masking rules without importing starter module DTOs.
- [x] 1.3 Use the starter auditing contracts/core/persistence files as reference and convert them to application-owned sink and schema seams.

## 2. Capture adapters

- [x] 2.1 Implement ASP.NET Core request, exception, security, correlation, and tenant/subject capture.
- [x] 2.2 Implement EF Core opt-in change capture and pre-serialization masking.
- [x] 2.3 Implement bounded asynchronous publishing, provider status, and configurable fail-open/fail-closed behavior.
- [x] 2.4 Add optional retention/dead-letter interfaces without implementing a platform-owned audit store.

## 3. Verification and documentation

- [x] 3.1 Add unit tests for event validation, masking, enrichment, HTTP capture, exception classification, and EF diffs.
- [x] 3.2 Add architecture tests proving contracts have no EF/ASP.NET dependency and adapters do not own product modules.
- [x] 3.3 Document migration from the starter auditing module and explicit storage/retention ownership.
- [x] 3.4 Run serial tests, `git diff --check`, and strict OpenSpec validation.
