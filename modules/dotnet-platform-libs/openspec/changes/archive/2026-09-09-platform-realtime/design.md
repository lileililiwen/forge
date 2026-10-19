## Context

Starter references include `BuildingBlocks/Web/Sse`, realtime registration, token services, connection management, SignalR Redis backplane setup, and dashboard/chat consumers.

Starter-kit references:

- `dotnet-starter-kit/src/BuildingBlocks/Web/Sse/`
- `dotnet-starter-kit/src/BuildingBlocks/Web/Realtime/`
- `dotnet-starter-kit/src/BuildingBlocks/Web/Extensions.cs`
- `dotnet-starter-kit/src/Tests/Integration.Middleware.Tests/`

Copy lifecycle and cancellation test patterns only. Do not copy chat channels, module events, tenant implementation types, or Redis assumptions.

## Goals / Non-Goals

**Goals:**

- Provide opt-in SSE and SignalR registration with connection authorization and tenant/subject context seams.
- Standardize cancellation, bounded payload/connection limits, and provider status.
- Allow applications to choose in-process or distributed backplanes.

**Non-Goals:**

- Owning hubs, event schemas, chat semantics, subscriptions, Redis clients, or frontend protocols beyond the transport boundary.

## Decisions

1. Separate transport-neutral contracts from `Platform.Realtime.AspNetCore` adapters.
2. Require application callbacks for connection authorization, tenant scope, and message serialization.
3. Keep SSE connection management bounded and cancellation-driven; do not guarantee durable delivery.
4. Make SignalR backplane selection application-owned and optional.

Alternative rejected: placing SignalR in the core platform would make non-web consumers install ASP.NET and Redis dependencies.

## Risks / Trade-offs

- [Risk] Long-lived connections exhaust resources → enforce limits, heartbeat/timeout options, and cancellation tests.
- [Risk] Cross-tenant broadcasts leak data → require application authorization and tenant routing callbacks.
- [Risk] SSE/SignalR are not durable → document reconnect/resync responsibility for applications.

## Migration Plan

Adopt the adapter for one existing SSE or SignalR endpoint, compare authorization and disconnect behavior, then migrate other endpoints. Rollback removes adapter registration and leaves application hubs/streams intact.

## Open Questions

- Whether SignalR and SSE should ship as separate packages for consumers that need only one transport.
