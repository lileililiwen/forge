## 1. Runner boundary

- [x] 1.1 Create the optional migrator package and define context-factory, options, result, seed, and exclusive-execution contracts.
- [x] 1.2 Implement pending and apply operations with cancellation and safe failure classification.
- [x] 1.3 Add a thin console-host integration example that owns argument parsing and exit codes.

## 2. Verification and documentation

- [x] 2.1 Add unit tests for pending/apply delegation, cancellation, seed/lock callbacks, and redaction.
- [x] 2.2 Add an integration test using an application-owned SQLite or PostgreSQL context where available.
- [x] 2.3 Add architecture tests and document migration, rollback, and deployment-lock ownership.
- [x] 2.4 Run focused tests, build, strict OpenSpec validation, and `git diff --check`.
