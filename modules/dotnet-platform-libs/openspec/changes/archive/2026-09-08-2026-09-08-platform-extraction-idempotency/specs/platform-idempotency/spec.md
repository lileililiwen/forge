# platform-idempotency Specification

## Purpose

Provide a framework-neutral idempotency contract and an in-memory default store that any application in the portfolio can adopt without bringing in ASP.NET Core, EF Core, or a specific provider. The package is the second Phase 3 adoption proof: a real consumer (VisualFlow) contributes the type surface, the platform owns the package, and the consumer migrates to it in a follow-up change.

## ADDED Requirements

### Requirement: Idempotency SHALL expose a documented record and fingerprint

The package SHALL expose an `IdempotencyRecord` value type and a `RequestFingerprint` helper, both of which are framework-neutral and free of provider dependencies.

#### Scenario: Consumer computes a fingerprint

- GIVEN a consumer supplies a method, route, and body hash
- WHEN the consumer calls `RequestFingerprint.Compute`
- THEN the helper returns a stable, documented fingerprint string
- AND the same inputs return the same fingerprint across processes

#### Scenario: Consumer stores a record

- GIVEN a consumer has a documented idempotency key and a documented response shape
- WHEN the consumer constructs an `IdempotencyRecord` and calls `SaveAsync`
- THEN the record is stored under the documented key
- AND a subsequent `TryGetAsync` returns the same record

### Requirement: Store SHALL be framework-neutral

The package SHALL depend only on `Platform.Core`, `Microsoft.Extensions.Options`, and `Microsoft.Extensions.DependencyInjection.Abstractions`. It SHALL NOT reference ASP.NET Core, EF Core, StackExchange.Redis, or a VisualFlow project.

#### Scenario: Architecture test

- GIVEN the architecture test runs
- WHEN it inspects the package's references
- THEN the test fails if any forbidden reference is present
- AND the test passes with the documented reference list

### Requirement: In-memory store SHALL honour documented retention and key-length rules

The in-memory store SHALL honour the documented `RetentionSeconds` and `MaxKeyLength` from configuration, SHALL evict expired records on a documented sweep, and SHALL reject keys longer than the documented maximum.

#### Scenario: Retention window

- GIVEN a record is stored at `t0` with `RetentionSeconds = 86400`
- WHEN the consumer calls `EvictExpiredAsync` at `t0 + 86401`
- THEN the record is removed
- AND a subsequent `TryGetAsync` returns `null`

#### Scenario: Oversized key

- GIVEN a consumer supplies a key longer than `MaxKeyLength`
- WHEN the consumer calls `SaveAsync`
- THEN the store throws the documented `ArgumentException`
- AND no record is stored

### Requirement: Store SHALL consume IClock

The in-memory store SHALL read the current time from `IClock` and SHALL NOT call `DateTimeOffset.UtcNow` directly.

#### Scenario: Deterministic test

- GIVEN a test fixes the platform clock at a known value
- WHEN the consumer calls `EvictExpiredAsync`
- THEN the sweep is computed from the fixed clock value
- AND the test does not depend on wall-clock time

### Requirement: Registration SHALL be opt-in and configurable

The package SHALL expose an `AddPlatformIdempotency` extension method and SHALL bind `IdempotencyOptions` to the documented `Idempotency` configuration section.

#### Scenario: Default registration

- GIVEN a consumer calls `AddPlatformIdempotency`
- WHEN the host starts
- THEN `IIdempotencyStore` resolves to `InMemoryIdempotencyStore`
- AND `IdempotencyOptions` is bound to the `Idempotency` configuration section

#### Scenario: Disabled registration

- GIVEN a consumer sets `Idempotency:Enabled = false`
- WHEN the host starts
- THEN the registration no-ops and the in-memory store is not registered
- AND the consumer can opt out without changing call sites

### Requirement: Metrics surface SHALL be preserved

The package SHALL preserve the documented `IdempotencyMetrics` surface so consumers can keep their existing dashboards.

#### Scenario: Existing dashboard

- GIVEN a consumer has a dashboard bound to the documented metric names
- WHEN the consumer upgrades to the platform package
- THEN the dashboard continues to receive the documented metrics
- AND no metric name changes are introduced
