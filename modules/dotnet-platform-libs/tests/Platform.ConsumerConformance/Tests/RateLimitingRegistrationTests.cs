using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Options;
using Platform.ConsumerConformance.Fixtures;
using Platform.Core.Time;
using Platform.RateLimiting;
using Platform.RateLimiting.DependencyInjection;

namespace Platform.ConsumerConformance.Tests;

public sealed class RateLimitingRegistrationTests
{
    [Fact]
    public void AddPlatformRateLimiting_registers_default_services_and_options()
    {
        var services = ConsumerServiceCollectionFactory.CreateServices();
        services.AddPlatformRateLimiting();

        using var provider = ConsumerServiceCollectionFactory.BuildProvider(services);
        var options = provider.GetRequiredService<IOptions<RateLimitingOptions>>().Value;
        Assert.Equal("RateLimiting", RateLimitingOptions.SectionName);
        var policies = options.Policies;
        Assert.NotNull(policies);
        var feed = policies.Find("feed");
        Assert.NotNull(feed);
        Assert.Equal(60, feed!.Limit);
        var limiter = provider.GetRequiredService<IRateLimiter>();
        Assert.IsType<InMemoryRateLimiter>(limiter);
        var resolver = provider.GetRequiredService<IRateLimitBypassResolver>();
        Assert.IsType<ConfigurationRateLimitBypassResolver>(resolver);
        var status = provider.GetRequiredService<IRateLimiterBackendStatusProvider>();
        Assert.IsType<InMemoryRateLimiterBackendStatusProvider>(status);
    }

    [Fact]
    public void AddPlatformRateLimiting_honours_consumer_clock()
    {
        var services = ConsumerServiceCollectionFactory.CreateServices();
        services.AddSingleton<IClock>(new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero)));
        services.AddPlatformRateLimiting();

        using var provider = ConsumerServiceCollectionFactory.BuildProvider(services);
        var clock = provider.GetRequiredService<IClock>();
        Assert.IsType<FixedClock>(clock);
    }

    [Fact]
    public async Task AddPlatformRateLimiting_limiter_returns_first_request_decision()
    {
        var services = ConsumerServiceCollectionFactory.CreateServices();
        services.AddPlatformRateLimiting();
        using var provider = ConsumerServiceCollectionFactory.BuildProvider(services);
        var limiter = provider.GetRequiredService<IRateLimiter>();
        var decision = await limiter.CheckAsync(new RateLimitKey("feed", "subject-a"), CancellationToken.None);
        Assert.True(decision.Allowed);
        Assert.Equal(60, decision.Limit);
    }
}
