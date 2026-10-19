## 1. Boundary and contracts

- [x] 1.1 Define minimal transport-neutral connection, authorization, tenant-routing, payload, and provider-status contracts.
- [x] 1.2 Create independent ASP.NET Core SignalR and SSE adapter projects or a clearly split package layout.
- [x] 1.3 Adapt the starter `Web/Sse`, `Web/Realtime`, and middleware tests without importing chat/module types.

## 2. Transport implementation

- [x] 2.1 Implement bounded SSE connection management, cancellation, heartbeat, and safe stream headers.
- [x] 2.2 Implement SignalR registration, connection authorization, tenant routing, and optional backplane seam.
- [x] 2.3 Implement payload/connection limits and provider health without exposing topology or credentials.
- [x] 2.4 Add explicit reconnect/resynchronization documentation and hooks.

## 3. Verification and documentation

- [x] 3.1 Add unit/integration tests for authorization, tenant routing, disconnect cleanup, cancellation, limits, and no-backplane mode.
- [x] 3.2 Add architecture tests keeping core contracts free of ASP.NET Core/SignalR/Redis.
- [x] 3.3 Document migration from starter SSE/SignalR and non-durable delivery semantics.
- [x] 3.4 Run serial tests, `git diff --check`, and strict OpenSpec validation.
