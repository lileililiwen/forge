using Hangfire;
using Hangfire.InMemory;
using Microsoft.Extensions.DependencyInjection;
using Platform.Jobs;
using Platform.Jobs.Hangfire;

namespace Platform.Jobs.Hangfire.Tests;

/// <summary>
/// End-to-end tests that intentionally run in parallel collections to
/// exercise the per-host storage and dispatcher isolation guarantees. Each
/// test owns its own <see cref="HangfireEndToEndHost"/>, so concurrent
/// execution must never leak dispatcher state, jobs, or telemetry across
/// hosts. The collection is independent of the sequential
/// <see cref="HangfireEndToEndReliabilityCollection"/> so the two suites
/// can run side by side.
/// </summary>
[Collection(nameof(HangfireParallelIsolationCollection))]
public class ParallelEndToEndIsolationTests : HangfireTest
{
    private static readonly TimeSpan PollTimeout = TimeSpan.FromSeconds(30);

    [Fact]
    public async Task Two_hosts_built_concurrently_keep_their_job_state_isolated()
    {
        var firstHandler = new RecordingPayloadHandler();
        var secondHandler = new RecordingPayloadHandler();

        var first = HangfireEndToEndHost.CreateBuilder()
            .ConfigureServices(services => services.AddSingleton<IJobPayloadHandler>(firstHandler))
            .Build();
        var second = HangfireEndToEndHost.CreateBuilder()
            .ConfigureServices(services => services.AddSingleton<IJobPayloadHandler>(secondHandler))
            .Build();

        await using (first)
        await using (second)
        {
            await Task.WhenAll(first.StartAsync(), second.StartAsync());

            var firstDispatcher = first.Services.GetRequiredService<IJobDispatcher>();
            var secondDispatcher = second.Services.GetRequiredService<IJobDispatcher>();
            await Task.WhenAll(
                firstDispatcher.EnqueueAsync(JobPayload.Create("first-host")),
                secondDispatcher.EnqueueAsync(JobPayload.Create("second-host")));

            var firstJob = await AwaitOrTimeout(firstHandler.HandledTask);
            var secondJob = await AwaitOrTimeout(secondHandler.HandledTask);

            Assert.Equal("first-host", firstJob.Name);
            Assert.Equal("second-host", secondJob.Name);

            Assert.DoesNotContain(firstHandler.Handled, h => h.Name == "second-host");
            Assert.DoesNotContain(secondHandler.Handled, h => h.Name == "first-host");
        }
    }

    [Fact]
    public async Task Storage_owned_via_the_builder_is_disposed_with_the_host()
    {
        var storage = new InMemoryStorage();
        var handler = new RecordingPayloadHandler();
        var host = HangfireEndToEndHost.CreateBuilder()
            .UseStorage(storage)
            .ConfigureServices(services => services.AddSingleton<IJobPayloadHandler>(handler))
            .Build();
        await host.StartAsync();

        // The host's storage must be the instance the caller supplied.
        Assert.Same(storage, host.Storage);

        var dispatcher = host.Services.GetRequiredService<IJobDispatcher>();
        await dispatcher.EnqueueAsync(JobPayload.Create("owned"));
        var handled = await AwaitOrTimeout(handler.HandledTask);
        Assert.Equal("owned", handled.Name);

        await host.DisposeAsync();
        Assert.True(host.IsDisposed);
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
}

/// <summary>
/// Collection that runs the parallel-isolation tests inside their own
/// collection so the sequential reliability collection can keep its
/// single-threaded ordering while the parallel tests verify cross-host
/// isolation.
/// </summary>
[CollectionDefinition(nameof(HangfireParallelIsolationCollection), DisableParallelization = false)]
public sealed class HangfireParallelIsolationCollection { }
