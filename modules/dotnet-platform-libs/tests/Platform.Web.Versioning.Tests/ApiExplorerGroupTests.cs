using Asp.Versioning;
using Asp.Versioning.ApiExplorer;
using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Hosting;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.Routing;
using Microsoft.AspNetCore.TestHost;
using Microsoft.Extensions.DependencyInjection;
using Platform.Web.Versioning.DependencyInjection;

namespace Platform.Web.Versioning.Tests;

public sealed class ApiExplorerGroupTests
{
    [Fact]
    public async Task Distinct_versions_produce_distinct_api_explorer_groups()
    {
        await using var app = BuildApp(options =>
        {
            options.GroupNameFormat = "'v'VVV";
        }, endpoints =>
        {
            var versioned = endpoints.NewVersionedApi();
            versioned.MapGet("/v{version:apiVersion}/ping", () => Results.Ok(new { v = 1 })).HasApiVersion(1, 0);
            versioned.MapGet("/v{version:apiVersion}/ping", () => Results.Ok(new { v = 2 })).HasApiVersion(2, 0);
        });

        var provider = app.Services.GetRequiredService<IApiVersionDescriptionProvider>();
        var groups = provider.ApiVersionDescriptions
            .Select(d => d.GroupName)
            .ToArray();

        Assert.Equal(2, groups.Length);
        Assert.Contains("v1", groups);
        Assert.Contains("v2", groups);
    }

    [Fact]
    public async Task Group_name_format_round_trips_through_consumer_supplied_convention()
    {
        await using var app = BuildApp(options =>
        {
            options.GroupNameFormat = "'v'VVV";
        }, endpoints =>
        {
            var versioned = endpoints.NewVersionedApi();
            versioned.MapGet("/v{version:apiVersion}/ping", () => Results.Ok()).HasApiVersion(1, 0);
            versioned.MapGet("/v{version:apiVersion}/ping", () => Results.Ok()).HasApiVersion(2, 0);
        });

        var seenGroups = new List<string>();
        var seenDescriptions = new List<ApiVersionDescription>();
        app.MapPlatformApiExplorerDescriptions((routeGroup, description) =>
        {
            seenGroups.Add(description.GroupName ?? string.Empty);
            seenDescriptions.Add(description);
        });

        Assert.Equal(2, seenGroups.Count);
        Assert.Contains("v1", seenGroups);
        Assert.Contains("v2", seenGroups);
        Assert.Equal(2, seenDescriptions.Count);
        Assert.All(seenDescriptions, d => Assert.NotNull(d.ApiVersion));
    }

    [Fact]
    public void Map_without_registration_throws_a_configuration_error()
    {
        var builder = WebApplication.CreateBuilder(new WebApplicationOptions { EnvironmentName = "Testing" });
        builder.WebHost.UseTestServer();
        var app = builder.Build();
        Assert.Throws<InvalidOperationException>(() => app.MapPlatformApiExplorerDescriptions((_, _) => { }));
    }

    private static WebApplication BuildApp(
        Action<Platform.Web.Versioning.PlatformWebVersioningOptions>? configure,
        Action<IEndpointRouteBuilder> configureEndpoints)
    {
        var builder = WebApplication.CreateBuilder(new WebApplicationOptions { EnvironmentName = "Testing" });
        builder.WebHost.UseTestServer();
        builder.Services.AddPlatformWebVersioning(configure ?? (_ => { }));
        var app = builder.Build();
        configureEndpoints(app);
        app.StartAsync().GetAwaiter().GetResult();
        return app;
    }
}
