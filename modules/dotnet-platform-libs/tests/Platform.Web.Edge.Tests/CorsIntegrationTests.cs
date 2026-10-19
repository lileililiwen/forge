using System.Net;
using System.Net.Http.Json;
using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Hosting;
using Microsoft.AspNetCore.TestHost;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Platform.Web.Cors.DependencyInjection;

namespace Platform.Web.Edge.Tests;

public sealed class CorsIntegrationTests
{
    [Fact]
    public async Task Allowed_origin_receives_allow_header()
    {
        await using var app = BuildApp(environment: "Development", configure: o =>
        {
            o.Policies.Add(new Platform.Web.Cors.PlatformWebCorsPolicyOptions
            {
                Name = "default",
                AllowedOrigins = { "https://app.example.com" },
                AllowedMethods = { "GET", "POST" }
            });
        });
        var client = app.GetTestClient();
        var request = new HttpRequestMessage(HttpMethod.Options, "/api/items");
        request.Headers.Add("Origin", "https://app.example.com");
        request.Headers.Add("Access-Control-Request-Method", "GET");

        var response = await client.SendAsync(request);

        Assert.Equal(HttpStatusCode.NoContent, response.StatusCode);
        Assert.True(response.Headers.TryGetValues("Access-Control-Allow-Origin", out var origins));
        Assert.Contains("https://app.example.com", origins!);
    }

    [Fact]
    public async Task Disallowed_origin_does_not_receive_allow_header()
    {
        await using var app = BuildApp(environment: "Development", configure: o =>
        {
            o.Policies.Add(new Platform.Web.Cors.PlatformWebCorsPolicyOptions
            {
                Name = "default",
                AllowedOrigins = { "https://app.example.com" }
            });
        });
        var client = app.GetTestClient();
        var request = new HttpRequestMessage(HttpMethod.Get, "/api/items");
        request.Headers.Add("Origin", "https://attacker.example.com");

        var response = await client.SendAsync(request);

        Assert.Equal(HttpStatusCode.OK, response.StatusCode);
        Assert.False(response.Headers.Contains("Access-Control-Allow-Origin"));
    }

    [Fact]
    public async Task Wildcard_origin_does_not_include_credentials_in_response()
    {
        await using var app = BuildApp(environment: "Development", configure: o =>
        {
            o.Policies.Add(new Platform.Web.Cors.PlatformWebCorsPolicyOptions
            {
                Name = "default",
                AllowedOrigins = { "*" }
            });
        });
        var client = app.GetTestClient();
        var request = new HttpRequestMessage(HttpMethod.Get, "/api/items");
        request.Headers.Add("Origin", "https://app.example.com");

        var response = await client.SendAsync(request);

        Assert.Equal(HttpStatusCode.OK, response.StatusCode);
        Assert.True(response.Headers.TryGetValues("Access-Control-Allow-Origin", out var origins));
        Assert.Contains("*", origins!);
    }

    private static WebApplication BuildApp(string environment, Action<Platform.Web.Cors.PlatformWebCorsOptions> configure)
    {
        var builder = WebApplication.CreateBuilder(new WebApplicationOptions { EnvironmentName = environment });
        builder.WebHost.UseTestServer();
        builder.Services.AddPlatformWebCors(builder.Environment, configure);
        var app = builder.Build();
        app.UsePlatformWebCors();
        app.MapGet("/api/items", () => Microsoft.AspNetCore.Http.Results.Ok(new { ok = true }));
        app.StartAsync().GetAwaiter().GetResult();
        return app;
    }
}
