using System.Net;
using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Hosting;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.TestHost;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Diagnostics.HealthChecks;
using Platform.Web.DependencyInjection;

namespace Platform.Web.Tests;

public sealed class RuntimeIntegrationTests
{
    [Fact]
    public async Task Live_and_ready_are_healthy_by_default()
    {
        await using var app = BuildApp();
        var client = app.GetTestClient();

        var live = await client.GetAsync("/live");
        var ready = await client.GetAsync("/ready");

        Assert.Equal(HttpStatusCode.OK, live.StatusCode);
        Assert.Contains("\"status\":\"live\"", await live.Content.ReadAsStringAsync());
        Assert.Equal(HttpStatusCode.OK, ready.StatusCode);
        Assert.Contains("\"status\":\"ready\"", await ready.Content.ReadAsStringAsync());
    }

    [Fact]
    public async Task Failed_dependency_makes_ready_unavailable_but_live_remains_healthy()
    {
        await using var app = BuildApp(services: services => services.AddHealthChecks().AddCheck("database", () => HealthCheckResult.Unhealthy("offline")));
        var client = app.GetTestClient();

        var live = await client.GetAsync("/live");
        var ready = await client.GetAsync("/ready");

        Assert.Equal(HttpStatusCode.OK, live.StatusCode);
        Assert.Equal(HttpStatusCode.ServiceUnavailable, ready.StatusCode);
        Assert.Contains("not_ready", await ready.Content.ReadAsStringAsync());
    }

    [Fact]
    public async Task Runtime_emits_security_headers_and_rejects_oversized_body()
    {
        await using var app = BuildApp(options => options.MaxRequestBodyBytes = 4);
        var client = app.GetTestClient();

        using var content = new StringContent("12345");
        var response = await client.PostAsync("/echo", content);

        Assert.Equal(HttpStatusCode.RequestEntityTooLarge, response.StatusCode);
        Assert.Equal("nosniff", response.Headers.GetValues("X-Content-Type-Options").Single());
        Assert.Equal("DENY", response.Headers.GetValues("X-Frame-Options").Single());
    }

    [Fact]
    public async Task Invalid_correlation_header_is_replaced_and_unknown_errors_are_safe()
    {
        await using var app = BuildApp(options =>
        {
            options.MaxCorrelationIdLength = 16;
            options.AcceptIncomingCorrelationHeader = true;
        });
        var client = app.GetTestClient();
        client.DefaultRequestHeaders.Add("X-Correlation-Id", new string('x', 17));

        var response = await client.GetAsync("/explode");

        Assert.Equal(HttpStatusCode.InternalServerError, response.StatusCode);
        var correlation = response.Headers.GetValues("X-Correlation-Id").Single();
        Assert.NotEqual(new string('x', 17), correlation);
        Assert.DoesNotContain("secret-stack", await response.Content.ReadAsStringAsync());
    }

    private static WebApplication BuildApp(Action<PlatformWebOptions>? configure = null, Action<IServiceCollection>? services = null)
    {
        var builder = WebApplication.CreateBuilder(new WebApplicationOptions { EnvironmentName = "Testing" });
        builder.WebHost.UseTestServer();
        builder.Services.AddPlatformWeb(configure ?? (_ => { }));
        services?.Invoke(builder.Services);
        var app = builder.Build();
        app.UsePlatformWeb();
        app.MapPost("/echo", async (HttpRequest request) => await request.Body.CopyToAsync(Stream.Null));
        app.MapGet("/explode", () => { throw new InvalidOperationException("secret-stack"); });
        app.MapPlatformRuntimeEndpoints();
        app.StartAsync().GetAwaiter().GetResult();
        return app;
    }
}
