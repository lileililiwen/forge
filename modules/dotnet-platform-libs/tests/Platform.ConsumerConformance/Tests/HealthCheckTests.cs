using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.Routing;
using Microsoft.AspNetCore.TestHost;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Diagnostics.HealthChecks;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Options;
using Platform.AspNetCore.DependencyInjection;
using Platform.AspNetCore.HealthChecks;
using Platform.ConsumerConformance.Fixtures;
using Platform.RateLimiting;
using Platform.RateLimiting.DependencyInjection;

namespace Platform.ConsumerConformance.Tests;

public sealed class HealthCheckTests
{
    [Fact]
    public void AddPlatformHealthCheck_registers_a_single_liveness_check()
    {
        var services = ConsumerServiceCollectionFactory.CreateServices();
        services.AddPlatformAspNetCore();
        services.AddPlatformHealthChecks();

        using var provider = ConsumerServiceCollectionFactory.BuildProvider(services);
        var options = provider.GetRequiredService<IOptions<HealthCheckServiceOptions>>().Value;
        var registration = Assert.Single(options.Registrations);
        Assert.Contains("live", registration.Tags);
    }

    [Fact]
    public async Task AddPlatformHealthChecks_returns_healthy_via_test_server()
    {
        var host = ConsumerTestHostFactory.Build(
            services =>
            {
                services.AddPlatformAspNetCore(options => options.HealthCheckPath = "/health");
                services.AddPlatformHealthChecks();
            },
            routes => routes.MapHealthChecks("/health"));
        try
        {
            var client = host.GetTestServer().CreateClient();
            var response = await client.GetAsync("/health");
            response.EnsureSuccessStatusCode();
            var body = await response.Content.ReadAsStringAsync();
            Assert.Equal("Healthy", body.Trim());
        }
        finally
        {
            await host.StopAsync();
        }
    }

    [Fact]
    public void AddPlatformHealthChecks_aggregates_unhealthy_when_a_check_fails()
    {
        var services = ConsumerServiceCollectionFactory.CreateServices();
        services.AddPlatformAspNetCore();
        services.AddPlatformHealthChecks()
            .AddCheck("always-fails", () => HealthCheckResult.Unhealthy("simulated"));

        using var provider = ConsumerServiceCollectionFactory.BuildProvider(services);
        var health = provider.GetRequiredService<HealthCheckService>();
        var report = health.CheckHealthAsync().GetAwaiter().GetResult();
        Assert.Equal(HealthStatus.Unhealthy, report.Status);
        Assert.Contains(report.Entries, entry => entry.Key == "always-fails");
    }

    [Fact]
    public void RateLimitBackendStatusProvider_reports_available_through_DI()
    {
        var services = ConsumerServiceCollectionFactory.CreateServices();
        services.AddPlatformRateLimiting();
        using var provider = ConsumerServiceCollectionFactory.BuildProvider(services);
        var statusProvider = provider.GetRequiredService<IRateLimiterBackendStatusProvider>();
        var status = statusProvider.GetStatus();
        Assert.True(status.Available);
        Assert.Equal("memory", status.Provider);
    }
}
