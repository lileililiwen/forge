using System.Collections.Concurrent;
using Hangfire;
using Hangfire.Common;
using Hangfire.States;
using Platform.Core.Time;

namespace Platform.Jobs.Hangfire;

/// <summary>
/// Hangfire-backed <see cref="IRecurringJobRegistry"/>. Registering a
/// descriptor attaches the platform recurring executor to the descriptor's
/// cron expression through the Hangfire recurring-job manager and records
/// the registration through the optional <see cref="IJobTelemetry"/>.
/// Registering a descriptor whose name was already registered is a no-op;
/// the earlier descriptor is kept, matching the platform contract.
/// </summary>
public sealed class HangfireRecurringJobRegistry : IRecurringJobRegistry
{
    private readonly IRecurringJobManager _manager;
    private readonly IClock _clock;
    private readonly IJobTelemetry? _telemetry;
    private readonly object _orderLock = new();
    private readonly List<string> _order = new();
    private readonly ConcurrentDictionary<string, RecurringJobDescriptor> _descriptors = new(StringComparer.Ordinal);

    /// <summary>
    /// Initializes a new instance of the <see cref="HangfireRecurringJobRegistry"/> class.
    /// </summary>
    /// <param name="manager">The Hangfire recurring-job manager.</param>
    /// <param name="clock">The platform clock.</param>
    /// <param name="telemetry">The optional platform job telemetry.</param>
    public HangfireRecurringJobRegistry(IRecurringJobManager manager, IClock clock, IJobTelemetry? telemetry = null)
    {
        ArgumentNullException.ThrowIfNull(manager);
        ArgumentNullException.ThrowIfNull(clock);
        _manager = manager;
        _clock = clock;
        _telemetry = telemetry;
    }

    /// <inheritdoc />
    public void Register(RecurringJobDescriptor descriptor)
    {
        ArgumentNullException.ThrowIfNull(descriptor);
        if (string.IsNullOrWhiteSpace(descriptor.Name))
        {
            throw new ArgumentException("The recurring job descriptor must carry a non-empty name.", nameof(descriptor));
        }

        if (string.IsNullOrWhiteSpace(descriptor.Cron))
        {
            throw new ArgumentException("The recurring job descriptor must carry a non-empty cron expression.", nameof(descriptor));
        }

        if (!_descriptors.TryAdd(descriptor.Name, descriptor))
        {
            return;
        }

        lock (_orderLock)
        {
            _order.Add(descriptor.Name);
        }

        try
        {
            _manager.AddOrUpdate(
                descriptor.Name,
                HangfireJobExecutor.CreateExecuteRecurringJob(descriptor.Name),
                descriptor.Cron,
                new RecurringJobOptions
                {
                    TimeZone = TimeZoneInfo.FindSystemTimeZoneById(descriptor.TimeZone),
                });
        }
        catch
        {
            _descriptors.TryRemove(descriptor.Name, out _);
            lock (_orderLock)
            {
                _order.Remove(descriptor.Name);
            }

            throw;
        }

        _telemetry?.JobRegistered(descriptor, _clock.UtcNow);
    }

    /// <inheritdoc />
    public IReadOnlyList<RecurringJobDescriptor> Registered
    {
        get
        {
            lock (_orderLock)
            {
                return _order.Select(name => _descriptors[name]).ToList();
            }
        }
    }
}
