# Platform.Realtime.AspNetCore

ASP.NET Core adapters for `Platform.Realtime`. Two independently adoptable
transports share the same transport-neutral contracts:

- **SSE** — `MapPlatformRealtimeSse` maps a Server-Sent Events stream with
  bounded connection management, cancellation-driven teardown, heartbeat, safe
  stream headers, and payload/tenant-routing enforcement.
- **SignalR** — `AddPlatformRealtimeSignalR` registers the SignalR pipeline with
  a connection-authorization filter and a tenant-routing hub base. The backplane
  (e.g. Redis) is **application-owned** and optional; without it the host runs
  in-process only.

Both transports require the application to register an
`IRealtimeConnectionAuthorizer` and `IRealtimeTenantRouter`; until they are
supplied the fail-closed defaults (`DenyAllRealtimeAuthorizer`,
`DenyCrossTenantRouter`) reject every connection and cross-tenant delivery.

Delivery is **non-durable**. See
[`docs/platform-realtime.md`](../docs/platform-realtime.md) for the migration
guide and the reconnect/resync contract.
