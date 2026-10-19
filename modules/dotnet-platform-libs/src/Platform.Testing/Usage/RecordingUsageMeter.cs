using Platform.Billing.Contracts.Identifiers;
using Platform.Billing.Contracts.Usage;

namespace Platform.Testing.Usage;

/// <summary>
/// One recorded call to the fake usage meter.
/// </summary>
/// <param name="Subject">The subject the call was made for.</param>
/// <param name="Feature">The feature the call was made against.</param>
/// <param name="Units">The number of units recorded (0 for <c>Check</c>).</param>
/// <param name="Operation">The recorded operation, <c>Check</c> or <c>Record</c>.</param>
public sealed record UsageCall(
    SubjectKey Subject,
    FeatureKey Feature,
    long Units,
    UsageCall.OperationKind Operation)
{
    /// <summary>Identifies the type of call the fake recorded.</summary>
    public enum OperationKind
    {
        /// <summary>A read-only <c>Check</c> call.</summary>
        Check = 0,

        /// <summary>A mutating <c>Record</c> call.</summary>
        Record = 1,
    }
}

/// <summary>
/// Recording <see cref="IUsageMeter"/> implementation for tests.
/// Every call is recorded and inspectable; counts are accumulated in
/// memory without storage or windowing assumptions.
/// </summary>
public sealed class RecordingUsageMeter : IUsageMeter
{
    private readonly Dictionary<(SubjectKey, FeatureKey), long> _totals = new();
    private readonly Dictionary<FeatureKey, long> _limits = new();
    private readonly List<UsageCall> _calls = new();
    private readonly object _gate = new();

    /// <summary>
    /// Configures the limit returned for the supplied
    /// <paramref name="feature"/>. Pass <c>null</c> to remove the
    /// limit.
    /// </summary>
    /// <param name="feature">The feature to limit.</param>
    /// <param name="limit">The limit, or <c>null</c> to remove it.</param>
    public void SetLimit(FeatureKey feature, long? limit)
    {
        lock (_gate)
        {
            if (limit is null)
            {
                _limits.Remove(feature);
            }
            else
            {
                _limits[feature] = limit.Value;
            }
        }
    }

    /// <inheritdoc />
    public Task<UsageCheckResult> CheckAsync(
        SubjectKey subject,
        FeatureKey feature,
        CancellationToken cancellationToken = default)
    {
        lock (_gate)
        {
            _calls.Add(new UsageCall(subject, feature, 0, UsageCall.OperationKind.Check));
            var used = _totals.TryGetValue((subject, feature), out var current) ? current : 0L;
            long? limit = _limits.TryGetValue(feature, out var configured) ? configured : null;
            return Task.FromResult(new UsageCheckResult(feature, used, limit, WindowStart: null, WindowEnd: null));
        }
    }

    /// <inheritdoc />
    public Task<UsageCheckResult> RecordAsync(
        SubjectKey subject,
        FeatureKey feature,
        long units,
        CancellationToken cancellationToken = default)
    {
        if (units < 0)
        {
            throw new ArgumentOutOfRangeException(nameof(units), units, "Units must be non-negative.");
        }

        lock (_gate)
        {
            _calls.Add(new UsageCall(subject, feature, units, UsageCall.OperationKind.Record));
            var key = (subject, feature);
            _totals[key] = (_totals.TryGetValue(key, out var current) ? current : 0L) + units;
            long? limit = _limits.TryGetValue(feature, out var configured) ? configured : null;
            return Task.FromResult(new UsageCheckResult(feature, _totals[key], limit, WindowStart: null, WindowEnd: null));
        }
    }

    /// <summary>
    /// Returns the total recorded units for the supplied
    /// <paramref name="subject"/> and <paramref name="feature"/>.
    /// </summary>
    public long TotalFor(SubjectKey subject, FeatureKey feature)
    {
        lock (_gate)
        {
            return _totals.TryGetValue((subject, feature), out var current) ? current : 0L;
        }
    }

    /// <summary>
    /// Returns a snapshot of all recorded calls in invocation order.
    /// </summary>
    public IReadOnlyList<UsageCall> Calls
    {
        get
        {
            lock (_gate)
            {
                return _calls.ToArray();
            }
        }
    }

    /// <summary>
    /// Clears all recorded calls and accumulated totals. Configured
    /// limits are preserved.
    /// </summary>
    public void Reset()
    {
        lock (_gate)
        {
            _totals.Clear();
            _calls.Clear();
        }
    }
}
