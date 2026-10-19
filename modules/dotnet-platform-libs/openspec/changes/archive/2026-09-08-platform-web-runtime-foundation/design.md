# Design

## Packages

- `Platform.Web` targets `net8.0` and depends on `Platform.AspNetCore` plus Microsoft
  abstractions only.
- Provider-specific observability integrations remain separate packages.
- `Platform.Testing` gains only generic host/test-server helpers when they do not create a
  production dependency.

## Bootstrap

Provide `AddPlatformWeb(Action<PlatformWebOptions>?)`, `UsePlatformWeb(Action<PlatformPipelineOptions>?)`,
and `MapPlatformRuntimeEndpoints()`. Registration must be explicit, idempotent, and
`TryAdd`-friendly. Options cover correlation policy, error detail policy, security headers,
request size, timeout, OpenAPI, and health endpoint paths.

## Runtime contracts

Expose stable live/readiness response shapes, configuration validation hooks, a redaction
interface, provider status records, and a safe exception boundary. Application checks are
composed into readiness rather than replaced.

## Verification

Use unit tests plus `WebApplication`/TestServer integration tests for ordering, safe error
responses, correlation behavior, options validation, security headers, and readiness
failure. Architecture tests forbid EF Core, provider SDKs, and application references.
