using Hangfire;
using Hangfire.Common;
using Hangfire.InMemory;
using Hangfire.States;
using Hangfire.Storage;
using Microsoft.Extensions.DependencyInjection;
using Platform.Core.Results;
using Platform.Core.Time;
using Platform.Jobs;
using Platform.Jobs.Hangfire;

namespace Platform.Jobs.Hangfire.Tests;

/// <summary>
/// Reliability regression tests for the Hangfire end-to-end host. Each test
/// creates, starts, and disposes an isolated <see cref="HangfireEndToEndHost"/>
/// with its own in-memory storage and service provider so failures surface
/// deterministically. The collection disables cross-class parallel execution
/// for the lifetime-sensitive tests; the parallel-host collection exercises
/// isolation with isolated hosts explicitly.
/// </summary>
[Collection(nameof(HangfireEndToEndReliabilityCollection))]
public class EndToEndReliabilityTests : HangfireTest
{
    private static readonly TimeSpan PollTimeout = TimeSpan.FromSeconds(30);

    [Fact]
    public async Task Sequential_dispatches_use_independent_handler_state()
    {
        var handler = new CountingPayloadHandler();
        await using var host = HangfireEndToEndHost.CreateBuilder()
            .ConfigureServices(services => services.AddSingleton<IJobPayloadHandler>(handler))
            .ConfigureOptions(options => options.WorkerCount = 1)
            .Build();
        await host.StartAsync();

        var dispatcher = host.Services.GetRequiredService<IJobDispatcher>();
        var completions = Enumerable.Range(0, 5)
            .Select(index => handler.NextAsync(index))
            .ToArray();
        for (var i = 0; i < 5; i++)
        {
            await dispatcher.EnqueueAsync(JobPayload.Create(
                "sequential",
                new Dictionary<string, object?> { ["index"] = (long)i }));
        }

        for (var i = 0; i < 5; i++)
        {
            var handled = await AwaitOrTimeout(completions[i]);
            Assert.Equal("sequential", handled.Name);
            Assert.Equal((long)i, handled.Arguments!["index"]);
        }

        Assert.Equal(5, handler.Handled.Count);
    }

