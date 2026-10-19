using Asp.Versioning;
using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Hosting;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.Routing;
using Microsoft.AspNetCore.TestHost;
using Microsoft.Extensions.DependencyInjection;
using Platform.Web.Versioning;
using Platform.Web.Versioning.DependencyInjection;

namespace Platform.Web.Versioning.Tests;

public sealed class VersioningRegistrationTests
{
    [Fact]
    public async Task Unregistered_app_serves_routes_without_versioning()
    {
        await using var app = BuildApp(registerVersioning: false, configureEndpoints: endpoints =>
        {
            endpoints.MapGet("/ping", () => Results.Ok(new { status = "ok" }));
        });
        var client = app.GetTestClient();
        var response = await client.GetAsync("/ping");
        Assert.Equal(StatusCodes.Status200OK, (int)response.StatusCode);
    }

    [Fact]
    public async Task Versioned_route_under_default_assumed_version_routes_to_v1_endpoint()
    {
        await using var app = BuildApp(registerVersioning: true, configureEndpoints: endpoints =>
        {
            var versioned = endpoints.NewVersionedApi();
            versioned.MapGet("/v{version:apiVersion}/ping", () => Results.Ok(new { version = "1.0" })).HasApiVersion(1, 0);
            versioned.MapGet("/v{version:apiVersion}/ping", () => Results.Ok(new { version = "2.0" })).HasApiVersion(2, 0);
        });
        var client = app.GetTestClient();
        var response = await client.GetAsync("/v1/ping");
        Assert.Equal(StatusCodes.Status200OK, (int)response.StatusCode);
        Assert.Contains("\"version\":\"1.0\"", await response.Content.ReadAsStringAsync());
    }

    [Fact]
    public async Task Default_version_when_unspecified_routes_to_default_version()
    {
        await using var app = BuildApp(registerVersioning: true, configureEndpoints: endpoints =>
        {
            var versioned = endpoints.NewVersionedApi();
            versioned.MapGet("/v{version:apiVersion}/ping", () => Results.Ok(new { version = "1.0" })).HasApiVersion(1, 0);
        });
        var client = app.GetTestClient();
        var response = await client.GetAsync("/v1/ping");
        Assert.Equal(StatusCodes.Status200OK, (int)response.StatusCode);
        Assert.Contains("\"version\":\"1.0\"", await response.Content.ReadAsStringAsync());
    }

    [Fact]
    public async Task Header_reader_routes_via_custom_header()
    {
        await using var app = BuildApp(registerVersioning: true, configure: options =>
        {
            options.Reader = PlatformVersionReaderKind.Header;
            options.HeaderName = "X-Api-Version";
        }, configureEndpoints: endpoints =>
        {
            var versioned = endpoints.NewVersionedApi();
            versioned.MapGet("/ping", () => Results.Ok(new { version = "2.0" })).HasApiVersion(2, 0);
        });
        var client = app.GetTestClient();
        client.DefaultRequestHeaders.Add("X-Api-Version", "2.0");
        var response = await client.GetAsync("/ping");
        Assert.Equal(StatusCodes.Status200OK, (int)response.StatusCode);
        Assert.Contains("\"version\":\"2.0\"", await response.Content.ReadAsStringAsync());
    }

    [Fact]
    public async Task Query_reader_routes_via_query_string()
    {
        await using var app = BuildApp(registerVersioning: true, configure: options =>
        {
            options.Reader = PlatformVersionReaderKind.QueryString;
            options.QueryParameterName = "api-version";
        }, configureEndpoints: endpoints =>
        {
            var versioned = endpoints.NewVersionedApi();
            versioned.MapGet("/ping", () => Results.Ok(new { version = "2.0" })).HasApiVersion(2, 0);
        });
        var client = app.GetTestClient();
        var response = await client.GetAsync("/ping?api-version=2.0");
        Assert.Equal(StatusCodes.Status200OK, (int)response.StatusCode);
        Assert.Contains("\"version\":\"2.0\"", await response.Content.ReadAsStringAsync());
    }

    [Fact]
    public async Task Defaults_provider_exposes_configured_default_version()
    {
        await using var app = BuildApp(registerVersioning: true, configure: options =>
        {
            options.DefaultMajor = 3;
            options.DefaultMinor = 4;
        });
        using var scope = app.Services.CreateScope();
        var defaults = scope.ServiceProvider.GetRequiredService<IPlatformVersioningDefaultsProvider>();
        Assert.Equal(new ApiVersion(3, 4), defaults.DefaultApiVersion);
    }

    [Fact]
    public void Repeated_registration_applies_the_latest_configuration()
    {
        var services = new ServiceCollection();
        services.AddLogging();
        services.AddPlatformWebVersioning();
        services.AddPlatformWebVersioning(options => options.DefaultMajor = 2);

        using var provider = services.BuildServiceProvider();
        var defaults = provider.GetRequiredService<IPlatformVersioningDefaultsProvider>();
        Assert.Equal(new ApiVersion(2, 0), defaults.DefaultApiVersion);
    }

    [Fact]
    public void Invalid_options_fail_at_options_resolution()
    {
        var services = new ServiceCollection();
        services.AddLogging();
        services.AddPlatformWebVersioning(options => options.DefaultMajor = -1);
        using var provider = services.BuildServiceProvider();
        Assert.Throws<Microsoft.Extensions.Options.OptionsValidationException>(
            () => provider.GetRequiredService<Microsoft.Extensions.Options.IOptions<PlatformWebVersioningOptions>>().Value);
    }

    private static WebApplication BuildApp(
        bool registerVersioning,
        Action<PlatformWebVersioningOptions>? configure = null,
        Action<IEndpointRouteBuilder>? configureEndpoints = null)
    {
        var builder = WebApplication.CreateBuilder(new WebApplicationOptions { EnvironmentName = "Testing" });
        builder.WebHost.UseTestServer();
        if (registerVersioning)
        {
            builder.Services.AddPlatformWebVersioning(configure ?? (_ => { }));
        }
        var app = builder.Build();
        configureEndpoints?.Invoke(app);
        app.StartAsync().GetAwaiter().GetResult();
        return app;
    }
}
