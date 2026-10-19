using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.Routing;
using Microsoft.AspNetCore.TestHost;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Diagnostics.HealthChecks;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Options;
using Platform.AspNetCore;
using Platform.AspNetCore.DependencyInjection;
using Platform.AspNetCore.HealthChecks;
using Platform.ConsumerConformance.Fixtures;
using Platform.Core.Time;

namespace Platform.ConsumerConformance.Tests;

public sealed class AspNetCoreRegistrationTests
{
    [Fact]
    public void AddPlatformAspNetCore_registers_options_clock_and_problem_details_mapper()
    {
        var services = ConsumerServiceCollectionFactory.CreateServices();
        services.AddPlatformAspNetCore();

        using var provider = ConsumerServiceCollectionFactory.BuildProvider(services);
        var optionsAccessor = provider.GetRequiredService<IOptions<PlatformAspNetCoreOptions>>();
        Assert.Equal("X-Correlation-Id", optionsAccessor.Value.CorrelationHeader);
        Assert.Equal("/health", optionsAccessor.Value.HealthCheckPath);
        var clock = provider.GetRequiredService<IClock>();
        Assert.IsType<SystemClock>(clock);
    }

    [Fact]
    public void AddPlatformAspNetCore_uses_consumer_clock_when_already_registered()
    {
        var services = ConsumerServiceCollectionFactory.CreateServices();
        services.AddSingleton<IClock>(new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero)));
        services.AddPlatformAspNetCore();

        using var provider = ConsumerServiceCollectionFactory.BuildProvider(services);
        var clock = provider.GetRequiredService<IClock>();
        Assert.IsType<FixedClock>(clock);
    }

    [Fact]
    public async Task AddPlatformHealthChecks_registers_a_liveness_health_check()
    {
        var host = ConsumerTestHostFactory.Build(
            services =>
            {
                services.AddPlatformAspNetCore();
                services.AddPlatformHealthChecks();
            },
            routes => routes.MapHealthChecks("/health"));
        try
        {
            var server = host.GetTestServer();
            var response = await server.CreateClient().GetAsync("/health");
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
    public void AddPlatformHealthChecks_returns_the_builder_for_further_configuration()
    {
        var services = ConsumerServiceCollectionFactory.CreateServices();
        services.AddPlatformAspNetCore();
        var builder = services.AddPlatformHealthChecks();
        Assert.NotNull(builder);
        using var provider = ConsumerServiceCollectionFactory.BuildProvider(services);
        var options = provider.GetRequiredService<IOptions<HealthCheckServiceOptions>>().Value;
        var registration = Assert.Single(options.Registrations);
        Assert.Contains("live", registration.Tags);
    }
}