    [Fact]
    public async Task Failed_job_lifecycle_records_redacted_telemetry_and_does_not_stop_the_worker()
    {
        var telemetry = new RecordingJobTelemetry();
        var handler = new RecordingPayloadHandler();
        var goodHandler = new RecordingPayloadHandler();
        var registry = new TypeSwitchingPayloadHandlerRegistry(
            ("fails-once", handler),
            ("keeps-running", goodHandler));

        await using var host = HangfireEndToEndHost.CreateBuilder()
            .ConfigureServices(services =>
            {
                services.AddSingleton<IJobPayloadHandler>(registry);
                services.AddSingleton<IJobTelemetry>(telemetry);
            })
            .Build();
        await host.StartAsync();

        handler.Failure = new InvalidOperationException("secret payload");
        var dispatcher = host.Services.GetRequiredService<IJobDispatcher>();

        await dispatcher.EnqueueAsync(JobPayload.Create("fails-once"));
        var failed = await AwaitOrTimeout(handler.FailedTask);

        handler.Failure = null;
        var goodCompletion = goodHandler.HandledTask;
        await dispatcher.EnqueueAsync(JobPayload.Create("keeps-running"));
        await AwaitOrTimeout(goodCompletion);

        Assert.Equal("fails-once", failed.Name);
        var failure = Assert.Single(telemetry.Failed);
        Assert.Equal("fails-once", failure.Name);
        Assert.Equal(HangfireJobExecutor.ExecutionFailedCode, failure.Error.Code);
        Assert.Equal("InvalidOperationException", failure.Error.Metadata!["exceptionType"]);
        Assert.DoesNotContain("secret", failure.Error.Message, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public async Task Cancellation_inside_a_handler_is_propagated_and_observed()
    {
        var handler = new GatedPayloadHandler();
        await using var host = HangfireEndToEndHost.CreateBuilder()
            .ConfigureServices(services => services.AddSingleton<IJobPayloadHandler>(handler))
            .Build();
        await host.StartAsync();

        var dispatcher = host.Services.GetRequiredService<IJobDispatcher>();
        await dispatcher.EnqueueAsync(JobPayload.Create("cancel-me"));

        // Wait for the worker to pick up the job and the handler to be invoked.
        var entered = await AwaitOrTimeout(handler.EntryTask);
        Assert.Same(handler, entered);

        // Cancel the handler and verify the cancellation is observed.
        handler.Cancel();
        var failure = await AwaitOrTimeout(handler.FailureTask);
        Assert.IsAssignableFrom<OperationCanceledException>(failure);
    }

    [Fact]
    public async Task Worker_readiness_signal_completes_before_the_first_enqueue()
    {
        var handler = new RecordingPayloadHandler();
        var readiness = new TaskCompletionSource<bool>(TaskCreationOptions.RunContinuationsAsynchronously);
        await using var host = HangfireEndToEndHost.CreateBuilder()
            .ConfigureServices(services => services.AddSingleton<IJobPayloadHandler>(handler))
            .Build();

        await host.StartAsync(TimeSpan.FromSeconds(10));

        // The worker must have registered with the storage before StartAsync
        // returns. If it had not, the readiness wait would have failed.
        var servers = host.Storage.GetMonitoringApi().Servers();
        Assert.NotEmpty(servers);
        readiness.TrySetResult(true);
    }

    [Fact]
    public async Task Enqueue_after_disposal_fails_with_a_deterministic_storage_error()
    {
        var handler = new RecordingPayloadHandler();
        var host = HangfireEndToEndHost.CreateBuilder()
            .ConfigureServices(services => services.AddSingleton<IJobPayloadHandler>(handler))
            .Build();
        await host.StartAsync();

        var dispatcher = host.Services.GetRequiredService<IJobDispatcher>();
        await host.DisposeAsync();

        Assert.True(host.IsDisposed);

        // After the host is disposed, the InMemoryStorage's background
        // dispatcher has been joined. A subsequent enqueue must surface a
        // deterministic storage error rather than a silent hang.
        await Assert.ThrowsAnyAsync<Exception>(async () => await dispatcher.EnqueueAsync(JobPayload.Create("after-dispose")));
    }

    [Fact]
    public async Task Multiple_isolated_hosts_can_be_built_in_sequence_with_no_shared_state()
    {
        var firstHandler = new RecordingPayloadHandler();
        var secondHandler = new RecordingPayloadHandler();
        await using (var first = HangfireEndToEndHost.CreateBuilder()
            .ConfigureServices(services => services.AddSingleton<IJobPayloadHandler>(firstHandler))
            .Build())
        {
            await first.StartAsync();
            var firstDispatcher = first.Services.GetRequiredService<IJobDispatcher>();
            await firstDispatcher.EnqueueAsync(JobPayload.Create("first"));
            var firstJob = await AwaitOrTimeout(firstHandler.HandledTask);
            Assert.Equal("first", firstJob.Name);
        }

        await using (var second = HangfireEndToEndHost.CreateBuilder()
            .ConfigureServices(services => services.AddSingleton<IJobPayloadHandler>(secondHandler))
            .Build())
        {
            await second.StartAsync();
            var secondDispatcher = second.Services.GetRequiredService<IJobDispatcher>();
            await secondDispatcher.EnqueueAsync(JobPayload.Create("second"));
            var secondJob = await AwaitOrTimeout(secondHandler.HandledTask);
            Assert.Equal("second", secondJob.Name);
            Assert.DoesNotContain(secondHandler.Handled, h => h.Name == "first");
        }
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

    private sealed class CountingPayloadHandler : IJobPayloadHandler
    {
        private readonly object _lock = new();
        private readonly Dictionary<long, TaskCompletionSource<JobPayload>> _pending = new();

        public List<JobPayload> Handled { get; } = new();

        public Task<JobPayload> NextAsync(long index)
        {
            lock (_lock)
            {
                var tcs = new TaskCompletionSource<JobPayload>(TaskCreationOptions.RunContinuationsAsynchronously);
                _pending[index] = tcs;
                return tcs.Task;
            }
        }

        public Task HandleAsync(JobPayload payload, CancellationToken cancellationToken)
        {
            lock (_lock)
            {
                Handled.Add(payload);
                if (payload.Arguments is not null
                    && payload.Arguments.TryGetValue("index", out var indexValue)
                    && Convert.ToInt64(indexValue) is long index
                    && _pending.TryGetValue(index, out var tcs))
                {
                    tcs.TrySetResult(payload);
                }
            }
            return Task.CompletedTask;
        }
    }

    private sealed class TypeSwitchingPayloadHandlerRegistry(params (string Name, RecordingPayloadHandler Handler)[] entries) : IJobPayloadHandler
    {
        public async Task HandleAsync(JobPayload payload, CancellationToken cancellationToken)
        {
            var match = Array.Find(entries, e => e.Name == payload.Name);
            ArgumentNullException.ThrowIfNull(match.Name);
            await match.Handler.HandleAsync(payload, cancellationToken);
        }
    }

    private sealed class GatedPayloadHandler : IJobPayloadHandler
    {
        private readonly TaskCompletionSource<GatedPayloadHandler> _entry = new(TaskCreationOptions.RunContinuationsAsynchronously);
        private readonly TaskCompletionSource<Exception> _failure = new(TaskCreationOptions.RunContinuationsAsynchronously);

        public Task<GatedPayloadHandler> EntryTask => _entry.Task;
        public Task<Exception> FailureTask => _failure.Task;

        public CancellationTokenSource TokenSource { get; } = new();

        public void Cancel() => TokenSource.Cancel();

        public async Task HandleAsync(JobPayload payload, CancellationToken cancellationToken)
        {
            _entry.TrySetResult(this);
            try
            {
                await Task.Delay(Timeout.Infinite, TokenSource.Token);
            }
            catch (OperationCanceledException ex)
            {
                _failure.TrySetResult(ex);
                throw;
            }
        }
    }
}

/// <summary>
/// xUnit collection that disables cross-class parallel execution for the
/// reliability tests. The reliability suite is timing-sensitive and shares
/// static Hangfire state (log provider, JobStorage.Current) across tests,
/// so it must run sequentially within a process.
/// </summary>
[CollectionDefinition(nameof(HangfireEndToEndReliabilityCollection), DisableParallelization = true)]
public sealed class HangfireEndToEndReliabilityCollection { }
