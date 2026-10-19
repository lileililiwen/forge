# Platform.Web.Versioning

Optional API versioning and API Explorer conventions for ASP.NET Core. Application
opt-in only; unversioned endpoints are unaffected.

## Packages

| Package | Purpose |
| --- | --- |
| `Platform.Web.Versioning` | Opt-in `IApiVersioningBuilder` registration, configurable version readers, API Explorer group naming, and minimal API binding. |

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

The platform does not own the `Asp.Versioning` configuration beyond the options above;
applications may still call `IApiVersioningBuilder` extensions (for example, to declare
versioned policies or to enable minimal-API parameter binding).

## Options

- `PlatformWebVersioningOptions` — `DefaultMajor` / `DefaultMinor`, `AssumeDefaultVersionWhenUnspecified`, `ReportApiVersions`, `RouteConstraintName`, `GroupNameFormat`, `Reader`, `HeaderName`, `QueryParameterName`. `Validate()` enforces non-negative versions, required reader-specific names, and non-empty format strings.

## Reader selection

- `PlatformVersionReaderKind.UrlSegment` — the recommended default; reads `v1` from `/v{version:apiVersion}/...`.
- `PlatformVersionReaderKind.Header` — reads from `X-Api-Version` (configurable).
- `PlatformVersionReaderKind.QueryString` — reads from `?api-version=1` (configurable).
- `PlatformVersionReaderKind.MediaType` — reads from the request media type.
- `PlatformVersionReaderKind.Composite` — combines `QueryString` and `UrlSegment` (the Asp.Versioning default).

## API Explorer integration

`MapPlatformApiExplorerDescriptions(configure)` groups the descriptions produced by
`IApiVersionDescriptionProvider` by `GroupNameFormat` and routes each group under
`/v<groupName>`. The platform never generates OpenAPI documents; consumers that need
JSON should still register an `IPlatformOpenApiDocumentProvider` and serve the document
themselves. The platform's OpenAPI package and the versioning package are intentionally
independent — applications wire them together inside their own composition root.

## Migration from the starter kit

1. Replace `AddHeroApiVersioning` with `AddPlatformWebVersioning`.
2. Move any custom reader configuration to `PlatformWebVersioningOptions`.
3. Keep the existing `HasApiVersion` calls unchanged.
4. Verify generated routes; rollback is removing the registration extension (the
   existing routes continue to work, with the platform reader reverted to the
   framework default).
