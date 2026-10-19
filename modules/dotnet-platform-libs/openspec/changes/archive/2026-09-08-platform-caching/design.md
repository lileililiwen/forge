## Context

The starter’s observable HybridCache and sibling Redis/in-memory caches demonstrate recurring needs: bounded keys, expiration, tag invalidation, safe degradation on transient provider errors, and cache operation telemetry. The platform currently has no cache package.

## Goals / Non-Goals

**Goals:**

- Provide a small async cache abstraction suitable for application-owned serializers and models.
- Make key prefixes, tenant isolation, expiration, and invalidation explicit.
- Provide a fast local implementation and optional Microsoft/Redis adapters.

**Non-Goals:**

- No cache-as-database semantics, distributed locks, session storage, or business-level cache policies.
- No mandatory Redis, HybridCache, ASP.NET Core, or JSON serialization dependency.

## Decisions

- **Use a narrow contract.** `ICacheStore` exposes get-or-create, set, remove, and tag invalidation; values remain generic and serialization remains adapter-owned.
- **Separate key construction.** `CacheKey`/`CacheKeyBuilder` validates length and whitespace and makes tenant/application prefixes explicit.
- **Treat cache failures as policy decisions.** Adapters report safe status and may degrade reads to misses, but the contract exposes enough result information for applications to choose fail-open or fail-closed behavior.
- **Copy starter observability patterns, not implementation coupling.** Use activity and metric names inspired by `ObservableHybridCache`, without depending on FSH telemetry types.
- **Package layout.** Keep `Platform.Caching` as a package root with `Contracts`, `Keys`, `Telemetry`, and `DependencyInjection`; put provider adapters in separate top-level package roots.

## Risks / Trade-offs

- [Risk] Generic cache serialization can hide schema incompatibility → [Mitigation] make serializer/provider adapters explicit and document versioned keys.
- [Risk] Fail-open behavior can serve stale or missing data → [Mitigation] keep cache non-authoritative and expose provider status for readiness when required.
- [Risk] Redis-specific features leak into contracts → [Mitigation] keep tags and expiration at the portable minimum.

## Migration Plan

Applications can wrap existing caches behind `ICacheStore`, migrate key construction first, and adopt an adapter later. Removing the package returns callers to their existing cache implementation without data migrations.

## Open Questions

- Whether the first distributed adapter should use `IDistributedCache` only or include native Redis tag operations.
