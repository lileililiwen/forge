# Platform.Realtime

Opt-in, application-owned realtime transports over provider-neutral contracts. The
platform supplies the *seams* — connection authorization, tenant routing, payload
and connection bounds, and provider status — but never owns hubs, event schemas,
chat semantics, subscriptions, or a Redis topology.

## Packages

| Package | Role |
| --- | --- |
| `Platform.Realtime` | Transport-neutral contracts: connection options, authorization, tenant routing, bounded messages, provider status, delivery disclosure. No ASP.NET Core / SignalR / Redis dependency. |
| `Platform.Realtime.AspNetCore` | ASP.NET Core SSE and SignalR adapters. Depends only on the `Microsoft.AspNetCore.App` framework reference; the backplane is application-owned and optional. |

## Why two packages

`Platform.Realtime` stays framework-neutral so non-web consumers can build against
the contracts. The ASP.NET Core adapters reference the shared framework only; an
SSE-only consumer maps streams without installing SignalR or Redis as separate
packages.

## Registration

```csharp
// Shared contracts + SSE
builder.Services.AddPlatformRealtimeAspNetCore(options =>
{
    options.MaxConcurrentConnections = 1000;
    options.MaxPayloadBytes = 64 * 1024;
    options.HeartbeatInterval = TimeSpan.FromSeconds(30);
});

// Optional SignalR transport (no backplane by default)
builder.Services.AddPlatformRealtimeSignalR(options =>
{
    // Application-owned backplane seam, e.g. Redis:
    // options.ConfigureBackplane = signalR => signalR.AddStackExchangeRedis(...);
});

// Application-owned decisions (fail-closed defaults reject everything otherwise)
builder.Services.AddSingleton<IRealtimeConnectionAuthorizer, MyAuthorizer>();
builder.Services.AddSingleton<IRealtimeTenantRouter, MyRouter>();
builder.Services.AddSingleton<IRealtimeCallerResolver, MyCallerResolver>();
builder.Services.AddSingleton<IRealtimeSseSource, MySseSource>();
```

## SSE

```csharp
app.MapPlatformRealtimeSse("/realtime/sse");
app.MapPlatformRealtimeStatus();      // safe topology-free health
app.MapPlatformRealtimeResync();      // client reconnect/resync hook
```

The endpoint:

1. resolves the caller via `IRealtimeCallerResolver`,
2. runs `IRealtimeConnectionAuthorizer` — rejects with `401` when denied,
3. acquires a bounded connection slot — rejects with `503` when full,
4. sets `text/event-stream`, `no-cache`, and `X-Accel-Buffering: no` headers,
5. streams `IRealtimeSseSource`, emitting an SSE heartbeat comment on the
   configured interval,
6. releases the connection slot on disconnect or cancellation (no unbounded
   background task).

Each message written to `IRealtimeMessageSink` is checked against the application
`IRealtimeTenantRouter` and the payload bound; rejected or oversized frames are
dropped before reaching the client.

## SignalR

`AddPlatformRealtimeSignalR` registers SignalR with a connection-authorization
filter (`IHubFilter`) and a tenant-routing-aware `RealtimeHubBase`. Applications
subclass the base hub and map it with `MapHub<T>`. The backplane is a
**deliberately application-owned seam**: supply `RealtimeSignalROptions.ConfigureBackplane`
to enable distributed scale-out; without it the host runs in-process.

## Migration from the starter SSE / SignalR

The starter `BuildingBlocks/Web/Sse`, `BuildingBlocks/Web/Realtime`, and the
middleware tests were a reference for connection lifecycle and cancellation only.
When migrating an endpoint:

1. Replace the bespoke SSE loop with `MapPlatformRealtimeSse` and move your
   per-connection streaming into an `IRealtimeSseSource`.
2. Move connection gating into `IRealtimeConnectionAuthorizer` and tenant scope
   into `IRealtimeTenantRouter` instead of inline middleware.
3. Register your hubs against `AddPlatformRealtimeSignalR` and subclass
   `RealtimeHubBase`; remove any in-hub Redis backplane wiring and express it
   through `ConfigureBackplane`.
4. Compare authorization and disconnect behavior: the adapter must reject
   unauthorized connections (`401`) and release connection slots on disconnect.
5. Rollback removes the adapter registration and leaves application hubs/streams
   intact.

Do **not** copy chat channels, module events, tenant implementation types, or
Redis assumptions from the starter — those remain application-owned.

## Non-durable delivery (required disclosure)

Realtime delivery is **non-durable**. `Platform.Realtime.Delivery.RealtimeDelivery.IsDurable`
is always `false`. The platform makes no guarantee that a message sent while a
client was disconnected will ever be received. Responsibilities:

- Applications own replay or resynchronization.
- On reconnect, a client calls the resync endpoint (`MapPlatformRealtimeResync`)
  which forwards a `RealtimeResyncRequest` to the application
  `IRealtimeResyncHandler`; the platform does not redeliver missed messages.
- When no `IRealtimeResyncHandler` is registered, the endpoint returns `404` so
  clients learn resync is not configured.
