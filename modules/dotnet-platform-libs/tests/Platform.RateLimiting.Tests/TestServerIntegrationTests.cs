using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Hosting;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.Mvc;
using Microsoft.AspNetCore.TestHost;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Options;
using Platform.RateLimiting.DependencyInjection;

namespace Platform.RateLimiting.Tests;

public class TestServerIntegrationTests
{
    [Fact]
    public async Task Host_with_AddPlatformRateLimiting_resolves_default_policy_catalog()
    {
        await using var app = BuildApp();
        var client = app.GetTestClient();

        var response = await client.GetAsync("/probe");

        Assert.Equal(StatusCodes.Status200OK, (int)response.StatusCode);
        var body = await response.Content.ReadAsStringAsync();
        Assert.Contains("\"policies\":[", body);
        Assert.Contains("\"name\":\"feed\"", body);
        Assert.Contains("\"name\":\"search\"", body);
        Assert.Contains("\"name\":\"uploads\"", body);
        Assert.Contains("\"name\":\"downloads\"", body);
        Assert.Contains("\"name\":\"account-recovery\"", body);
    }

    [Fact]
    public async Task Host_with_configured_bypass_tokens_overrides_defaults()
    {
        await using var app = BuildApp(extraConfiguration: new Dictionary<string, string?>
        {
            ["RateLimiting:BypassTokens:0"] = "service-token",
        });
        var client = app.GetTestClient();

        var response = await client.GetAsync("/probe");

        Assert.Equal(StatusCodes.Status200OK, (int)response.StatusCode);
        var body = await response.Content.ReadAsStringAsync();
        Assert.Contains("\"bypassTokens\":[\"service-token\"]", body);
    }

    [Fact]
    public async Task Host_limiter_check_returns_decision_through_di()
    {
        await using var app = BuildApp();
        var client = app.GetTestClient();

        var response = await client.GetAsync("/check?policy=feed&subject=subject-1");

        Assert.Equal(StatusCodes.Status200OK, (int)response.StatusCode);
        var body = await response.Content.ReadAsStringAsync();
        Assert.Contains("\"allowed\":true", body);
        Assert.Contains("\"limit\":60", body);
    }

    [Fact]
    public async Task Host_status_provider_reports_in_memory_backend()
    {
        await using var app = BuildApp();
        var client = app.GetTestClient();

        var response = await client.GetAsync("/status");

        Assert.Equal(StatusCodes.Status200OK, (int)response.StatusCode);
        var body = await response.Content.ReadAsStringAsync();
        Assert.Contains("\"provider\":\"memory\"", body);
        Assert.Contains("\"available\":true", body);
    }

    private static WebApplication BuildApp(
        IReadOnlyDictionary<string, string?>? extraConfiguration = null)
    {
        var options = new WebApplicationOptions
        {
            EnvironmentName = "Testing",
        };
        var builder = WebApplication.CreateBuilder(options);
        builder.WebHost.UseTestServer();
        if (extraConfiguration is not null)
        {
            builder.Configuration.AddInMemoryCollection(extraConfiguration);
        }

        builder.Services.AddPlatformRateLimiting(rateLimitingOptions =>
            builder.Configuration
                .GetSection(RateLimitingOptions.SectionName)
                .Bind(rateLimitingOptions));

        var app = builder.Build();
        app.MapGet("/probe", (IOptions<RateLimitingOptions> options) => new
        {
            policies = options.Value.Policies.Entries.Select(e => new { name = e.Name, limit = e.Limit, windowSeconds = e.WindowSeconds }),
            bypassTokens = options.Value.BypassTokens,
        });
        app.MapGet("/check", async (
            string policy,
            string subject,
            [FromServices] IRateLimiter limiter) =>
        {
            var decision = await limiter.CheckAsync(new RateLimitKey(policy, subject));
            return Results.Ok(new
            {
                allowed = decision.Allowed,
                limit = decision.Limit,
                remaining = decision.Remaining,
                retryAfterSeconds = decision.RetryAfterSeconds,
            });
        });
        app.MapGet("/status", ([FromServices] IRateLimiterBackendStatusProvider status) =>
        {
            var snapshot = status.GetStatus();
            return Results.Ok(new
            {
                provider = snapshot.Provider,
                available = snapshot.Available,
            });
        });

        app.StartAsync().GetAwaiter().GetResult();
        return app;
    }
}
