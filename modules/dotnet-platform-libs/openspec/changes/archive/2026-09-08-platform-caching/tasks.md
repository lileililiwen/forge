## 1. Contracts and key conventions

- [x] 1.1 Create `Platform.Caching` with cache result types, `ICacheStore`, expiration/tag options, provider status, and validated key builders.
- [x] 1.2 Add cache telemetry contracts with redaction-safe metric and activity names.
- [x] 1.3 Add unit tests for key validation, tenant isolation, expiration options, and telemetry redaction.

## 2. Implementations

- [x] 2.1 Add thread-safe in-memory implementation using `IClock` for deterministic expiry tests.
- [x] 2.2 Add optional HybridCache adapter using starter behavior as a reference and keeping Microsoft caching dependencies isolated.
- [x] 2.3 Add optional Redis adapter with bounded operation timeouts, safe transient failures, and provider health.
- [x] 2.4 Add adapter tests for hits, misses, invalidation, expiry, and unavailable backends.

## 3. Adoption and organization

- [x] 3.1 Use `Contracts`, `Keys`, `Telemetry`, and `DependencyInjection` folders inside the base package and separate provider packages at `src/` root.
- [x] 3.2 Document cache authority, serialization/versioning, tenant prefixes, fail-open policy, and migration from sibling caches.
- [x] 3.3 Add starter composition examples without making distributed caching mandatory.

## 4. Verification

- [x] 4.1 Run targeted tests, full build/test, package packing, strict OpenSpec validation, and architecture tests.
- [x] 4.2 Run `git diff --check` and document any environment-only provider test limitations.
