## Why

The starter kit implements refresh tokens, sessions, password lifecycle, two-factor authentication, impersonation grants, groups, and user lifecycle events. The platform currently provides authentication and authorization seams but not normalized contracts for these repeated identity workflows.

## What Changes

- Add provider-neutral identity lifecycle contracts for sessions, refresh tokens, password recovery, two-factor operations, and impersonation.
- Add result/error semantics that do not expose secrets or provider-specific entities.
- Add ASP.NET Core integration seams where transport behavior is genuinely reusable.
- Add deterministic testing helpers and consumer conformance scenarios.
- Keep users, roles, claims, Identity stores, EF mappings, migrations, and product authorization policy application-owned.

## Capabilities

### New Capabilities

- `platform-identity-lifecycle`: reusable identity lifecycle contracts and application adapters.

### Modified Capabilities

- `platform-identity-authorization`: extend the existing identity boundary with lifecycle integration seams.

## Impact

Adds public contracts and optional adapters under `Platform.Identity.*`, with potential package/API versioning implications. It must not introduce application-specific Identity entities or database schema.
