## Why

The starter kit provides tenant-aware feature gates and a standard outbound HTTP resilience pipeline, but the platform does not expose either capability. These are reusable host concerns that should be adoptable without importing the starter's full web building block.

## What Changes

- Add an optional feature-management adapter with application-defined filters and endpoint metadata.
- Add an optional HTTP resilience adapter with validated retry, timeout, circuit-breaker, and bulkhead policy options.
- Define safe telemetry and failure behavior.
- Keep feature definitions, rollout state, and provider configuration application-owned.

## Capabilities

### New Capabilities

- `platform-featureflags-resilience`: Optional feature flags and outbound HTTP resilience host integrations.

### Modified Capabilities

- None.

## Impact

Adds optional ASP.NET Core, feature-management, and HTTP resilience dependencies in separate adapters or clearly isolated package areas. Core contracts remain framework-neutral. Existing `Platform.RateLimiting` and `Platform.Web` APIs are not replaced.
