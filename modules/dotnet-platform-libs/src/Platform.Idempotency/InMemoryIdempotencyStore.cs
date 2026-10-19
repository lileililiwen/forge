using Microsoft.Extensions.Options;
using Platform.Core.Time;

namespace Platform.Idempotency;

/// <summary>
/// Default in-memory implementation of <see cref="IIdempotencyStore"/>.
/// Records are kept in a thread-safe dictionary keyed by
/// <see cref="IdempotencyRecord.Key"/>. The store reads the current
/// time from an injected <see cref="IClock"/> so the
/// <see cref="EvictExpiredAsync"/> sweep is deterministic in tests.
/// </summary>
public sealed class InMemoryIdempotencyStore : IIdempotencyStore
{
    private readonly IClock _clock;
    private readonly IdempotencyOptions _options;
    private readonly Dictionary<string, IdempotencyRecord> _records = new(StringComparer.Ordinal);
    private readonly object _lock = new();

    /// <summary>
    /// Initializes a new <see cref="InMemoryIdempotencyStore"/> with
    /// the supplied dependencies.
    /// </summary>
    /// <param name="clock">The platform clock.</param>
    /// <param name="options">The idempotency options.</param>
    /// <exception cref="ArgumentNullException">A required dependency is <c>null</c>.</exception>
    public InMemoryIdempotencyStore(IClock clock, IOptions<IdempotencyOptions> options)
    {
        ArgumentNullException.ThrowIfNull(clock);
        ArgumentNullException.ThrowIfNull(options);
        _clock = clock;
        _options = options.Value;
    }

    /// <inheritdoc />
    public Task<IdempotencyRecord?> TryGetAsync(string key, CancellationToken cancellationToken = default)
    {
        ValidateKey(key);

        lock (_lock)
        {
            if (!_records.TryGetValue(key, out var record))
            {
                return Task.FromResult<IdempotencyRecord?>(null);
            }
            if (IsExpired(record, _clock.UtcNow))
            {
                _records.Remove(key);
                return Task.FromResult<IdempotencyRecord?>(null);
            }
            return Task.FromResult<IdempotencyRecord?>(record);
        }
    }

    /// <inheritdoc />
    public Task SaveAsync(IdempotencyRecord record, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(record);
        ValidateKey(record.Key);
        if (record.Key.Length > _options.MaxKeyLength)
        {
            throw new ArgumentException(
                $"Idempotency key length {record.Key.Length} exceeds the configured maximum of {_options.MaxKeyLength}.",
                nameof(record));
        }

        lock (_lock)
        {
            _records[record.Key] = record;
        }
        return Task.CompletedTask;
    }

    /// <inheritdoc />
    public Task<int> EvictExpiredAsync(CancellationToken cancellationToken = default)
    {
        var now = _clock.UtcNow;
        var removed = 0;
        lock (_lock)
        {
            var expired = _records
                .Where(pair => IsExpired(pair.Value, now))
                .Select(pair => pair.Key)
                .ToArray();
            foreach (var key in expired)
            {
                _records.Remove(key);
                removed++;
            }
        }
        return Task.FromResult(removed);
    }

    private static void ValidateKey(string key)
    {
        if (string.IsNullOrWhiteSpace(key))
        {
            throw new ArgumentException("Idempotency key must be a non-empty string.", nameof(key));
        }
    }

    private bool IsExpired(IdempotencyRecord record, DateTimeOffset now)
    {
        var age = now - record.CreatedAt;
        return age.TotalSeconds > _options.RetentionSeconds;
    }
}
