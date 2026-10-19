# Platform.Web.Versioning

Optional ASP.NET Core API versioning and API Explorer conventions. Application
opt-in only; unversioned endpoints are unaffected.

## Packages

| Package | Purpose |
| --- | --- |
| `Platform.Web.Versioning` | Opt-in `IApiVersioningBuilder` registration, configurable version readers, API Explorer group naming, and minimal API binding. |

The package depends on `Platform.Core`, `Microsoft.AspNetCore.App` (via
`FrameworkReference`), and the centrally managed `Asp.Versioning.Http` and
`Asp.Versioning.Mvc.ApiExplorer` packages. It does not reference Swashbuckle,
NSwag, EF Core, or application projects.

## Adoption

```csharp
services.AddPlatformWebVersioning(options =>
{
    options.DefaultMajor = 1;
    options.DefaultMinor = 0;
    options.AssumeDefaultVersionWhenUnspecified = true;
    options.Reader = PlatformVersionReaderKind.UrlSegment;
    options.GroupNameFormat = "'v'VVV";
});
```

After registration, mark endpoints with `HasApiVersion(...)`:

```csharp
app.MapGet("/v{version:apiVersion}/ping", () => Results.Ok("pong"))
    .HasApiVersion(1, 0);
```

The platform does not own the `Asp.Versioning` configuration beyond the
options above; applications may still call `IApiVersioningBuilder` extensions
(for example, to declare versioned policies or to enable minimal-API parameter
binding via `EnablePlatformApiVersionBinding`).

## Options

- `PlatformWebVersioningOptions`
  - `DefaultMajor` / `DefaultMinor` — default `1.0`.
  - `AssumeDefaultVersionWhenUnspecified` — default `true`.
  - `ReportApiVersions` — adds `api-supported-versions` and
    `api-deprecated-versions` response headers; default `false`.
  - `RouteConstraintName` — default `apiVersion`.
  - `GroupNameFormat` — default `'v'VVV` (renders `v1`, `v1.1`).
  - `Reader` — see "Reader selection" below; default `UrlSegment`.
  - `HeaderName` — default `X-Api-Version` (used by the `Header` reader).
  - `QueryParameterName` — default `api-version` (used by the `QueryString`
    reader).
  - `Validate()` rejects negative versions, missing reader-specific names,
    and empty format strings; the registration extension installs an
    `IValidateOptions<>` so the failure surfaces at first resolution.

## Reader selection

- `PlatformVersionReaderKind.UrlSegment` — the recommended default; reads
  `v1` from `/v{version:apiVersion}/...`.
- `PlatformVersionReaderKind.Header` — reads from `X-Api-Version`
  (configurable).
- `PlatformVersionReaderKind.QueryString` — reads from
  `?api-version=1` (configurable).
- `PlatformVersionReaderKind.MediaType` — reads from the request media
  type.
- `PlatformVersionReaderKind.Composite` — combines `QueryString` and
  `UrlSegment` (the Asp.Versioning default).

## API Explorer integration

`MapPlatformApiExplorerDescriptions(configure)` groups the descriptions
produced by `IApiVersionDescriptionProvider` by `GroupNameFormat` and
invokes `configure` for each description so the application can attach
its own endpoints, authorization, or output formatters. The platform
never generates OpenAPI documents; consumers that need JSON should
still register an `IPlatformOpenApiDocumentProvider` and serve the
document themselves via `Platform.Web.OpenApi`. The platform's OpenAPI
package and the versioning package are intentionally independent —
applications wire them together inside their own composition root.

`EnablePlatformApiVersionBinding(IApiVersioningBuilder)` is an opt-in
helper for consumers that want to bind `ApiVersion` directly in
minimal-API route signatures.

## Migration from the starter kit

1. Replace `AddHeroApiVersioning` (or any custom
   `services.AddApiVersioning(...)` setup) with
   `AddPlatformWebVersioning(options => ...)`.
2. Move any custom reader, default-version, or group-format
   configuration to `PlatformWebVersioningOptions`.
3. Keep the existing `HasApiVersion(...)` calls unchanged.
4. If the application previously referenced the starter's
   `IApiVersionDescriptionProvider` directly, the platform's
   `MapPlatformApiExplorerDescriptions` provides the same hook.
5. Verify generated routes; rollback is removing the registration
   extension (the existing routes continue to work, with the platform
   reader reverted to the framework default).
