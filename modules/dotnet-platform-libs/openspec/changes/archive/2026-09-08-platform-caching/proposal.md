## Why

Caching is repeated in the starter kit, VisualFlow, and Mortalect, but each application owns incompatible cache keys, failure behavior, and provider wiring. A small cache boundary would let sibling applications share safe conventions without forcing Redis or a specific serialization stack.

## What Changes

- Add provider-neutral cache contracts for get, set, remove, expiration, and tags.
- Add safe cache-key construction and optional cache telemetry contracts.
- Add in-memory and HybridCache-backed implementations for development and single-host deployments.
- Add a separate optional Redis adapter with health reporting.
- Add deterministic tests for hits, misses, expiry, invalidation, provider failure, and key isolation.

## Capabilities

### New Capabilities

- `platform-caching`: cache contracts, key/tag conventions, default implementations, and optional distributed adapter seams.

### Modified Capabilities

- None.

## Impact

- New `Platform.Caching`, `Platform.Caching.Hybrid`, and optional `Platform.Caching.Redis` packages.
- New public cache APIs and telemetry names.
- Redis and ASP.NET Core dependencies remain outside the base contract package.
