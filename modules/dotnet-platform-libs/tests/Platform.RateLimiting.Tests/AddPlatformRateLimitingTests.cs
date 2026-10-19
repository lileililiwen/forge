using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Options;
using Platform.Core.Time;
using Platform.RateLimiting.DependencyInjection;

namespace Platform.RateLimiting.Tests;

public class AddPlatformRateLimitingTests
{
    [Fact]
    public void Default_registration_registers_limiter_bypass_resolver_and_status()
    {
        var services = new ServiceCollection();

        services.AddPlatformRateLimiting();

        using var provider = services.BuildServiceProvider();
        var options = provider.GetRequiredService<IOptions<RateLimitingOptions>>().Value;

        Assert.NotNull(provider.GetRequiredService<IRateLimiter>());
        Assert.NotNull(provider.GetRequiredService<IRateLimitBypassResolver>());
        Assert.NotNull(provider.GetRequiredService<IRateLimiterBackendStatusProvider>());
        Assert.IsType<InMemoryRateLimiter>(provider.GetRequiredService<IRateLimiter>());
        Assert.IsType<ConfigurationRateLimitBypassResolver>(provider.GetRequiredService<IRateLimitBypassResolver>());
        Assert.IsType<InMemoryRateLimiterBackendStatusProvider>(provider.GetRequiredService<IRateLimiterBackendStatusProvider>());
        Assert.NotNull(options);
    }

    [Fact]
    public void Default_registration_applies_documented_policy_catalog()
    {
        var services = new ServiceCollection();

        services.AddPlatformRateLimiting();

        using var provider = services.BuildServiceProvider();
        var options = provider.GetRequiredService<IOptions<RateLimitingOptions>>().Value;

        Assert.NotNull(options.Policies.Find("feed"));
        Assert.NotNull(options.Policies.Find("search"));
        Assert.NotNull(options.Policies.Find("uploads"));
        Assert.NotNull(options.Policies.Find("downloads"));
        Assert.NotNull(options.Policies.Find("account-recovery"));
    }

    [Fact]
    public void Configure_delegate_overrides_defaults()
    {
        var services = new ServiceCollection();

        services.AddPlatformRateLimiting(options =>
        {
            options.BypassTokens = new[] { "service-token" };
            options.Policies = new RateLimitPolicies(new[]
            {
                new RateLimitPolicyOptions { Name = "custom", Limit = 1, WindowSeconds = 1 },
            });
        });

        using var provider = services.BuildServiceProvider();
        var options = provider.GetRequiredService<IOptions<RateLimitingOptions>>().Value;

        Assert.Single(options.BypassTokens);
        Assert.Equal("service-token", options.BypassTokens[0]);
        Assert.Null(options.Policies.Find("feed"));
        Assert.NotNull(options.Policies.Find("custom"));
    }

    [Fact]
    public void Consumer_can_register_custom_RateLimitPolicies()
    {
        var services = new ServiceCollection();
        services.AddSingleton(RateLimitPolicies.Default());

        services.AddPlatformRateLimiting();

        using var provider = services.BuildServiceProvider();
        var options = provider.GetRequiredService<IOptions<RateLimitingOptions>>().Value;

        Assert.NotNull(options.Policies);
    }

    [Fact]
    public void Registration_preserves_existing_clock()
    {
        var services = new ServiceCollection();
        var preExisting = new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        services.AddSingleton<IClock>(preExisting);

        services.AddPlatformRateLimiting();

        using var provider = services.BuildServiceProvider();
        Assert.Same(preExisting, provider.GetRequiredService<IClock>());
    }

    [Fact]
    public void Null_services_throws()
    {
        Assert.Throws<ArgumentNullException>(() => ((IServiceCollection)null!).AddPlatformRateLimiting());
        Assert.Throws<ArgumentNullException>(
            () => ((IServiceCollection)null!).AddPlatformRateLimiting(_ => { }));
        Assert.Throws<ArgumentNullException>(
            () => new ServiceCollection().AddPlatformRateLimiting(configure: null!));
    }
}
