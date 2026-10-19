## Why

`Platform.Quota` provides provider-neutral atomic reservation contracts, but consumers still need to rebuild HTTP enforcement, RFC 9457 responses, `Retry-After`, exemptions, and current-subject mapping. The starter kit demonstrates this missing ASP.NET Core boundary.

## What Changes

- Add an optional ASP.NET Core quota enforcement adapter.
- Map request identity and application resource selection into platform quota operations.
- Return safe 429 problem details and retry metadata.
- Keep plans, prices, entitlements, units, persistence, and quota limits application-owned.

## Capabilities

### New Capabilities

- `platform-quota-aspnetcore`: HTTP quota enforcement over `Platform.Quota` contracts.

### Modified Capabilities

- None.

## Impact

Adds an optional ASP.NET Core package. Existing `Platform.Quota` contracts and stores remain framework-neutral. No billing plan resolver or tenant provider is embedded.
