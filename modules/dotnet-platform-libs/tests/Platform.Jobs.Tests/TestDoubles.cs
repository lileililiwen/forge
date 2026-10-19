using Platform.Core.Results;
using Platform.Core.Time;

namespace Platform.Jobs.Tests;

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

public sealed class RecordingDispatcher : IJobDispatcher
{
    private readonly IClock _clock;
    private readonly IJobTelemetry? _telemetry;

    public RecordingDispatcher(IClock clock, IJobTelemetry? telemetry = null)
    {
        _clock = clock;
        _telemetry = telemetry;
    }

    public List<JobPayload> Enqueued { get; } = new();

    public Task EnqueueAsync(JobPayload payload, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(payload);
        Enqueued.Add(payload);
        _telemetry?.JobEnqueued(payload, _clock.UtcNow);
        return Task.CompletedTask;
    }
}

public sealed class InMemoryRecurringJobRegistry : IRecurringJobRegistry
{
    private readonly IClock _clock;
    private readonly IJobTelemetry? _telemetry;
    private readonly Dictionary<string, RecurringJobDescriptor> _byName = new(StringComparer.Ordinal);

    public InMemoryRecurringJobRegistry(IClock clock, IJobTelemetry? telemetry = null)
    {
        _clock = clock;
        _telemetry = telemetry;
    }

    public IReadOnlyList<RecurringJobDescriptor> Registered =>
        _byName.Values.ToList();

    public void Register(RecurringJobDescriptor descriptor)
    {
        ArgumentNullException.ThrowIfNull(descriptor);
        if (_byName.ContainsKey(descriptor.Name))
        {
            return;
        }

        _byName[descriptor.Name] = descriptor;
        _telemetry?.JobRegistered(descriptor, _clock.UtcNow);
    }
}
