## Context

Starter references:

- `dotnet-starter-kit/src/BuildingBlocks/Web/FeatureFlags/Extensions.cs`
- `dotnet-starter-kit/src/BuildingBlocks/Web/FeatureFlags/TenantFeatureFilter.cs`
- `dotnet-starter-kit/src/BuildingBlocks/Web/FeatureFlags/FeatureGateEndpointFilter.cs`
- `dotnet-starter-kit/src/BuildingBlocks/Web/HttpResilience/Extensions.cs`
- `dotnet-starter-kit/src/BuildingBlocks/Web/HttpResilience/HttpResilienceOptions.cs`
- `dotnet-starter-kit/src/Tests/Framework.Tests/Web/`

Agents may reuse policy shape and tests, but must make tenant context and feature evaluation application-provided rather than copying Finbuckle or product feature names.

## Goals / Non-Goals

**Goals:**

- Provide opt-in endpoint/handler feature gates with application-defined evaluation and tenant context.
- Provide standard outbound HTTP resilience registration with explicit limits and safe telemetry.
- Avoid retrying non-idempotent requests by default and preserve cancellation.

**Non-Goals:**

- Owning a feature flag service, rollout database, plan entitlements, or product flag names.
- Hiding provider-specific resilience policy from consumers.
- Adding feature-management or Polly dependencies to framework-neutral packages.

## Decisions

1. Separate feature flag and HTTP resilience adapters so consumers can adopt one without the other.
2. Define a small feature evaluator/filter seam and endpoint metadata helper; application code supplies tenant/subject context and flag state.
3. Build the resilience pipeline from the `Microsoft.Extensions.Http.Resilience` resilience
   primitives (`ResiliencePipelineBuilder<HttpResponseMessage>` with base retry, timeout,
   circuit-breaker, and rate-limiter strategy options). The `AddStandardResilienceHandler`
   path does not populate `RequestMetadata` inside retry/circuit-breaker predicates, so the
   HTTP method is carried through a `ResilienceContext` property set by the outer
   `DelegatingHandler`. This keeps idempotency classification reliable without depending on
   the standard handler's internal initializer.
4. Exclude non-idempotent methods from automatic retries unless the caller opts in with metadata.

Alternative rejected: copying the starter's `AddHeroPlatform` registration would couple flags, resilience, and unrelated web concerns.

## Risks / Trade-offs

- [Risk] Retries amplify writes → default to safe methods and require explicit opt-in for other methods.
- [Risk] Tenant flag evaluation leaks rollout state → require application evaluator and bounded diagnostic labels.
- [Risk] Global policies hide service-specific needs → allow named clients/options and document override order.

## Migration Plan

Adopt feature gates and one outbound client in parallel with starter registrations, compare decisions and retry telemetry, then remove duplicated registrations. Rollback removes the adapters and restores prior client policies.

## Open Questions

- Whether feature-gate contracts belong in `Platform.Authorization` or remain local to the adapter package.
