# Platform caching

`Platform.Caching` is a non-authoritative, asynchronous cache boundary. Applications own the
cached model, serializer, versioned key, authority, and stale-data policy. A cache miss must
always have a correct application-owned fallback.

## Packages and layout

- `Platform.Caching` — contracts, `Keys`, `Telemetry`, `DependencyInjection`, and the deterministic
  thread-safe in-memory implementation.
- `Platform.Caching.Hybrid` — optional Microsoft `HybridCache` adapter for local or single-host use.
- `Platform.Caching.Redis` — optional StackExchange.Redis adapter with application-provided
  serialization, bounded operation waits, and safe provider status.

The base package has no Redis, HybridCache, ASP.NET Core, or JSON serialization dependency.

## Keys, tenants, tags, and versions

Build physical keys with `CacheKeyBuilder`; tenant-scoped keys must include the tenant identifier.
Include a schema or model version in the logical key whenever serialized data changes, for example
`profile:v2`. Do not put secrets, access tokens, or raw payloads in keys or telemetry. Tags are
portable invalidation hints, not authorization boundaries; tenant prefixes remain mandatory.

## Authority and failure policy

The database or domain service remains authoritative. Use fail-open reads only when a miss can be
recomputed safely; use `Unavailable` status to make fail-closed decisions when data absence is
unsafe. Redis transient failures return safe unavailable results without connection strings or
exception bodies. The adapter does not provide locks, sessions, or cache-as-database semantics.

## Adoption and rollback

Wrap an existing sibling cache behind `ICacheStore`, migrate key construction and versioning first,
then choose the in-memory, HybridCache, or Redis registration. Roll back by restoring the prior
registration and callers; no platform migration is required.
