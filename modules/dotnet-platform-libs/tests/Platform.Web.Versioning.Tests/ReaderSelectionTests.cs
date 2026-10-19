using Asp.Versioning;
using Asp.Versioning.ApiExplorer;
using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Hosting;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.TestHost;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Options;
using Platform.Web.Versioning;
using Platform.Web.Versioning.DependencyInjection;

namespace Platform.Web.Versioning.Tests;

public sealed class ReaderSelectionTests
{
    [Fact]
    public void UrlSegment_reader_is_configured_by_default()
    {
        using var provider = Build();
        var defaults = provider.GetRequiredService<IPlatformVersioningDefaultsProvider>();
        var versioningOptions = provider.GetRequiredService<IOptions<ApiVersioningOptions>>().Value;
        Assert.IsType<UrlSegmentApiVersionReader>(versioningOptions.ApiVersionReader);
        Assert.Equal(new ApiVersion(1, 0), defaults.DefaultApiVersion);
        Assert.True(defaults.AssumeDefaultVersionWhenUnspecified);
    }

    [Fact]
    public void Header_reader_uses_configured_header_name()
    {
        using var provider = Build(options =>
        {
            options.Reader = PlatformVersionReaderKind.Header;
            options.HeaderName = "X-Api";
        });
        var versioningOptions = provider.GetRequiredService<IOptions<ApiVersioningOptions>>().Value;
        var reader = Assert.IsType<HeaderApiVersionReader>(versioningOptions.ApiVersionReader);
        Assert.Contains("X-Api", reader.HeaderNames);
    }

    [Fact]
    public void Query_reader_uses_configured_parameter_name()
    {
        using var provider = Build(options =>
        {
            options.Reader = PlatformVersionReaderKind.QueryString;
            options.QueryParameterName = "ver";
        });
        var versioningOptions = provider.GetRequiredService<IOptions<ApiVersioningOptions>>().Value;
        var reader = Assert.IsType<QueryStringApiVersionReader>(versioningOptions.ApiVersionReader);
        Assert.Contains("ver", reader.ParameterNames);
    }

    [Fact]
    public void MediaType_reader_is_registered_for_opt_in()
    {
        using var provider = Build(options => options.Reader = PlatformVersionReaderKind.MediaType);
        var versioningOptions = provider.GetRequiredService<IOptions<ApiVersioningOptions>>().Value;
        Assert.IsType<MediaTypeApiVersionReader>(versioningOptions.ApiVersionReader);
    }

    [Fact]
    public void Composite_reader_combines_query_and_url_segment()
    {
        using var provider = Build(options => options.Reader = PlatformVersionReaderKind.Composite);
        var versioningOptions = provider.GetRequiredService<IOptions<ApiVersioningOptions>>().Value;
        Assert.IsNotType<UrlSegmentApiVersionReader>(versioningOptions.ApiVersionReader);
        Assert.IsNotType<QueryStringApiVersionReader>(versioningOptions.ApiVersionReader);
    }

    [Fact]
    public void ApiExplorer_options_adopt_group_name_format()
    {
        using var provider = Build(options => options.GroupNameFormat = "'api'VVV");
        var explorerOptions = provider.GetRequiredService<IOptions<ApiExplorerOptions>>().Value;
        Assert.Equal("'api'VVV", explorerOptions.GroupNameFormat);
    }

    [Fact]
    public void ReportApiVersions_toggles_the_versioning_options()
    {
        using var provider = Build(options => options.ReportApiVersions = true);
        var versioningOptions = provider.GetRequiredService<IOptions<ApiVersioningOptions>>().Value;
        Assert.True(versioningOptions.ReportApiVersions);
    }

    [Fact]
    public void RouteConstraintName_is_applied_to_the_versioning_options()
    {
        using var provider = Build(options => options.RouteConstraintName = "api-version");
        var versioningOptions = provider.GetRequiredService<IOptions<ApiVersioningOptions>>().Value;
        Assert.Equal("api-version", versioningOptions.RouteConstraintName);
    }

    private static ServiceProvider Build(Action<PlatformWebVersioningOptions>? configure = null)
    {
        var services = new ServiceCollection();
        services.AddLogging();
        services.AddPlatformWebVersioning(configure ?? (_ => { }));
        return services.BuildServiceProvider();
    }
}
