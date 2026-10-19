## Why

The platform has provider-neutral identity and authorization contracts, while the starter kit provides a complete JWT/ASP.NET Identity host integration with current-user propagation, sessions, permissions, and tenant claims. Consumers need an optional host adapter without importing the starter's product identity module.

## What Changes

- Add ASP.NET Core authentication/current-user projection and authorization policy helpers.
- Add optional EF Core identity persistence seams and token/session adapter contracts.
- Add identity audit hooks, claim mapping, and explicit configuration validation.
- Keep user entities, token format, tenant provisioning, permissions, and credential policy application-owned.

## Capabilities

### New Capabilities

- `platform-identity-host-integration`: Optional ASP.NET Core and persistence integration for platform identity contracts.

### Modified Capabilities

- None.

## Impact

Adds optional ASP.NET Core authentication/authorization and EF Core integration packages. Existing identity contracts remain provider-neutral. No application user schema or JWT implementation is prescribed.
