# Platform.Realtime

Transport-neutral realtime contracts for connection lifecycle, authorization,
tenant routing, payload limits, and provider status. This package has **no**
ASP.NET Core, SignalR, EF Core, or Redis dependency so non-web consumers can
adopt the contracts without pulling in a web stack.

The concrete transport adapters live in `Platform.Realtime.AspNetCore`:

- SSE streams (`MapPlatformRealtimeSse`)
- SignalR hubs (`AddPlatformRealtimeSignalR`)

Applications own hubs, event schemas, chat semantics, subscriptions, and any
Redis topology. The platform only standardizes the seams so authorization,
tenant scope, and payload handling are applied consistently.

See [`docs/platform-realtime.md`](../docs/platform-realtime.md) for the
migration guide and the non-durable delivery disclosure.
