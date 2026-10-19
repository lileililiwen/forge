using Hangfire;
using Hangfire.Common;
using Hangfire.InMemory;
using Hangfire.Server;
using Hangfire.States;
using Hangfire.Storage;
using Microsoft.Extensions.DependencyInjection;
using Platform.Core.Results;
using Platform.Core.Time;
using Platform.Jobs.Hangfire;
using Platform.Jobs.Hangfire.DependencyInjection;

[assembly: CollectionBehavior(DisableTestParallelization = true)]

namespace Platform.Jobs.Hangfire.Tests;

public sealed class FixedClock(DateTimeOffset now) : IClock
{
    public DateTimeOffset Now { get; set; } = now;

    public DateTimeOffset UtcNow => Now;
}

public abstract class HangfireTest
{
    protected HangfireTest() => GlobalConfiguration.Configuration.UseNoOpLogProvider();
}

public sealed class RecordingJobTelemetry : IJobTelemetry
{
    public List<(RecurringJobDescriptor Descriptor, DateTimeOffset At)> Registered { get; } = new();
    public List<(JobPayload Payload, DateTimeOffset At)> Enqueued { get; } = new();
    public List<(string Name, DateTimeOffset At)> Executed { get; } = new();
    public List<(string Name, Error Error, DateTimeOffset At)> Failed { get; } = new();

    public void JobRegistered(RecurringJobDescriptor descriptor, DateTimeOffset registeredAt) =>
        Registered.Add((descriptor, registeredAt));

    public void JobEnqueued(JobPayload payload, DateTimeOffset enqueuedAt) =>
        Enqueued.Add((payload, enqueuedAt));

    public void JobExecuted(string name, DateTimeOffset executedAt) =>
        Executed.Add((name, executedAt));

    public void JobFailed(string name, Error error, DateTimeOffset failedAt) =>
        Failed.Add((name, error, failedAt));
}

public sealed class RecordingBackgroundJobClient : IBackgroundJobClient
{
    public List<(Job Job, IState State)> Created { get; } = new();

    public string Create(Job job, IState state)
    {
        Created.Add((job, state));
        return $"job-{Created.Count}";
    }

    public string Create(Job job, IState state, CancellationToken cancellationToken) => Create(job, state);

    public bool ChangeState(string jobId, IState state, string? expectedState)
    {
        StateChanges.Add((jobId, state, expectedState));
        return true;
    }

    public List<(string JobId, IState State, string? ExpectedState)> StateChanges { get; } = new();
}

public sealed class RecordingRecurringJobManager : IRecurringJobManager
{
    public List<(string Id, Job Job, string Cron, RecurringJobOptions Options)> Added { get; } = new();

    public void AddOrUpdate(string recurringJobId, Job job, string cronExpression, RecurringJobOptions options) =>
        Added.Add((recurringJobId, job, cronExpression, options));

    public void Trigger(string recurringJobId) => Triggered.Add(recurringJobId);

    public void RemoveIfExists(string recurringJobId) => Removed.Add(recurringJobId);

    public List<string> Triggered { get; } = new();

    public List<string> Removed { get; } = new();
}

public sealed class RecordingJobContextBridge : IJobExecutionContext
{
    private static readonly AsyncLocal<(string? Tenant, string? Subject)> Ambient = new();

    public List<JobContextSnapshot> Captured { get; } = new();
    public List<JobContextSnapshot> Restored { get; } = new();
    public List<JobContextSnapshot> Disposed { get; } = new();

    public static (string? Tenant, string? Subject) Current => Ambient.Value;

    public static void SetAmbient(string? tenant, string? subject) => Ambient.Value = (tenant, subject);

    public JobContextSnapshot? Capture()
    {
        var (tenant, subject) = Ambient.Value;
        if (tenant is null && subject is null)
        {
            return null;
        }

        var snapshot = new JobContextSnapshot(tenant, subject);
        Captured.Add(snapshot);
        return snapshot;
    }

    public IDisposable Restore(JobContextSnapshot snapshot)
    {
        Restored.Add(snapshot);
        Ambient.Value = (snapshot.TenantId, snapshot.SubjectId);
        return new Restoration(this, snapshot);
    }

    private sealed class Restoration(RecordingJobContextBridge bridge, JobContextSnapshot snapshot) : IDisposable
    {
        public void Dispose()
        {
            bridge.Disposed.Add(snapshot);
            Ambient.Value = (null, null);
        }
    }
}

public sealed class RecordingPayloadHandler : IJobPayloadHandler
{
    private readonly TaskCompletionSource<JobPayload> _completion = new(TaskCreationOptions.RunContinuationsAsynchronously);
    private readonly TaskCompletionSource<JobPayload> _failed = new(TaskCreationOptions.RunContinuationsAsynchronously);

    public List<JobPayload> Handled { get; } = new();

    public Task<JobPayload> HandledTask => _completion.Task;
    public Task<JobPayload> FailedTask => _failed.Task;

    public Exception? Failure { get; set; }

    public async Task HandleAsync(JobPayload payload, CancellationToken cancellationToken)
    {
        Handled.Add(payload);
        if (Failure is not null)
        {
            _failed.TrySetResult(payload);
            throw Failure;
        }

        _completion.TrySetResult(payload);
        await Task.CompletedTask;
    }
}

public sealed class RecordingRecurringHandler : IRecurringJobHandler
{
    private readonly TaskCompletionSource<object?> _completion = new(TaskCreationOptions.RunContinuationsAsynchronously);

    public int Executions { get; private set; }

    public Exception? Failure { get; set; }

    public Task ExecuteAsync(CancellationToken cancellationToken)
    {
        Executions++;
        if (Failure is not null)
        {
            throw Failure;
        }

        _completion.TrySetResult(null);
        return Task.CompletedTask;
    }
}

public sealed class RecordingDispatcher : IJobDispatcher
{
    public List<JobPayload> Enqueued { get; } = new();

    public Task EnqueueAsync(JobPayload payload, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(payload);
        Enqueued.Add(payload);
        return Task.CompletedTask;
    }
}

public sealed class InMemoryRecurringJobRegistry : IRecurringJobRegistry
{
    private readonly Dictionary<string, RecurringJobDescriptor> _byName = new(StringComparer.Ordinal);

    public IReadOnlyList<RecurringJobDescriptor> Registered => _byName.Values.ToList();

    public void Register(RecurringJobDescriptor descriptor)
    {
        ArgumentNullException.ThrowIfNull(descriptor);
        _byName.TryAdd(descriptor.Name, descriptor);
    }
}

public sealed class ThrowingStorage : JobStorage
{
    public override IMonitoringApi GetMonitoringApi() => throw new InvalidOperationException("storage unreachable");

    public override IStorageConnection GetConnection() => throw new InvalidOperationException("storage unreachable");
}

public sealed class FakeJobCancellationToken : IJobCancellationToken
{
    public CancellationToken ShutdownToken => CancellationToken.None;

    public void ThrowIfCancellationRequested()
    {
    }
}

public static class HangfireTestHost
{
    public static (ServiceProvider Provider, InMemoryStorage Storage) Build(Action<HangfireJobsOptions>? configure = null)
    {
        var services = new ServiceCollection();
        services.AddLogging();
        services.AddPlatformHangfireJobs(configure);
        var provider = services.BuildServiceProvider();
        var storage = (InMemoryStorage)provider.GetRequiredService<JobStorage>();
        return (provider, storage);
    }
}
