using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Hosting;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.TestHost;
using Microsoft.Extensions.DependencyInjection;
using Platform.AspNetCore.DependencyInjection;
using Platform.AspNetCore.Errors;
using Platform.AspNetCore.HealthChecks;
using Platform.Core.Results;

namespace Platform.AspNetCore.Tests.Integration;

public class MinimalHostTests
{
    [Fact]
    public async Task Known_validation_failure_returns_problem_details()
    {
        await using var app = BuildApp();
        var client = app.GetTestClient();

        var response = await client.GetAsync("/validation");

        Assert.Equal(StatusCodes.Status400BadRequest, (int)response.StatusCode);
        Assert.Equal("application/problem+json", response.Content.Headers.ContentType?.MediaType);
        var body = await response.Content.ReadAsStringAsync();
        Assert.Contains("\"code\":\"platform.validation\"", body);
        Assert.Contains("name is required", body);
    }

    [Fact]
    public async Task Unknown_failure_returns_sanitized_500()
    {
        await using var app = BuildApp();
        var client = app.GetTestClient();

        var response = await client.GetAsync("/explode");

        Assert.Equal(StatusCodes.Status500InternalServerError, (int)response.StatusCode);
        var body = await response.Content.ReadAsStringAsync();
        Assert.DoesNotContain("boom-detail", body);
        Assert.DoesNotContain("InvalidOperationException", body);
    }

    [Fact]
    public async Task Correlation_id_is_generated_and_returned_when_absent()
    {
        await using var app = BuildApp();
        var client = app.GetTestClient();

        var response = await client.GetAsync("/validation");

        Assert.True(response.Headers.TryGetValues("X-Correlation-Id", out var values));
        var id = Assert.Single(values!);
        Assert.False(string.IsNullOrWhiteSpace(id));
    }

    [Fact]
    public async Task Correlation_id_is_not_taken_from_request_by_default()
    {
        await using var app = BuildApp();
        var client = app.GetTestClient();
        client.DefaultRequestHeaders.Add("X-Correlation-Id", "client-supplied");

        var response = await client.GetAsync("/validation");

        Assert.True(response.Headers.TryGetValues("X-Correlation-Id", out var values));
        var id = Assert.Single(values!);
        Assert.NotEqual("client-supplied", id);
    }

    [Fact]
    public async Task Health_endpoint_returns_healthy()
    {
        await using var app = BuildApp();
        var client = app.GetTestClient();

        var response = await client.GetAsync("/health");

        Assert.Equal(StatusCodes.Status200OK, (int)response.StatusCode);
        var body = await response.Content.ReadAsStringAsync();
        Assert.Equal("Healthy", body.Trim());
    }

    private static WebApplication BuildApp()
    {
        var options = new WebApplicationOptions
        {
            EnvironmentName = "Testing",
        };
        var builder = WebApplication.CreateBuilder(options);
        builder.WebHost.UseTestServer();
        builder.Services.AddPlatformAspNetCore(platformOptions =>
        {
            platformOptions.CorrelationHeader = "X-Correlation-Id";
            platformOptions.HealthCheckPath = "/health";
        });
        builder.Services.AddPlatformHealthChecks();

        var app = builder.Build();
        app.UsePlatformAspNetCore();

        app.MapGet("/validation", () =>
        {
            throw new PlatformProblemException(Error.Validation("name is required"));
        });
        app.MapGet("/explode", () =>
        {
            throw new InvalidOperationException("boom-detail");
        });

        app.MapPlatformEndpoints();

        app.StartAsync().GetAwaiter().GetResult();
        return app;
    }
}
