# platform-ratelimiting Specification

## Purpose

The platform already ships `Platform.Core`, `Platform.AspNetCore`, and `Platform.Billing.Contracts` for the cross-cutting concerns that recur across every application in the portfolio. Rate limiting is the same kind of concern — every consumer in the portfolio that exposes HTTP endpoints needs a documented `IRateLimiter`, named policies, an in-memory default backend, and a documented bypass contract — but the platform does not yet ship it. VisualFlow has already implemented exactly this in `src/BuildingBlocks/RateLimiting/`, and the type surface (`IRateLimiter`, `InMemoryRateLimiter`, `RateLimitPolicies`, `RateLimitDecision`, `RateLimitKey`, `IRateLimitBypassResolver`, `IRateLimiterBackendStatusProvider`, `HttpContextAbstraction`, `RateLimitingOptions`) is provider-neutral, framework-neutral, and free of EF Core. Promoting it into the platform proves the Phase 3 adoption hypothesis: a real consumer can adopt a shared contract without rewriting its host.
## Requirements
### Requirement: Rate limiting SHALL expose a documented contract

The package SHALL expose an `IRateLimiter` contract, the documented `RateLimitDecision` and `RateLimitKey` value types, and a `RateLimitingOptions` configuration type that a consumer can bind to configuration.

#### Scenario: Consumer binds to configuration

- GIVEN a consumer binds the `RateLimiting` configuration section
- WHEN the host starts
- THEN `IRateLimiter` resolves to the in-memory default backend
- AND the `RateLimitPolicies` default catalog is registered as a singleton

#### Scenario: Consumer supplies a custom policy catalog

- GIVEN a consumer registers a custom `RateLimitPolicies` registration before `AddPlatformRateLimiting`
- WHEN the host starts
- THEN the custom catalog takes precedence over the documented default

### Requirement: In-memory backend SHALL enforce a documented windowed counter

The in-memory backend SHALL track a per-key windowed counter, SHALL return a `RateLimitDecision` with the documented fields, and SHALL honour the configured `Limit` and `WindowSeconds`.

#### Scenario: First request within the window

- GIVEN a fresh bucket and a policy of `Limit = 5, WindowSeconds = 60`
- WHEN the consumer calls `Check` with the documented key
- THEN the decision is `Allowed = true`, `Remaining = 4`, and `RetryAfterSeconds = 0`

#### Scenario: Burst over the limit

- GIVEN a bucket that has already accumulated 5 requests in the documented window
- WHEN the consumer calls `Check` with the same key
- THEN the decision is `Allowed = false`, `Remaining = 0`, and `RetryAfterSeconds` is the documented ceil of the time remaining in the window

#### Scenario: Window rolls over

- GIVEN a bucket that was full at `t0`
- WHEN the consumer calls `Check` at `t0 + WindowSeconds`
- THEN the decision is `Allowed = true` with a fresh counter

### Requirement: Backend SHALL be framework-neutral

The package SHALL depend only on `Platform.Core`, `Microsoft.Extensions.Options`, and `Microsoft.Extensions.DependencyInjection.Abstractions`. It SHALL NOT reference ASP.NET Core, EF Core, StackExchange.Redis, or a VisualFlow project.

#### Scenario: Architecture test

- GIVEN the architecture test runs
- WHEN it inspects the package's references
- THEN the test fails if any forbidden reference is present
- AND the test passes with the documented reference list

### Requirement: Bypass resolver SHALL be opt-in

The package SHALL expose an `IRateLimitBypassResolver` contract and SHALL ship a default implementation that reads documented bypass tokens from configuration.

#### Scenario: Documented bypass token

- GIVEN a request carries a documented service token
- WHEN the consumer evaluates the bypass resolver
- THEN the resolver returns `Allowed = true` with the documented label
- AND the call site can short-circuit before invoking the limiter

#### Scenario: No bypass token

- GIVEN a request does not carry a documented service token
- WHEN the consumer evaluates the bypass resolver
- THEN the resolver returns `Allowed = false`
- AND the call site proceeds to the limiter

### Requirement: Backend status SHALL be observable

The package SHALL expose an `IRateLimiterBackendStatusProvider` that returns the configured provider, the documented availability flag, and a documented detail string so a consumer can register a readiness check.

#### Scenario: Readiness surfaces backend

- GIVEN a consumer registers a readiness check that calls `IRateLimiterBackendStatusProvider.GetStatus`
- WHEN the readiness pipeline runs
- THEN the check reports the configured provider, `Available = true`, and the documented detail
- AND a provider-specific implementation can later override the status without changing the consumer

### Requirement: Clock access SHALL come from Platform.Core

The in-memory backend SHALL read the current time from `IClock` and SHALL NOT call `DateTimeOffset.UtcNow` directly.

#### Scenario: Deterministic test

- GIVEN a test fixes the platform clock at a known value
- WHEN the consumer calls `Check`
- THEN the bucket is computed from the fixed clock value
- AND the test does not depend on wall-clock time

