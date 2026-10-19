using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Options;
using Platform.ConsumerConformance.Fixtures;
using Platform.Core.Time;
using Platform.Jobs;
using Platform.Jobs.DependencyInjection;

namespace Platform.ConsumerConformance.Tests;

public sealed class JobsRegistrationTests
{
    [Fact]
    public void AddPlatformJobs_registers_options_and_clock_only()
    {
        var services = ConsumerServiceCollectionFactory.CreateServices();
        services.AddPlatformJobs();

        using var provider = ConsumerServiceCollectionFactory.BuildProvider(services);
        var options = provider.GetRequiredService<IOptions<BackgroundJobsOptions>>().Value;
        Assert.Equal("BackgroundJobs", BackgroundJobsOptions.SectionName);
        Assert.Equal("UTC", options.DefaultTimeZone);
        var clock = provider.GetRequiredService<IClock>();
        Assert.IsType<SystemClock>(clock);
        Assert.Null(provider.GetService<IJobDispatcher>());
        Assert.Null(provider.GetService<IRecurringJobRegistry>());
        Assert.Null(provider.GetService<IJobTelemetry>());
    }

    [Fact]
    public void AddPlatformJobs_applies_configure_delegate()
    {
        var services = ConsumerServiceCollectionFactory.CreateServices();
        services.AddPlatformJobs(options => options.DefaultTimeZone = "Europe/Berlin");

        using var provider = ConsumerServiceCollectionFactory.BuildProvider(services);
        var options = provider.GetRequiredService<IOptions<BackgroundJobsOptions>>().Value;
        Assert.Equal("Europe/Berlin", options.DefaultTimeZone);
    }

    [Fact]
    public void AddPlatformJobs_honours_consumer_clock()
    {
        var services = ConsumerServiceCollectionFactory.CreateServices();
        services.AddSingleton<IClock>(new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero)));
        services.AddPlatformJobs();

        using var provider = ConsumerServiceCollectionFactory.BuildProvider(services);
        var clock = provider.GetRequiredService<IClock>();
        Assert.IsType<FixedClock>(clock);
    }
}
