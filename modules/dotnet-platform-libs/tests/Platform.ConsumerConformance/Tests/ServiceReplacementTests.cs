using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Platform.AspNetCore.DependencyInjection;
using Platform.Billing.Contracts.Providers;
using Platform.ConsumerConformance.Fixtures;
using Platform.Core.Time;
using Platform.Jobs;
using Platform.Jobs.DependencyInjection;
using Platform.Mailing;
using Platform.Mailing.DependencyInjection;
using Platform.RateLimiting;
using Platform.RateLimiting.DependencyInjection;

namespace Platform.ConsumerConformance.Tests;

public sealed class ServiceReplacementTests
{
    [Fact]
    public void Consumer_clock_is_preserved_when_AddPlatformRateLimiting_is_called()
    {
        var consumerClock = new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        var services = ConsumerServiceCollectionFactory.CreateServices();
        services.AddSingleton<IClock>(consumerClock);
        services.AddPlatformRateLimiting();

        using var provider = ConsumerServiceCollectionFactory.BuildProvider(services);
        var resolved = provider.GetRequiredService<IClock>();
        Assert.Same(consumerClock, resolved);
    }

    [Fact]
    public void Consumer_clock_is_preserved_when_AddPlatformJobs_is_called()
    {
        var consumerClock = new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        var services = ConsumerServiceCollectionFactory.CreateServices();
        services.AddSingleton<IClock>(consumerClock);
        services.AddPlatformJobs();

        using var provider = ConsumerServiceCollectionFactory.BuildProvider(services);
        var resolved = provider.GetRequiredService<IClock>();
        Assert.Same(consumerClock, resolved);
    }

    [Fact]
    public void Consumer_clock_is_preserved_when_AddPlatformMailing_is_called()
    {
        var consumerClock = new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        var services = ConsumerServiceCollectionFactory.CreateServices();
        services.AddSingleton<IClock>(consumerClock);
        services.AddPlatformMailing();

        using var provider = ConsumerServiceCollectionFactory.BuildProvider(services);
        var resolved = provider.GetRequiredService<IClock>();
        Assert.Same(consumerClock, resolved);
    }

    [Fact]
    public void Consumer_can_replace_the_default_rate_limiter_after_AddPlatformRateLimiting()
    {
        var services = ConsumerServiceCollectionFactory.CreateServices();
        services.AddPlatformRateLimiting();
        services.AddSingleton<IRateLimiter, StubRateLimiter>();

        using var provider = ConsumerServiceCollectionFactory.BuildProvider(services);
        var limiter = provider.GetRequiredService<IRateLimiter>();
        Assert.IsType<StubRateLimiter>(limiter);
    }

    [Fact]
    public void Consumer_can_register_mail_service_when_AddPlatformMailing_is_called()
    {
        var services = ConsumerServiceCollectionFactory.CreateServices();
        services.AddPlatformMailing();
        services.AddSingleton<IMailService, StubMailService>();

        using var provider = ConsumerServiceCollectionFactory.BuildProvider(services);
        var mail = provider.GetRequiredService<IMailService>();
        Assert.IsType<StubMailService>(mail);
    }

    [Fact]
    public void Consumer_can_register_job_dispatcher_when_AddPlatformJobs_is_called()
    {
        var services = ConsumerServiceCollectionFactory.CreateServices();
        services.AddPlatformJobs();
        services.AddSingleton<IJobDispatcher, StubJobDispatcher>();

        using var provider = ConsumerServiceCollectionFactory.BuildProvider(services);
        var dispatcher = provider.GetRequiredService<IJobDispatcher>();
        Assert.IsType<StubJobDispatcher>(dispatcher);
    }

    [Fact]
    public void Consumer_options_configuration_takes_precedence_over_default_values()
    {
        var services = ConsumerServiceCollectionFactory.CreateServices();
        services.AddPlatformRateLimiting(options => options.BypassTokens = new[] { "CONSUMER_TOKEN" });
        using var provider = ConsumerServiceCollectionFactory.BuildProvider(services);
        var options = provider.GetRequiredService<Microsoft.Extensions.Options.IOptions<RateLimitingOptions>>().Value;
        Assert.Contains("CONSUMER_TOKEN", options.BypassTokens);
    }

    private sealed class StubRateLimiter : IRateLimiter
    {
        public Task<RateLimitDecision> CheckAsync(RateLimitKey key, CancellationToken cancellationToken = default) =>
            Task.FromResult(new RateLimitDecision(true, 1, 1, 0));
    }

    private sealed class StubMailService : IMailService
    {
        public Task<MailSendResult> SendAsync(MailMessage message, CancellationToken cancellationToken = default) =>
            Task.FromResult(new MailSendResult(MailSendOutcome.Sent, "stub-id"));
    }

    private sealed class StubJobDispatcher : IJobDispatcher
    {
        public Task EnqueueAsync(JobPayload payload, CancellationToken cancellationToken = default) => Task.CompletedTask;
    }
}
