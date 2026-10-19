## Dependencies

This change depends on `platform-repository-foundation` and `platform-core-contracts`.

## Components

Add a package-level service-registration extension, ProblemDetails mapping for known core failures, a correlation/request-context accessor, and health-check registration helpers. Keep middleware opt-in and composable.

Known failures map to safe status/category responses. Unknown exceptions remain under the host application's exception policy and must not expose stack traces in production.

## Compatibility

Target the selected .NET 8 ASP.NET Core baseline. Do not require a specific hosting model, serializer, logging provider, database, or authentication provider.

## Verification

Use unit tests for mappings and a minimal host integration test for registration and middleware ordering. Verify anonymous, known failure, unknown failure, and correlation scenarios.
