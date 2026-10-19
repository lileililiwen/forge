using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Options;
using Platform.ConsumerConformance.Fixtures;
using Platform.Core.Time;
using Platform.Mailing;
using Platform.Mailing.DependencyInjection;

namespace Platform.ConsumerConformance.Tests;

public sealed class MailingRegistrationTests
{
    [Fact]
    public void AddPlatformMailing_registers_options_and_clock_only()
    {
        var services = ConsumerServiceCollectionFactory.CreateServices();
        services.AddPlatformMailing();

        using var provider = ConsumerServiceCollectionFactory.BuildProvider(services);
        var options = provider.GetRequiredService<IOptions<MailingOptions>>().Value;
        Assert.Equal("Mailing", MailingOptions.SectionName);
        Assert.Equal("templates", options.TemplatesPath);
        var clock = provider.GetRequiredService<IClock>();
        Assert.IsType<SystemClock>(clock);
        Assert.Null(provider.GetService<IMailService>());
    }

    [Fact]
    public void AddPlatformMailing_applies_configure_delegate()
    {
        var services = ConsumerServiceCollectionFactory.CreateServices();
        services.AddPlatformMailing(options =>
        {
            options.DefaultFromAddress = "noreply@test.invalid";
            options.MaxAttempts = 7;
        });

        using var provider = ConsumerServiceCollectionFactory.BuildProvider(services);
        var options = provider.GetRequiredService<IOptions<MailingOptions>>().Value;
        Assert.Equal("noreply@test.invalid", options.DefaultFromAddress);
        Assert.Equal(7, options.MaxAttempts);
    }

    [Fact]
    public void AddPlatformMailing_honours_consumer_clock()
    {
        var services = ConsumerServiceCollectionFactory.CreateServices();
        services.AddSingleton<IClock>(new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero)));
        services.AddPlatformMailing();

        using var provider = ConsumerServiceCollectionFactory.BuildProvider(services);
        var clock = provider.GetRequiredService<IClock>();
        Assert.IsType<FixedClock>(clock);
    }
}
