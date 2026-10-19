## Why

The starter kit treats auditing as a first-class capability across HTTP, security, exceptions, entity changes, masking, retention, and durable sinks. The platform currently exposes only audit primitives and identity hooks, leaving every consumer to rebuild compliance-sensitive behavior.

## What Changes

- Add provider-neutral audit event, scope, enrichment, masking, sink, and retention contracts.
- Add optional ASP.NET Core and EF Core capture adapters.
- Provide safe defaults for sensitive data handling, correlation, subject, tenant, and failure classification.
- Keep audit storage schema, retention schedule, product projections, and migrations application-owned.

## Capabilities

### New Capabilities

- `platform-auditing`: Composable audit contracts and optional HTTP/EF capture adapters.

### Modified Capabilities

- None.

## Impact

Adds contracts plus optional ASP.NET Core/EF Core packages. Existing `IAuditable`/save-change behavior remains compatible. No product audit tables, query endpoints, or business-specific event payloads are prescribed.
