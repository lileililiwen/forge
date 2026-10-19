using Platform.Caching.Contracts;
using Platform.Caching.Keys;
using Platform.Caching.Telemetry;
using Platform.Core.Time;

namespace Platform.Caching.InMemory;

/// <summary>Thread-safe, non-authoritative cache for local and test deployments.</summary>
public sealed class InMemoryCacheStore : ICacheStore, ICacheProviderStatus
{
    private readonly object _gate = new();
    private readonly Dictionary<string, Entry> _entries = new(StringComparer.Ordinal);
    private readonly IClock _clock;

    /// <summary>Creates an in-memory store using the supplied clock.</summary>
    public InMemoryCacheStore(IClock clock) => _clock = clock ?? throw new ArgumentNullException(nameof(clock));

    /// <inheritdoc />
    public CacheProviderStatus Status => CacheProviderStatus.Healthy("in-memory");

    /// <inheritdoc />
    public Task<CacheReadResult<T>> GetAsync<T>(CacheKey key, CancellationToken cancellationToken = default)
    {
        cancellationToken.ThrowIfCancellationRequested();
        lock (_gate)
        {
            if (!_entries.TryGetValue(key.Value, out var entry) || entry.ExpiresAt <= _clock.UtcNow)
            {
                _entries.Remove(key.Value);
                CacheTelemetry.Misses.Add(1);
                return Task.FromResult(new CacheReadResult<T>(CacheReadStatus.Miss));
            }
            if (entry.Value is not T value)
            {
                CacheTelemetry.Misses.Add(1);
                return Task.FromResult(new CacheReadResult<T>(CacheReadStatus.Miss));
            }
            CacheTelemetry.Hits.Add(1);
            return Task.FromResult(new CacheReadResult<T>(CacheReadStatus.Hit, value));
        }
    }

    /// <inheritdoc />
    public async Task<CacheReadResult<T>> GetOrCreateAsync<T>(CacheKey key, Func<CancellationToken, Task<T>> factory, CacheEntryOptions? options = null, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(factory);
        var result = await GetAsync<T>(key, cancellationToken).ConfigureAwait(false);
        if (result.Status == CacheReadStatus.Hit) return result;
        var value = await factory(cancellationToken).ConfigureAwait(false);
        await SetAsync(key, value, options, cancellationToken).ConfigureAwait(false);
        return new CacheReadResult<T>(CacheReadStatus.Hit, value);
    }

    /// <inheritdoc />
    public Task<CacheOperationResult> SetAsync<T>(CacheKey key, T value, CacheEntryOptions? options = null, CancellationToken cancellationToken = default)
    {
        cancellationToken.ThrowIfCancellationRequested();
        options ??= new CacheEntryOptions();
        options.Validate();
        lock (_gate) _entries[key.Value] = new Entry(value, _clock.UtcNow.Add(options.AbsoluteExpirationRelativeToNow), options.Tags.ToHashSet(StringComparer.Ordinal));
        return Task.FromResult(CacheOperationResult.Succeeded());
    }

    /// <inheritdoc />
    public Task<CacheOperationResult> RemoveAsync(CacheKey key, CancellationToken cancellationToken = default)
    {
        cancellationToken.ThrowIfCancellationRequested();
        lock (_gate) _entries.Remove(key.Value);
        return Task.FromResult(CacheOperationResult.Succeeded());
    }

    /// <inheritdoc />
    public Task<CacheOperationResult> RemoveByTagAsync(string tag, CancellationToken cancellationToken = default)
    {
        if (string.IsNullOrWhiteSpace(tag) || tag.Any(char.IsWhiteSpace)) throw new ArgumentException("Tags must be non-empty and whitespace-free.", nameof(tag));
        cancellationToken.ThrowIfCancellationRequested();
        lock (_gate)
        {
            foreach (var key in _entries.Where(pair => pair.Value.Tags.Contains(tag)).Select(pair => pair.Key).ToArray()) _entries.Remove(key);
        }
        return Task.FromResult(CacheOperationResult.Succeeded());
    }

    private sealed record Entry(object? Value, DateTimeOffset ExpiresAt, HashSet<string> Tags);
}
