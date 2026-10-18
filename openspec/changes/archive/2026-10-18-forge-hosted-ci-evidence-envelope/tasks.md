## 1. BFS — Baseline and impact coverage

- [x] 1.1 Confirm current Gate evidence serialization, project-id validation, timestamp and error conventions; map every embedded field and caller impact.
- [x] 1.2 Add envelope contract tests before implementation and observe expected missing-module failure, then add invalid-case coverage.
- [x] 1.3 Confirm no persistence, transport, generated workflow, or UI is in scope and document ingest dependency.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Implement typed `forge-hosted-ci-evidence/1.0` model with strict envelope/run unknown-field handling and bounded values.
- [x] 2.2 Implement cross-field validation for project, GitHub run metadata, URL, timestamp, Gate contract, revision and executed terminal verdict.
- [x] 2.3 Export the contract from Forge Core and provide deterministic serialize/parse validation helpers.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Exercise supported and refused cases including rehearsal, unsupported shapes, untrusted URL variants and contradictory identity/revision.
- [x] 3.2 Review disclosure, bounds, error text and confirm no arbitrary secrets/logs are accepted or emitted.
- [x] 3.3 Update CI/CD system model to mark envelope definition complete and hosted authenticated upload/ingest as next.

## 4. Verification

- [x] 4.1 Run focused contract tests and changed-file Rust formatting checks.
- [x] 4.2 Run name preflight, strict OpenSpec validation, diff check, and review new files.
- [x] 4.3 Run Gate dry-run and bounded full Gate; record the blocked verdict and confirmed unrelated failure/remediation in HANDOFF.
- [x] 4.4 Archive with canonical spec promotion; commit scoped implementation/archive, update HANDOFF, commit HANDOFF only, and stop.
