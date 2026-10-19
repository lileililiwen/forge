## 1. Contracts and security

- [x] 1.1 Create `Platform.Webhooks.Contracts` with inbound verification, replay decisions, outbound subscriptions, delivery attempts, retry options, and safe failures.
- [x] 1.2 Add raw-byte signature verifier and secret-resolver interfaces with timestamp/replay metadata.
- [x] 1.3 Add unit tests for invalid signatures, duplicate identifiers, redaction, retry classification, and option validation.

## 2. Web and persistence adapters

- [x] 2.1 Add `Platform.Webhooks.AspNetCore` request capture and response helpers without embedding provider-specific routes.
- [x] 2.2 Add `Platform.Webhooks.EfCore` persistence contracts/configuration and claim/delivery state without owning migrations.
- [x] 2.3 Implement SSRF-safe target validation with DNS resolution, redirect policy, and deterministic tests.
- [x] 2.4 Add HTTP delivery behavior with bounded timeouts, cancellation, retries, and safe status projection.

## 3. Adoption and organization

- [x] 3.1 Port relevant Catchen/VisualFlow webhook behavior and tests after removing product entities and provider assumptions.
- [x] 3.2 Document the boundary with billing adapters and durable eventing, including secret and payload ownership.
- [x] 3.3 Organize packages using `Inbound`, `Outbound`, `Security`, `Persistence`, and `DependencyInjection` folders.

## 4. Verification

- [x] 4.1 Run security, adapter, full solution, packing, architecture, and strict OpenSpec validation tests.
- [x] 4.2 Run `git diff --check`; record external DNS/provider tests separately from deterministic tests.
