using Hangfire;
using Hangfire.Client;
using Hangfire.Common;
using Hangfire.InMemory;
using Hangfire.Server;
using Hangfire.States;
using Hangfire.Storage;
using Microsoft.Extensions.DependencyInjection;
using Platform.Core.Time;
using Platform.Jobs.Hangfire;

namespace Platform.Jobs.Hangfire.Tests;

public class JobContextTests : HangfireTest
{
    private static readonly Job SimpleJob =
        new(typeof(object), typeof(object).GetMethod("ToString")!, Array.Empty<object>());

    private sealed class FakeStorageConnection : IStorageConnection
    {
        public Dictionary<(string Id, string Name), string> Parameters { get; } = new();

        public IWriteOnlyTransaction CreateWriteTransaction() => throw new NotSupportedException();

        public IDisposable AcquireDistributedLock(string resource, TimeSpan timeout) => throw new NotSupportedException();

        public string CreateExpiredJob(Job job, IDictionary<string, string>? parameters, DateTime createdAt, TimeSpan expireIn) =>
            throw new NotSupportedException();

        public IFetchedJob FetchNextJob(string[] queues, CancellationToken cancellationToken) => throw new NotSupportedException();

        public void SetJobParameter(string id, string name, string value) => Parameters[(id, name)] = value;

        public string? GetJobParameter(string id, string name) =>
            Parameters.TryGetValue((id, name), out var value) ? value : null;

        public JobData GetJobData(string jobId) => throw new NotSupportedException();

        public StateData? GetStateData(string jobId) => throw new NotSupportedException();

        public void AnnounceServer(string serverId, ServerContext context) => throw new NotSupportedException();

        public void RemoveServer(string serverId) => throw new NotSupportedException();

        public void Heartbeat(string serverId) => throw new NotSupportedException();

        public int RemoveTimedOutServers(TimeSpan timeOut) => throw new NotSupportedException();

        public HashSet<string> GetAllItemsFromSet(string key) => throw new NotSupportedException();

        public string? GetFirstByLowestScoreFromSet(string key, double fromScore, double toScore) => throw new NotSupportedException();

        public void SetRangeInHash(string key, IEnumerable<KeyValuePair<string, string>> keyValuePairs) => throw new NotSupportedException();

        public Dictionary<string, string?> GetAllEntriesFromHash(string key) => throw new NotSupportedException();

        public void Dispose()
        {
        }
    }

    [Fact]
    public void Capture_filter_stores_the_ambient_context_as_a_job_parameter()
    {
        var bridge = new RecordingJobContextBridge();
        var filter = new JobContextCaptureFilter(new ServiceCollection()
            .AddSingleton<IJobExecutionContext>(bridge)
            .BuildServiceProvider());
        var creating = new CreatingContext(new CreateContext(
            new InMemoryStorage(),
            new FakeStorageConnection(),
            SimpleJob,
            new EnqueuedState()));

        RecordingJobContextBridge.SetAmbient("tenant-1", "subject-1");
        try
        {
            filter.OnCreating(creating);

            var snapshot = creating.GetJobParameter<JobContextSnapshot>(JobContextCaptureFilter.JobParameterName);
            Assert.NotNull(snapshot);
            Assert.Equal("tenant-1", snapshot!.TenantId);
            Assert.Equal("subject-1", snapshot.SubjectId);
        }
        finally
        {
            RecordingJobContextBridge.SetAmbient(null, null);
        }
    }

    [Fact]
    public void Capture_filter_skips_jobs_without_an_active_context()
    {
        var bridge = new RecordingJobContextBridge();
        var filter = new JobContextCaptureFilter(new ServiceCollection().BuildServiceProvider());
        var creating = new CreatingContext(new CreateContext(
            new InMemoryStorage(),
            new FakeStorageConnection(),
            SimpleJob,
            new EnqueuedState()));

        RecordingJobContextBridge.SetAmbient(null, null);
        filter.OnCreating(creating);

        Assert.Null(creating.GetJobParameter<JobContextSnapshot>(JobContextCaptureFilter.JobParameterName));
        Assert.Empty(bridge.Captured);
    }

    [Fact]
    public void Capture_filter_without_a_registered_bridge_captures_nothing()
    {
        var filter = new JobContextCaptureFilter(new ServiceCollection().BuildServiceProvider());
        var creating = new CreatingContext(new CreateContext(
            new InMemoryStorage(),
            new FakeStorageConnection(),
            SimpleJob,
            new EnqueuedState()));

        RecordingJobContextBridge.SetAmbient("tenant-1", null);
        try
        {
            filter.OnCreating(creating);
        }
        finally
        {
            RecordingJobContextBridge.SetAmbient(null, null);
        }

        Assert.Null(creating.GetJobParameter<JobContextSnapshot>(JobContextCaptureFilter.JobParameterName));
    }

    [Fact]
    public void Activator_restores_the_captured_context_and_disposes_it_with_the_scope()
    {
        var bridge = new RecordingJobContextBridge();
        var scopeFactory = new ServiceCollection()
            .AddSingleton<IJobExecutionContext>(bridge)
            .BuildServiceProvider()
            .GetRequiredService<IServiceScopeFactory>();
        var connection = new FakeStorageConnection();
        var performContext = new PerformContext(connection, new BackgroundJob("job-1", SimpleJob, DateTime.UtcNow), new FakeJobCancellationToken());
        performContext.SetJobParameter(
            JobContextCaptureFilter.JobParameterName,
            new JobContextSnapshot("tenant-1", "subject-1"));
        var activator = new ScopedJobActivator(scopeFactory);

        var scope = activator.BeginScope(performContext);
        Assert.Single(bridge.Restored);
        Assert.Equal("tenant-1", bridge.Restored[0].TenantId);
        Assert.Empty(bridge.Disposed);

        scope.Dispose();

        var disposed = Assert.Single(bridge.Disposed);
        Assert.Equal("tenant-1", disposed.TenantId);
    }

    [Fact]
    public void Activator_runs_jobs_without_context_free()
    {
        var bridge = new RecordingJobContextBridge();
        var scopeFactory = new ServiceCollection()
            .AddSingleton<IJobExecutionContext>(bridge)
            .BuildServiceProvider()
            .GetRequiredService<IServiceScopeFactory>();
        var performContext = new PerformContext(
            new FakeStorageConnection(),
            new BackgroundJob("job-1", SimpleJob, DateTime.UtcNow),
            new FakeJobCancellationToken());
        var activator = new ScopedJobActivator(scopeFactory);

        using var scope = activator.BeginScope(performContext);

        Assert.Empty(bridge.Restored);
        Assert.NotNull(scope.Resolve(typeof(RecordingPayloadHandler)));
    }

    [Fact]
    public void Activator_fails_closed_when_a_context_carrying_job_has_no_bridge()
    {
        var scopeFactory = new ServiceCollection()
            .BuildServiceProvider()
            .GetRequiredService<IServiceScopeFactory>();
        var connection = new FakeStorageConnection();
        var performContext = new PerformContext(connection, new BackgroundJob("job-1", SimpleJob, DateTime.UtcNow), new FakeJobCancellationToken());
        performContext.SetJobParameter(
            JobContextCaptureFilter.JobParameterName,
            new JobContextSnapshot("tenant-1", null));
        var activator = new ScopedJobActivator(scopeFactory);

        Assert.Throws<InvalidOperationException>(() => activator.BeginScope(performContext));
    }
}
