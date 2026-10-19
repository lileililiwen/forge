# Proposal: Adopt VisualFlow's rate-limiting building block into the platform

## Why

The platform already ships `Platform.Core`, `Platform.AspNetCore`, and `Platform.Billing.Contracts` for the cross-cutting concerns that recur across every application in the portfolio. Rate limiting is the same kind of concern — every consumer in the portfolio that exposes HTTP endpoints needs a documented `IRateLimiter`, named policies, an in-memory default backend, and a documented bypass contract — but the platform does not yet ship it. VisualFlow has already implemented exactly this in `src/BuildingBlocks/RateLimiting/`, and the type surface (`IRateLimiter`, `InMemoryRateLimiter`, `RateLimitPolicies`, `RateLimitDecision`, `RateLimitKey`, `IRateLimitBypassResolver`, `IRateLimiterBackendStatusProvider`, `HttpContextAbstraction`, `RateLimitingOptions`) is provider-neutral, framework-neutral, and free of EF Core. Promoting it into the platform proves the Phase 3 adoption hypothesis: a real consumer can adopt a shared contract without rewriting its host.

## What Changes

- Add a new production package `Platform.RateLimiting` that owns the rate-limiting contracts and the in-memory default backend.
- The package depends on `Platform.Core` for `IClock` (already shipped) and the documented `Error` shape; it does not depend on ASP.NET Core, EF Core, Redis, or a specific provider.
- Re-export the VisualFlow type surface under the `Platform.RateLimiting` namespace, with the documented breaking-rename rules (the existing `IRateLimiterBackendStatus` record and `IRateLimiter` interface move verbatim; `RateLimitPolicies` becomes the documented default policy catalog).
- VisualFlow's `src/BuildingBlocks/RateLimiting/` is removed in a follow-up VisualFlow change that adopts the new package; this change is the platform side only.

## Capabilities

### New Capabilities

- `platform-ratelimiting`: Documented rate-limit contract surface, in-memory default backend, named policy catalog, and bypass-resolver contract.

### Modified Capabilities

- (none)

## Impact

- Adds one new production NuGet package, `Platform.RateLimiting`, versioned in `Directory.Packages.props`.
- Expands the architecture test to forbid `Platform.RateLimiting` from referencing ASP.NET Core, EF Core, Redis, or any rate-limit provider SDK.
- VisualFlow adopts the package in a separate change after this lands; no VisualFlow code is modified in this change.

## Context

Phase 3 of the platform roadmap is the adoption proof — a real consumer proves that the shared contracts reduce duplication without forcing a host rewrite. The VisualFlow rate-limiting block is the cleanest candidate: it is small, framework-neutral, has zero provider dependencies, and the only `Microsoft.Extensions.*` reference is `IOptions<>` and `IServiceCollection`. Promoting it demonstrates that the platform can adopt consumer code without re-platforming the consumer.

## Goals / Non-Goals

**Goals:**

- Ship a self-contained, framework-neutral rate-limit package.
- Preserve the documented decision shape so consumers can swap in a Redis-backed backend without changing the call sites.
- Make the policy catalog (`RateLimitPolicies`) a default that consumers can override, not a closed list.

**Non-Goals:**

- Ship a Redis or distributed backend in this change; a follow-up `platform-ratelimiting-redis` change can adopt one if a second consumer needs it.
- Ship an ASP.NET Core middleware; that is the responsibility of the consumer's `Platform.AspNetCore` integration.
- Replace any rate-limit implementation in any consumer.

## Decisions

- Reuse the existing `Platform.Core.IClock` rather than re-introducing a `DateTimeOffset.UtcNow` dependency.
- Use the `Platform.Core.Error` shape for bypass and backend-status error surfaces.
- Keep the `IRateLimiterBackendStatusProvider` as part of the public contract; this lets consumers surface a readiness check from a provider-specific implementation later.
- The new package is added to `Directory.Build.props` packaging metadata with a stable 0.1.0 version.

## Risks / Trade-offs

- The VisualFlow block uses `Microsoft.Extensions.Options` and `Microsoft.Extensions.DependencyInjection`. Both are available as transitive dependencies through the existing platform packages, but the architecture test must be expanded to ensure the new package does not pull in ASP.NET Core by accident.
- `HttpContextAbstraction` is a thin ASP.NET-Core-free wrapper around the bypass resolver; the contract is preserved verbatim.
