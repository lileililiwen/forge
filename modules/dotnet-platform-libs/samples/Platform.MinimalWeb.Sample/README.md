# Minimal web sample (stage 1)

The smallest platform adoption: the shared web runtime via
`Platform.Starter` with every optional capability disabled.

## Owned by the application

- The `/` endpoint and any further routes.
- The decision to enable identity, admin, billing, AI, notifications,
  or SMS later through `PlatformApplicationOptions`.

## Owned by the platform

- Correlation, safe errors, security headers, request limits, and
  timeout middleware (`Platform.Web`).
- The `/live` liveness endpoint mapped by
  `MapPlatformApplicationEndpoints`.

## Rollback

Remove the `Platform.Starter` project reference and the
`AddPlatformApplication` / `UsePlatformApplication` /
`MapPlatformApplicationEndpoints` calls; the remaining minimal API
keeps working.

## Non-goals

Not a recommended architecture and not a product: no database,
identity, frontend, or deployment stack.
