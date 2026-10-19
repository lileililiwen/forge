using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Options;
using Platform.Core.Time;
using Platform.Jobs.DependencyInjection;

namespace Platform.Jobs.Tests;

public class AddPlatformJobsTests
{
    [Fact]
    public void Default_registration_binds_options_with_utc_default()
    {
        var services = new ServiceCollection();

        services.AddPlatformJobs();

        using var provider = services.BuildServiceProvider();
        var options = provider.GetRequiredService<IOptions<BackgroundJobsOptions>>().Value;

        Assert.Equal("UTC", options.DefaultTimeZone);
    }

    [Fact]
    public void Configure_delegate_overrides_defaults()
    {
        var services = new ServiceCollection();

        services.AddPlatformJobs(options => options.DefaultTimeZone = "Europe/Berlin");

        using var provider = services.BuildServiceProvider();
        var options = provider.GetRequiredService<IOptions<BackgroundJobsOptions>>().Value;

        Assert.Equal("Europe/Berlin", options.DefaultTimeZone);
    }

    [Fact]
    public void Registration_adds_clock_when_missing()
    {
        var services = new ServiceCollection();

        services.AddPlatformJobs();

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

        services.AddPlatformJobs();

        using var provider = services.BuildServiceProvider();
        var clock = provider.GetRequiredService<IClock>();

        Assert.Same(preExisting, clock);
    }

    [Fact]
    public void Null_services_throws()
    {
        Assert.Throws<ArgumentNullException>(() => ((IServiceCollection)null!).AddPlatformJobs());
        Assert.Throws<ArgumentNullException>(
            () => ((IServiceCollection)null!).AddPlatformJobs(_ => { }));
        Assert.Throws<ArgumentNullException>(
            () => new ServiceCollection().AddPlatformJobs(configure: null!));
    }

    [Fact]
    public void AddPlatformJobs_returns_same_service_collection()
    {
        var services = new ServiceCollection();

        var result = services.AddPlatformJobs();

        Assert.Same(services, result);
    }
}
