## Why

Catchen, VisualFlow, and the billing adapters all need webhook signature checks, replay protection, durable processing, retries, and delivery visibility. These concerns are currently repeated or provider-specific, leaving no common boundary for sibling applications.

## What Changes

- Add provider-neutral inbound webhook verification, normalization, replay, and processing contracts.
- Add outbound webhook subscription, delivery attempt, retry, and status contracts.
- Add optional ASP.NET Core endpoint helpers and EF Core persistence adapters.
- Add SSRF-safe target validation and secret redaction rules.
- Keep billing-provider event normalization and application event payloads outside the generic package.

## Capabilities

### New Capabilities

- `platform-webhooks`: inbound and outbound webhook lifecycle contracts and optional adapters.

### Modified Capabilities

- None.

## Impact

- New `Platform.Webhooks.Contracts`, `Platform.Webhooks.AspNetCore`, and optional `Platform.Webhooks.EfCore` packages.
- Public webhook verification, inbox, subscription, and delivery APIs.
- Requires careful security testing; no provider secrets, URLs, migrations, or product events are included.
