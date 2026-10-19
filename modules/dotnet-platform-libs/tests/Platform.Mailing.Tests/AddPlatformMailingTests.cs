using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Options;
using Platform.Core.Time;
using Platform.Mailing.DependencyInjection;

namespace Platform.Mailing.Tests;

public class AddPlatformMailingTests
{
    [Fact]
    public void Default_registration_binds_options_with_documented_defaults()
    {
        var services = new ServiceCollection();

        services.AddPlatformMailing();

        using var provider = services.BuildServiceProvider();
        var options = provider.GetRequiredService<IOptions<MailingOptions>>().Value;

        Assert.Equal("noreply@example.invalid", options.DefaultFromAddress);
        Assert.Equal(3, options.MaxAttempts);
    }

    [Fact]
    public void Configure_delegate_overrides_defaults()
    {
        var services = new ServiceCollection();

        services.AddPlatformMailing(options =>
        {
            options.DefaultFromAddress = "team@example.com";
            options.MaxAttempts = 5;
        });

        using var provider = services.BuildServiceProvider();
        var options = provider.GetRequiredService<IOptions<MailingOptions>>().Value;

        Assert.Equal("team@example.com", options.DefaultFromAddress);
        Assert.Equal(5, options.MaxAttempts);
    }

    [Fact]
    public void Registration_adds_clock_when_missing()
    {
        var services = new ServiceCollection();

        services.AddPlatformMailing();

        using var provider = services.BuildServiceProvider();
        var clock = provider.GetRequiredService<IClock>();

        Assert.NotNull(clock);
    }

    [Fact]
    public void Registration_preserves_existing_clock()
    {
        var services = new ServiceCollection();
        var preExisting = new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        services.AddSingleton<IClock>(preExisting);

        services.AddPlatformMailing();

        using var provider = services.BuildServiceProvider();
        var clock = provider.GetRequiredService<IClock>();

        Assert.Same(preExisting, clock);
    }

    [Fact]
    public void Null_services_throws()
    {
        Assert.Throws<ArgumentNullException>(() => ((IServiceCollection)null!).AddPlatformMailing());
        Assert.Throws<ArgumentNullException>(
            () => ((IServiceCollection)null!).AddPlatformMailing(_ => { }));
        Assert.Throws<ArgumentNullException>(
            () => new ServiceCollection().AddPlatformMailing(configure: null!));
    }

    [Fact]
    public void AddPlatformMailing_returns_same_service_collection()
    {
        var services = new ServiceCollection();

        var result = services.AddPlatformMailing();

        Assert.Same(services, result);
    }

    [Fact]
    public void AddPlatformMailing_does_not_register_default_MailService_or_Renderer()
    {
        var services = new ServiceCollection();

        services.AddPlatformMailing();

        using var provider = services.BuildServiceProvider();
        Assert.Null(provider.GetService<IMailService>());
        Assert.Null(provider.GetService<IMailTemplateRenderer<object>>());
    }
}
