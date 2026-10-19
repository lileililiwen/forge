## Context

`Platform.Web` already owns correlation, errors, headers, request limits, timeouts, and readiness. The starter adds several adjacent web capabilities, while sibling applications need consistent API security and outbound HTTP behavior. The repository also has many package roots, so this change must improve organization without flattening all concerns into one project.

## Goals / Non-Goals

**Goals:**

- Add focused, independently adoptable web-edge packages.
- Validate dangerous production defaults such as allow-all CORS and missing API metadata.
- Define redaction-safe request/provider telemetry conventions.
- Preserve the existing web runtime API and dependency direction.

**Non-Goals:**

- No mandatory OpenAPI UI, CORS policy, resilience handler, FluentValidation, Serilog, or OpenTelemetry exporter.
- No SignalR, SSE, frontend client, or application module loader in this change.
- No mass filesystem move of existing packages.

## Decisions

- **Prefer focused package roots over deep nesting.** Each independently versionable dependency group gets a top-level `src/Platform.*` root. Within it, use shallow folders by concern (`Contracts`, `Middleware`, `DependencyInjection`, `Telemetry`). Solution folders and documentation provide visual grouping.
- **Keep integrations optional.** CORS, resilience, and OpenAPI packages reference only the frameworks they need; `Platform.Web` remains the stable baseline.
- **Use host-owned policy.** The platform provides validation and conventions, while applications choose origins, API titles, retry scopes, and exporters.
- **Copy starter configuration patterns selectively.** Port validation and safe defaults, not FSH names, routes, UI, or logging stack assumptions.
- **Test at the boundary.** Use unit tests for options and telemetry plus TestServer tests for middleware/endpoint behavior; avoid requiring external services.

## Risks / Trade-offs

- [Risk] Too many tiny packages increase version-management overhead → [Mitigation] group only dependencies that can be adopted together and document a package matrix.
- [Risk] Retrying non-idempotent HTTP calls causes duplicate side effects → [Mitigation] default to safe methods/statuses and require explicit opt-in for broader retries.
- [Risk] OpenAPI metadata exposes internal endpoints → [Mitigation] make document mapping explicit and preserve endpoint authorization metadata.

## Migration Plan

Existing applications keep using `Platform.Web`. A consumer adds one edge package at a time, configures it through its own options, and removes duplicate startup code after parity tests pass. Rollback removes the extension package and its registration; no data migration is required.

## Open Questions

- Which OpenAPI implementation should be supported first: the built-in .NET 9/10 stack or a third-party abstraction compatible with .NET 8.
- Whether API versioning deserves its own change after the first consumer pilot.
