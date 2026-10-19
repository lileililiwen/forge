## 1. Package structure and contracts

- [x] 1.1 Split the work into `Platform.Web.Telemetry`, `Platform.Web.Cors`,
      `Platform.Web.Resilience`, and `Platform.Web.OpenApi` after confirming
      .NET 8 compatibility and central package versions. `Platform.Web.Telemetry`
      is framework-neutral; the three ASP.NET Core packages depend on it.
- [x] 1.2 Add redaction-safe telemetry names, a configurable safe-value policy,
      and the `AddValidatedOptions<T>()` extension helper without changing the
      existing `Platform.Web` behavior. The helper is consumed by every new
      package.
- [x] 1.3 Add architecture tests verifying each edge package has only the
      intended project and framework references (no EF Core, no Swashbuckle/NSwag,
      no Polly, no VisualFlow projects).

## 2. Optional integrations

- [x] 2.1 Implement `Platform.Web.Cors` with named policy options, production
      validation (wildcard+credentials, wildcard in production, missing origin in
      production, non-absolute origins), `AddPlatformWebCors` registration, and
      TestServer coverage.
- [x] 2.2 Implement `Platform.Web.Resilience` with `PlatformHttpResilienceOptions`,
      the `PlatformHttpResilienceHandler` `DelegatingHandler` (bounded retry,
      per-attempt timeout, circuit breaker), idempotent-method handling, telemetry
      bridge, and option/handler/circuit-breaker tests.
- [x] 2.3 Implement `Platform.Web.OpenApi` with the `IPlatformOpenApiDocumentProvider`
      contract, an aggregating registry, named document options, an explicit
      `MapPlatformOpenApiDocument(name)` helper, and `MapPlatformOpenApiDocuments`
      for the registered set. The platform owns no Swashbuckle/NSwag dependency;
      applications supply the document JSON through a provider.
- [x] 2.4 Add `PlatformOptionsValidator` and `AddValidatedOptions<T>()` as a
      reusable option-validation seam that the ASP.NET Core packages consume.

## 3. Organization and adoption

- [x] 3.1 Apply the platform package convention: independent top-level roots,
      shallow `DependencyInjection` folders, single assembly marker, README,
      description element, and `Directory.Packages.props` entries.
- [x] 3.2 Port the starter validation/resilience/OpenAPI patterns selectively:
      validate the production CORS posture, default to safe HTTP methods,
      restrict retries to idempotent methods, and require explicit document
      registration for OpenAPI. No FSH-specific names, UI, or logging
      assumptions.
- [x] 3.3 Add `docs/platform-web-edge.md` and update `docs/packages.md` with
      adoption examples and a per-package reference.

## 4. Verification

- [x] 4.1 Run unit and TestServer tests for all four packages, the full
      solution build/test, package packing for the four new packages, and the
      new architecture tests in `Platform.Architecture.Tests`.
- [x] 4.2 Run `git diff --check`; document the OpenAPI implementation
      responsibility and the deferred SignalR/SSE work in the handoff.
