## Why

The starter kit repeatedly supplies web-edge capabilities—CORS, HTTP resilience, OpenAPI, validation, versioning, and richer observability—that are absent from the current `Platform.Web` runtime. These should be added as small opt-in extensions rather than turning the runtime package into another monolith.

## What Changes

- Add optional CORS configuration and safe production validation.
- Add HTTP client resilience conventions for retry, timeout, and circuit breaker policies.
- Add optional OpenAPI and API-versioning helpers with secure defaults.
- Add reusable pagination/validation integration points without requiring a particular mediator or validator framework.
- Define platform observability conventions for request and provider instrumentation.
- Document package-level folder and dependency organization.

## Capabilities

### New Capabilities

- `platform-web-edge`: opt-in web-edge integrations and observability conventions.

### Modified Capabilities

- None.

## Impact

- New focused packages such as `Platform.Web.Cors`, `Platform.Web.Resilience`, and `Platform.Web.OpenApi`, or equivalent package grouping after dependency review.
- Optional ASP.NET Core and Microsoft resilience/OpenAPI dependencies.
- No mandatory middleware expansion of `Platform.Web`; no SignalR/SSE implementation in this change.
