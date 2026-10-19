using System.Net;
using System.Text;
using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Hosting;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.TestHost;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Platform.FeatureManagement;
using Platform.FeatureManagement.DependencyInjection;

namespace Platform.FeatureManagement.Tests;

public sealed class FeatureManagementTests
{
    [Fact]
    public async Task Disabled_feature_for_unknown_tenant_returns_configured_status()
    {
        var resolver = new StubResolver("other");
        await using var app = BuildApp(resolver);
        var client = app.GetTestClient();

        var response = await client.GetAsync("/beta");

        Assert.Equal(HttpStatusCode.NotFound, response.StatusCode);
        Assert.Contains("Feature disabled", await response.Content.ReadAsStringAsync());
    }

    [Fact]
    public async Task Enabled_feature_for_allowed_tenant_executes_endpoint()
    {
        var resolver = new StubResolver("acme");
        await using var app = BuildApp(resolver);
        var client = app.GetTestClient();

        var response = await client.GetAsync("/beta");

        Assert.Equal(HttpStatusCode.OK, response.StatusCode);
        Assert.Equal("\"beta-on\"", await response.Content.ReadAsStringAsync());
    }

    [Fact]
    public async Task Application_resolver_context_is_invoked_during_evaluation()
    {
        var resolver = new StubResolver("acme");
        await using var app = BuildApp(resolver);
        var client = app.GetTestClient();

        await client.GetAsync("/beta");

        Assert.Equal(1, resolver.Calls);
    }

    [Fact]
    public async Task Disabled_feature_uses_configured_status_and_title()
    {
        var resolver = new StubResolver("other");
        await using var app = BuildApp(resolver, options =>
        {
            options.DisabledStatusCode = 403;
            options.DisabledTitle = "Beta is invite-only";
        });
        var client = app.GetTestClient();

        var response = await client.GetAsync("/beta");

        Assert.Equal(HttpStatusCode.Forbidden, response.StatusCode);
        Assert.Contains("Beta is invite-only", await response.Content.ReadAsStringAsync());
    }

    [Fact]
    public async Task Feature_evaluation_uses_application_supplied_context_not_platform_state()
    {
        var resolver = new StubResolver("acme");
        await using var app = BuildApp(resolver);

        Assert.Equal(HttpStatusCode.OK, (await app.GetTestClient().GetAsync("/beta")).StatusCode);

        resolver.CurrentTenant = "contoso";
        Assert.Equal(HttpStatusCode.NotFound, (await app.GetTestClient().GetAsync("/beta")).StatusCode);
    }

    [Fact]
    public void Options_validation_rejects_out_of_range_status_code()
    {
        var options = new FeatureManagementOptions { DisabledStatusCode = 9 };
        var errors = options.Validate();
        Assert.Contains(errors, e => e.Contains("HTTP status", StringComparison.OrdinalIgnoreCase));
    }

    [Fact]
    public void Options_validation_rejects_empty_title()
    {
        var options = new FeatureManagementOptions { DisabledTitle = "   " };
        Assert.Contains(options.Validate(), e => e.Contains("title", StringComparison.OrdinalIgnoreCase));
    }

    private static WebApplication BuildApp(IFeatureContextResolver resolver, Action<FeatureManagementOptions>? configure = null)
    {
        var builder = WebApplication.CreateBuilder(new WebApplicationOptions { EnvironmentName = "Testing" });
        builder.WebHost.UseTestServer();
        builder.Services.AddPlatformFeatureManagement(FeatureConfiguration(), configure ?? (_ => { }));
        builder.Services.AddSingleton(resolver);
        var app = builder.Build();
        app.MapGet("/beta", () => Results.Ok("beta-on")).RequireFeature("beta");
        app.StartAsync().GetAwaiter().GetResult();
        return app;
    }

    private static IConfiguration FeatureConfiguration()
    {
        var values = new Dictionary<string, string?>
        {
            ["FeatureManagement:beta:EnabledFor:0:Name"] = "PlatformTenant",
            ["FeatureManagement:beta:EnabledFor:0:Parameters:AllowedTenants:0"] = "acme",
        };
        return new ConfigurationBuilder().AddInMemoryCollection(values).Build();
    }

    private sealed class StubResolver : IFeatureContextResolver
    {
        public string CurrentTenant { get; set; }
        public int Calls { get; private set; }

        public StubResolver(string tenant) => CurrentTenant = tenant;

        public ValueTask<FeatureContext> ResolveAsync(CancellationToken cancellationToken = default)
        {
            Calls++;
            return new ValueTask<FeatureContext>(new FeatureContext(TenantId: CurrentTenant));
        }
    }
}
