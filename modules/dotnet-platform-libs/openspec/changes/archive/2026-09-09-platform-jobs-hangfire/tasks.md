## 1. Package and contracts

- [x] 1.1 Create `Platform.Jobs.Hangfire` and centralize Hangfire versions without modifying `Platform.Jobs` dependencies.
- [x] 1.2 Define application-owned execution-context, dashboard-authorization, storage, and health seams.
- [x] 1.3 Adapt the starter `Extensions.cs`, `FshJobFilter.cs`, `FshJobActivator.cs`, and health check patterns without copying product identity or tenant types.

## 2. Adapter implementation

- [x] 2.1 Implement dispatch and recurring registration using the platform descriptors and payloads.
- [x] 2.2 Implement scoped context restoration and disposal around Hangfire activations.
- [x] 2.3 Implement opt-in dashboard mapping, authorization, provider health, and safe telemetry.
- [x] 2.4 Implement idempotent DI registration and explicit retry-policy documentation.

## 3. Verification and documentation

- [x] 3.1 Add unit tests for payload mapping, recurring descriptors, context restoration, authorization, and failure redaction.
- [x] 3.2 Add optional PostgreSQL/Hangfire integration tests with Docker gating.
- [x] 3.3 Add starter-kit migration guidance and rollback instructions.
- [ ] 3.4 Run serial tests, architecture checks, `git diff --check`, and strict OpenSpec validation.
