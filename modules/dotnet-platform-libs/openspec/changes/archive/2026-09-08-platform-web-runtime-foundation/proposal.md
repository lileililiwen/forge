# Proposal: Platform web runtime foundation

## Why

ChinaGo, Sutravia, Stylify, Triplanify, Ploutify, Smotoox, and other C# applications
repeat correlation IDs, exception handling, health endpoints, configuration checks,
security headers, request limits, and provider diagnostics. The current
`Platform.AspNetCore` package covers only part of this surface.

## Scope

Add an opt-in `Platform.Web` or equivalent package on top of `Platform.AspNetCore` for
consistent runtime bootstrap, configuration validation, security defaults, observability
hooks, request limits, and live/readiness conventions. Preserve the existing small package
boundaries and allow individual features to be enabled independently.

## Non-goals

- no authentication, identity database, tenant model, or business authorization;
- no mandatory Serilog, OpenTelemetry exporter, Redis, or database dependency;
- no replacement for application-specific health checks;
- no silent middleware registration outside documented opt-in methods.

## API impact

Adds public options, registration extensions, middleware contracts, health endpoint
contracts, and test helpers. Existing APIs remain compatible.
