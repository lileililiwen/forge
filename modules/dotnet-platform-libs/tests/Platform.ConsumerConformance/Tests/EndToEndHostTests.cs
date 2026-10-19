using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.TestHost;
using Microsoft.AspNetCore.Routing;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Platform.AspNetCore.DependencyInjection;
using Platform.AspNetCore.Errors;
using Platform.AspNetCore.HealthChecks;
using Platform.ConsumerConformance.Fixtures;
using Platform.Core.Results;

namespace Platform.ConsumerConformance.Tests;

public sealed class EndToEndHostTests
{
    [Fact]
    public async Task Minimal_host_built_from_packages_returns_problem_details_for_validation_errors()
    {
        var app = ConsumerTestHostFactory.BuildWebApplication(services =>
        {
            services.AddPlatformAspNetCore();
            services.AddPlatformHealthChecks();
        },
        app =>
        {
            app.UsePlatformAspNetCore();
            app.MapGet("/validation", (HttpContext _) => throw new PlatformProblemException(Error.Validation("name is required")));
            app.MapPlatformEndpoints();
        });
        try
        {
            var client = app.GetTestClient();
            var response = await client.GetAsync("/validation");
            Assert.Equal(400, (int)response.StatusCode);
            var body = await response.Content.ReadAsStringAsync();
            Assert.Contains("platform.validation", body);
        }
        finally
        {
            await app.StopAsync();
        }
    }

    [Fact]
    public async Task Minimal_host_returns_sanitized_500_for_unknown_failures()
    {
        var app = ConsumerTestHostFactory.BuildWebApplication(services =>
        {
            services.AddPlatformAspNetCore();
            services.AddPlatformHealthChecks();
        },
        app =>
        {
            app.UsePlatformAspNetCore();
            app.MapGet("/explode", (HttpContext _) => throw new InvalidOperationException("boom-detail"));
            app.MapPlatformEndpoints();
        });
        try
        {
            var client = app.GetTestClient();
            var response = await client.GetAsync("/explode");
            Assert.Equal(500, (int)response.StatusCode);
            var body = await response.Content.ReadAsStringAsync();
            Assert.DoesNotContain("boom-detail", body);
        }
        finally
        {
            await app.StopAsync();
        }
    }

    [Fact]
    public async Task Minimal_host_generates_and_returns_correlation_identifier()
    {
        var app = ConsumerTestHostFactory.BuildWebApplication(services =>
        {
            services.AddPlatformAspNetCore();
        },
        app =>
        {
            app.UsePlatformAspNetCore();
            app.MapGet("/probe", async context => await context.Response.WriteAsync("ok"));
        });
        try
        {
            var client = app.GetTestClient();
            var response = await client.GetAsync("/probe");
            response.EnsureSuccessStatusCode();
            Assert.True(response.Headers.TryGetValues("X-Correlation-Id", out var values));
            var id = Assert.Single(values!);
            Assert.False(string.IsNullOrWhiteSpace(id));
        }
        finally
        {
            await app.StopAsync();
        }
    }

    [Fact]
    public async Task Minimal_host_health_endpoint_reports_healthy()
    {
        var app = ConsumerTestHostFactory.BuildWebApplication(services =>
        {
            services.AddPlatformAspNetCore();
            services.AddPlatformHealthChecks();
        },
        app =>
        {
            app.UsePlatformAspNetCore();
            app.MapPlatformEndpoints();
        });
        try
        {
            var client = app.GetTestClient();
            var response = await client.GetAsync("/health");
            response.EnsureSuccessStatusCode();
            var body = await response.Content.ReadAsStringAsync();
            Assert.Equal("Healthy", body.Trim());
        }
        finally
        {
            await app.StopAsync();
        }
    }
}
