## Why

The starter kit has a small reusable API-versioning setup based on URL segments and API Explorer, while the platform has no equivalent package. Without a common contract, the 26+ applications will diverge in URL shape, default-version behavior, and generated API documentation.

## What Changes

- Add an opt-in `Platform.Web.Versioning` ASP.NET Core package.
- Provide configurable default version, URL-segment/header/query readers, and API Explorer grouping.
- Keep version policy application-configurable and preserve existing unversioned applications.
- Add OpenAPI integration guidance and TestServer coverage.

## Capabilities

### New Capabilities

- `platform-web-api-versioning`: common opt-in API versioning and explorer conventions.

### Modified Capabilities

- None.

## Impact

Adds an ASP.NET Core adapter and centrally managed `Asp.Versioning.*` dependencies. It affects web composition only; domain contracts and endpoint ownership remain in applications.
