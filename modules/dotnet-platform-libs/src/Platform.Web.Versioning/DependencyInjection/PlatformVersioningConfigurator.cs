using Asp.Versioning;
using Asp.Versioning.ApiExplorer;
using Microsoft.Extensions.Options;

namespace Platform.Web.Versioning.DependencyInjection;

/// <summary>Applies the platform <see cref="PlatformWebVersioningOptions"/> to the Asp.Versioning and API Explorer options.</summary>
internal static class PlatformVersioningConfigurator
{
    /// <summary>Configures the Asp.Versioning options from the platform options.</summary>
    public static void Configure(ApiVersioningOptions target, IOptions<PlatformWebVersioningOptions> platform)
    {
        var options = platform.Value;
        target.DefaultApiVersion = new ApiVersion(options.DefaultMajor, options.DefaultMinor);
        target.AssumeDefaultVersionWhenUnspecified = options.AssumeDefaultVersionWhenUnspecified;
        target.ReportApiVersions = options.ReportApiVersions;
        target.RouteConstraintName = options.RouteConstraintName;
        target.ApiVersionReader = ResolveReader(options);
    }

    /// <summary>Configures the API Explorer options from the platform options.</summary>
    public static void ConfigureExplorer(ApiExplorerOptions target, IOptions<PlatformWebVersioningOptions> platform)
    {
        var options = platform.Value;
        target.GroupNameFormat = options.GroupNameFormat;
        target.AssumeDefaultVersionWhenUnspecified = options.AssumeDefaultVersionWhenUnspecified;
    }

    private static IApiVersionReader ResolveReader(PlatformWebVersioningOptions options) => options.Reader switch
    {
        PlatformVersionReaderKind.Header => new HeaderApiVersionReader(options.HeaderName),
        PlatformVersionReaderKind.QueryString => new QueryStringApiVersionReader(options.QueryParameterName),
        PlatformVersionReaderKind.MediaType => new MediaTypeApiVersionReader(),
        PlatformVersionReaderKind.Composite => ApiVersionReader.Combine(
            new QueryStringApiVersionReader(options.QueryParameterName),
            new UrlSegmentApiVersionReader()),
        _ => new UrlSegmentApiVersionReader(),
    };
}
