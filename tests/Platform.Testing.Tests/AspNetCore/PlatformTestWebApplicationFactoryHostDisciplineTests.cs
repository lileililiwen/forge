using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Platform.Testing.AspNetCore;
using Platform.Testing.TestApplication;

namespace Platform.Testing.Tests.AspNetCore;

/// <summary>
/// Coverage for the one-host-per-instance discipline and for expressing a
/// configuration permutation without a derived class per scenario.
/// <para>
/// The discipline was adopted after measuring the workspace: 79
/// <c>protected override void ConfigureWebHost</c> declarations (69 files) and 118
/// <c>WithWebHostBuilder(...)</c> call sites (106 files across 17 repositories).
/// Every <c>WithWebHostBuilder</c> call derives a new factory, so each of those
/// sites also pays for another host build.
/// </para>
/// </summary>
public sealed class PlatformTestWebApplicationFactoryHostDisciplineTests
{
    [Fact]
    public async Task Repeated_access_reuses_one_host()
    {
        await using var factory = new PlatformTestWebApplicationFactory<PlatformTestApplicationEntryPoint>();

        var services = factory.Services;
        var server = factory.Server;

        // Second access to the same factory must not build a second host.
        Assert.Same(services, factory.Services);
        Assert.Same(server, factory.Server);
    }

    [Fact]
    public async Task Configured_callbacks_run_once_against_a_single_host()
    {
        var runs = 0;
        await using var factory = new PlatformTestWebApplicationFactory<PlatformTestApplicationEntryPoint>()
            .ConfigureWebHost(_ => runs++)
            .WithConfiguration("Platform:Counted", "1");

        _ = factory.Services;
        _ = factory.Services;
        using var client = factory.CreateClient();

        Assert.Equal(1, runs);
    }

    [Fact]
    public async Task Configuring_the_factory_never_derives_a_second_factory()
    {
        await using var factory = new PlatformTestWebApplicationFactory<PlatformTestApplicationEntryPoint>()
            .WithConfiguration("Platform:Key", "value");

        // The inherited WithWebHostBuilder is the shape that would derive a second
        // factory. The fluent surface on this type keeps Factories empty instead.
        Assert.Empty(factory.Factories);
    }

    // Configuration permutations: one factory type, configured per scenario.

    [Fact]
    public async Task One_factory_type_covers_several_scenarios()
    {
        await using var defaultScenario = new PlatformTestWebApplicationFactory<PlatformTestApplicationEntryPoint>()
            .WithConfiguration("Scenario", "alpha");

        await using var strictScenario = new PlatformTestWebApplicationFactory<PlatformTestApplicationEntryPoint>()
            .WithConfiguration("Scenario", "beta")
            .WithConfiguration("Strict", "true");

        using var defaultClient = defaultScenario.CreateClient();
        using var strictClient = strictScenario.CreateClient();

        Assert.Equal("alpha", await defaultClient.GetStringAsync("/configuration/Scenario"));
        Assert.Equal("beta", await strictClient.GetStringAsync("/configuration/Scenario"));

        // The strict scenario's extra key is not observed by the other factory.
        Assert.Equal("true", await strictClient.GetStringAsync("/configuration/Strict"));
        Assert.Equal("<missing>", await defaultClient.GetStringAsync("/configuration/Strict"));
    }

    [Fact]
    public async Task Two_differently_configured_factories_stay_isolated()
    {
        await using var first = new PlatformTestWebApplicationFactory<PlatformTestApplicationEntryPoint>()
            .WithConfiguration("Platform:Shared", "first");
        await using var second = new PlatformTestWebApplicationFactory<PlatformTestApplicationEntryPoint>()
            .WithConfiguration("Platform:Shared", "second");

        using var firstClient = first.CreateClient();
        using var secondClient = second.CreateClient();

        Assert.Equal("first", await firstClient.GetStringAsync("/configuration/Platform:Shared"));
        Assert.Equal("second", await secondClient.GetStringAsync("/configuration/Platform:Shared"));
    }

    [Fact]
    public async Task Service_state_registered_in_one_factory_is_not_observed_by_another()
    {
        await using var first = new PlatformTestWebApplicationFactory<PlatformTestApplicationEntryPoint>()
            .ReplaceService<IGreetingSender, ScenarioGreetingSender>();
        await using var second = new PlatformTestWebApplicationFactory<PlatformTestApplicationEntryPoint>();

        using var firstClient = first.CreateClient();
        using var secondClient = second.CreateClient();

        Assert.Equal("scenario", await firstClient.GetStringAsync("/greeting"));
        // The application registered IGreetingSender twice, so without a
        // replacement the last registration is the effective one — and it is the
        // application's own, not the other factory's fake.
        Assert.Equal("duplicate", await secondClient.GetStringAsync("/greeting"));
    }

    [Fact]
    public async Task Hosted_services_can_be_removed_through_the_configuration_seam()
    {
        await using var factory = new PlatformTestWebApplicationFactory<PlatformTestApplicationEntryPoint>()
            .ConfigureTestServices((services, _) =>
            {
                foreach (var descriptor in services
                    .Where(d => d.ServiceType == typeof(IHostedService))
                    .ToArray())
                {
                    services.Remove(descriptor);
                }
            });

        var probes = factory.Services.GetServices<IHostedService>().OfType<StopProbeHostedService>().ToArray();

        Assert.Empty(probes);
    }

    [Fact]
    public async Task The_application_started_its_own_hosted_service_when_not_removed()
    {
        await using var factory = new PlatformTestWebApplicationFactory<PlatformTestApplicationEntryPoint>();

        var probe = Assert.Single(factory.Services.GetServices<IHostedService>().OfType<StopProbeHostedService>());

        Assert.Equal(1, probe.StartCount);
    }

    [Fact]
    public async Task Test_services_callback_configuration_sources_are_honoured_by_the_application()
    {
        await using var factory = new PlatformTestWebApplicationFactory<PlatformTestApplicationEntryPoint>()
            .ConfigureTestServices((_, configuration) =>
                configuration.AddInMemoryCollection(new Dictionary<string, string?>
                {
                    ["Platform:FromCallback"] = "callback",
                }));

        Assert.Equal("callback", factory.Services.GetRequiredService<IConfiguration>()["Platform:FromCallback"]);
    }

    [Fact]
    public async Task A_derived_factory_over_ConfigureWebHost_still_builds_one_host()
    {
        // The inherited subclass pattern remains valid: a consumer that already
        // owns a factory subclass can keep it and still get exactly one host.
        await using var factory = new DerivedScenarioFactory();

        Assert.Equal("derived", factory.Services.GetRequiredService<IConfiguration>()["Platform:Derived"]);
        Assert.Empty(factory.Factories);
    }

    private sealed class ScenarioGreetingSender : IGreetingSender
    {
        public string LastGreeting => "scenario";
    }

    private sealed class DerivedScenarioFactory : PlatformTestWebApplicationFactory<PlatformTestApplicationEntryPoint>
    {
        protected override void ConfigureWebHost(Microsoft.AspNetCore.Hosting.IWebHostBuilder builder)
        {
            base.ConfigureWebHost(builder);
            builder.UseSetting("Platform:Derived", "derived");
        }
    }
}
