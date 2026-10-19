## 1. Package setup

- [x] 1.1 Create independently adoptable feature-flag and HTTP-resilience adapter packages with central dependency versions.
- [x] 1.2 Define evaluator/context, endpoint metadata, policy options, request classification, and safe telemetry seams.
- [x] 1.3 Adapt the starter feature flag and resilience files/tests listed in `design.md` without copying product flags or tenant entities.

## 2. Adapter implementation

- [x] 2.1 Implement opt-in feature evaluation and endpoint/handler gating with configurable disabled responses.
- [x] 2.2 Implement standard outbound HTTP resilience with bounded defaults, method classification, and named-client overrides.
- [x] 2.3 Implement cancellation/error classification and redacted policy telemetry.
- [x] 2.4 Add replacement-friendly DI registration and configuration validation.

## 3. Verification and documentation

- [x] 3.1 Add tests for enabled/disabled flags, evaluator context, tenant decisions, retry safety, timeout, circuit breaking, and cancellation.
- [x] 3.2 Add architecture tests keeping framework-neutral packages free of feature-management/Polly dependencies.
- [x] 3.3 Document migration from starter feature gates/resilience and retry ownership.
- [x] 3.4 Run serial tests, `git diff --check`, and strict OpenSpec validation.
