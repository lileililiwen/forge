using Hangfire;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Options;
using Platform.Jobs;
using Platform.Jobs.Hangfire;
using Platform.Jobs.Hangfire.DependencyInjection;

namespace Platform.Jobs.Hangfire.Tests;

public class EndToEndTests : HangfireTest
{
    private static readonly TimeSpan PollTimeout = TimeSpan.FromSeconds(30);

    [Fact]
    public async Task Dispatched_jobs_execute_inside_the_restored_context()
    {
        var bridge = new RecordingJobContextBridge();
        var handler = new RecordingPayloadHandler();
        await using var host = HangfireEndToEndHost.CreateBuilder()
            .ConfigureServices(services =>
            {
                services.AddSingleton<IJobExecutionContext>(bridge);
                services.AddSingleton<IJobPayloadHandler>(handler);
            })
            .Build();
        await host.StartAsync();

        RecordingJobContextBridge.SetAmbient("tenant-1", "subject-1");
        try
        {
            var dispatcher = host.Services.GetRequiredService<IJobDispatcher>();
            await dispatcher.EnqueueAsync(JobPayload.Create(
                "cleanup",
                new Dictionary<string, object?> { ["tenant"] = "tenant-1" }));
        }
        finally
        {
            RecordingJobContextBridge.SetAmbient(null, null);
        }

        var handled = await AwaitOrTimeout(handler.HandledTask);
        Assert.Equal("cleanup", handled.Name);
        Assert.Equal("tenant-1", handled.Arguments!["tenant"]);

        var restored = Assert.Single(bridge.Restored);
        Assert.Equal("tenant-1", restored.TenantId);
        Assert.Equal("subject-1", restored.SubjectId);

        await AwaitOrTimeout(WaitForDisposal(bridge));
    }

    [Fact]
    public async Task Dispatched_jobs_run_context_free_without_an_ambient_context()
    {
        var bridge = new RecordingJobContextBridge();
        var handler = new RecordingPayloadHandler();
        await using var host = HangfireEndToEndHost.CreateBuilder()
            .ConfigureServices(services =>
            {
                services.AddSingleton<IJobExecutionContext>(bridge);
                services.AddSingleton<IJobPayloadHandler>(handler);
            })
            .Build();
        await host.StartAsync();

        RecordingJobContextBridge.SetAmbient(null, null);
        var dispatcher = host.Services.GetRequiredService<IJobDispatcher>();
        await dispatcher.EnqueueAsync(JobPayload.Create("cleanup"));

        await AwaitOrTimeout(handler.HandledTask);

        Assert.Empty(bridge.Restored);
    }

    [Fact]
    public async Task Recurring_jobs_execute_their_registered_handler()
    {
        var handler = new RecordingRecurringHandler();
        await using var host = HangfireEndToEndHost.CreateBuilder()
            .ConfigureServices(services => services.AddSingleton<IRecurringJobHandler>(handler))
            .Build();
        await host.StartAsync();

        var registry = host.Services.GetRequiredService<IRecurringJobRegistry>();
        registry.Register(new RecurringJobDescriptor(
            "e2e-recurring",
            "* * * * *",
            typeof(RecordingRecurringHandler)));

        var manager = host.Services.GetRequiredService<IRecurringJobManager>();
        manager.Trigger("e2e-recurring");

        await AwaitOrTimeout(WaitForExecutions(handler));
    }

    [Fact]
    public async Task Server_options_are_applied_from_the_validated_configuration()
    {
        await using var host = HangfireEndToEndHost.CreateBuilder()
            .ConfigureOptions(options =>
            {
                options.WorkerCount = 2;
                options.Queues = new[] { "default", "email" };
            })
            .Build();
        await host.StartAsync();

        var options = host.Services.GetRequiredService<IOptions<HangfireJobsOptions>>().Value;
        Assert.Equal(2, options.WorkerCount);
        Assert.Equal(new[] { "default", "email" }, options.Queues);
    }

    private static async Task<T> AwaitOrTimeout<T>(Task<T> task)
    {
        var completed = await Task.WhenAny(task, Task.Delay(PollTimeout));
        if (completed != task)
        {
            throw new TimeoutException($"The job did not complete within {PollTimeout.TotalSeconds} seconds.");
        }

        return await task;
    }

    private static async Task<bool> WaitForDisposal(RecordingJobContextBridge bridge)
    {
        while (bridge.Disposed.Count == 0)
        {
            await Task.Delay(50);
        }

        return true;
    }

    private static async Task<bool> WaitForExecutions(RecordingRecurringHandler handler)
    {
        while (handler.Executions == 0)
        {
            await Task.Delay(50);
        }

        return true;
    }
}
