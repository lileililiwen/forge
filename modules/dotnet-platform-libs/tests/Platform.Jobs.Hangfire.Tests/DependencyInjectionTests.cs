using Hangfire;
using Hangfire.InMemory;
using Hangfire.PostgreSql;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Diagnostics.HealthChecks;
using Platform.Core.Time;
using Platform.Jobs;
using Platform.Jobs.Hangfire;
using Platform.Jobs.Hangfire.DependencyInjection;

namespace Platform.Jobs.Hangfire.Tests;

public class DependencyInjectionTests : HangfireTest
{
    [Fact]
    public void Registration_wires_the_platform_seams_and_the_storage()
    {
        var (provider, storage) = HangfireTestHost.Build();

        Assert.IsAssignableFrom<IJobDispatcher>(provider.GetRequiredService<IJobDispatcher>());
        Assert.IsAssignableFrom<IRecurringJobRegistry>(provider.GetRequiredService<IRecurringJobRegistry>());
        Assert.IsAssignableFrom<IHealthCheck>(provider.GetRequiredService<IHealthCheck>());
        Assert.IsType<InMemoryStorage>(storage);
        Assert.IsType<SystemClock>(provider.GetRequiredService<IClock>());
        Assert.NotNull(provider.GetRequiredService<HangfireJobExecutor>());
    }

    [Fact]
    public void Registration_is_idempotent()
    {
        var services = new ServiceCollection();
        services.AddLogging();
        services.AddPlatformHangfireJobs();
        services.AddPlatformHangfireJobs(options => options.WorkerCount = 42);

        using var provider = services.BuildServiceProvider();

        var dispatcher = provider.GetRequiredService<IJobDispatcher>();
        Assert.Same(dispatcher, provider.GetRequiredService<IJobDispatcher>());
        Assert.Single(provider.GetServices<IHostedService>());
    }

    [Fact]
    public void Application_owned_dispatcher_and_registry_win()
    {
        var services = new ServiceCollection();
        services.AddLogging();
        services.AddSingleton<IJobDispatcher>(new RecordingDispatcher());
        services.AddSingleton<IRecurringJobRegistry>(new InMemoryRecurringJobRegistry());

        services.AddPlatformHangfireJobs();

        using var provider = services.BuildServiceProvider();
        Assert.IsType<RecordingDispatcher>(provider.GetRequiredService<IJobDispatcher>());
        Assert.IsType<InMemoryRecurringJobRegistry>(provider.GetRequiredService<IRecurringJobRegistry>());
    }

    [Fact]
    public void Invalid_options_fail_at_registration()
    {
        var services = new ServiceCollection();

        Assert.Throws<ArgumentOutOfRangeException>(
            () => services.AddPlatformHangfireJobs(options => options.WorkerCount = 0));
    }

    [Fact]
    public void PostgreSql_storage_is_selected_through_configuration()
    {
        var services = new ServiceCollection();
        services.AddLogging();
        services.AddPlatformHangfireJobs(options =>
        {
            options.Storage = HangfireStorageKind.PostgreSql;
            options.PostgreSqlConnectionString = "Host=localhost;Database=hangfire;Username=test;Password=test";
        });

        using var provider = services.BuildServiceProvider();

        Assert.IsType<PostgreSqlStorage>(provider.GetRequiredService<JobStorage>());
    }

    [Fact]
    public void Dispatched_jobs_reach_the_storage_through_the_real_pipeline()
    {
        var (provider, storage) = HangfireTestHost.Build();
        var dispatcher = provider.GetRequiredService<IJobDispatcher>();

        dispatcher.EnqueueAsync(JobPayload.Create("cleanup", new Dictionary<string, object?> { ["count"] = 1 }));

        var api = storage.GetMonitoringApi();
        Assert.Equal(1, api.EnqueuedCount("default"));
    }
}
