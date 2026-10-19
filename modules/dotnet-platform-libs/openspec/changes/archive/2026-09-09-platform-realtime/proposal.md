## Why

The starter kit includes SignalR and SSE infrastructure with tenant-aware connection/token handling, while the platform has no reusable realtime adapter. This forces each application to rebuild transport lifecycle, authorization, and provider health behavior.

## What Changes

- Add provider-neutral realtime connection and message contracts where needed.
- Add optional ASP.NET Core SignalR and SSE adapters.
- Add application-owned authorization, tenant scope, payload, and hub/stream registration seams.
- Add bounded connection lifecycle, cancellation, and health tests.

## Capabilities

### New Capabilities

- `platform-realtime`: Optional SignalR/SSE host adapters and safe realtime contracts.

### Modified Capabilities

- None.

## Impact

Adds optional ASP.NET Core/SignalR packages. No product hubs, event payloads, Redis topology, or authorization policy moves into the platform.
