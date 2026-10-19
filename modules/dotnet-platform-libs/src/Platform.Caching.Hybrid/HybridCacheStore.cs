using Microsoft.Extensions.Caching.Hybrid;
using Platform.Caching.Contracts;
using Platform.Caching.Keys;
using Platform.Caching.Telemetry;

namespace Platform.Caching.Hybrid;

/// <summary>Platform cache boundary backed by Microsoft HybridCache.</summary>
public sealed class HybridCacheStore : ICacheStore, ICacheProviderStatus
{
    private readonly Microsoft.Extensions.Caching.Hybrid.HybridCache _inner;

    /// <summary>Creates an adapter over a registered HybridCache.</summary>
    public HybridCacheStore(Microsoft.Extensions.Caching.Hybrid.HybridCache inner) => _inner = inner ?? throw new ArgumentNullException(nameof(inner));

    /// <inheritdoc />
    public CacheProviderStatus Status => CacheProviderStatus.Healthy("hybrid");

    /// <inheritdoc />
    public async Task<CacheReadResult<T>> GetAsync<T>(CacheKey key, CancellationToken cancellationToken = default)
    {
        var invoked = false;
        T? value = default;
        try
        {
            value = await _inner.GetOrCreateAsync(
                key.Value,
                new FactoryState<T>(static _ => Task.FromResult(default(T)!), () => invoked = true),
                static async (state, ct) =>
                {
                    state.MarkInvoked();
                    return await state.Factory(ct).ConfigureAwait(false);
                },
                new HybridCacheEntryOptions { Expiration = TimeSpan.FromMilliseconds(1) },
                cancellationToken: cancellationToken).ConfigureAwait(false);
        }
        catch (Exception exception) when (IsProviderFailure(exception))
        {
            CacheTelemetry.Failures.Add(1);
            return new CacheReadResult<T>(CacheReadStatus.Unavailable, default, new CacheFailure("cache.provider_unavailable", "The cache provider is unavailable.", true));
        }

        // HybridCache has no read-only operation. A short-lived probe cannot safely return the
        // factory sentinel as a public value, so this adapter uses GetOrCreate for normal reads.
        return invoked
            ? new CacheReadResult<T>(CacheReadStatus.Miss)
            : new CacheReadResult<T>(CacheReadStatus.Hit, value);
    }

    /// <inheritdoc />
    public async Task<CacheReadResult<T>> GetOrCreateAsync<T>(CacheKey key, Func<CancellationToken, Task<T>> factory, CacheEntryOptions? options = null, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(factory);
        options ??= new CacheEntryOptions();
        options.Validate();
        var invoked = false;
        try
        {
            var value = await _inner.GetOrCreateAsync(
                key.Value,
                new FactoryState<T>(factory, () => invoked = true),
                static async (state, ct) =>
                {
                    state.MarkInvoked();
                    return await state.Factory(ct).ConfigureAwait(false);
                },
                new HybridCacheEntryOptions { Expiration = options.AbsoluteExpirationRelativeToNow },
                options.Tags,
                cancellationToken).ConfigureAwait(false);
            (invoked ? CacheTelemetry.Misses : CacheTelemetry.Hits).Add(1);
            return new CacheReadResult<T>(CacheReadStatus.Hit, value);
        }
        catch (Exception exception) when (IsProviderFailure(exception))
        {
            CacheTelemetry.Failures.Add(1);
            return new CacheReadResult<T>(CacheReadStatus.Unavailable, default, new CacheFailure("cache.provider_unavailable", "The cache provider is unavailable.", true));
        }
    }

    /// <inheritdoc />
    public async Task<CacheOperationResult> SetAsync<T>(CacheKey key, T value, CacheEntryOptions? options = null, CancellationToken cancellationToken = default)
    {
        options ??= new CacheEntryOptions();
        options.Validate();
        try
        {
            await _inner.SetAsync(key.Value, value, new HybridCacheEntryOptions { Expiration = options.AbsoluteExpirationRelativeToNow }, options.Tags, cancellationToken).ConfigureAwait(false);
            return CacheOperationResult.Succeeded();
        }
        catch (Exception exception) when (IsProviderFailure(exception)) { return Unavailable(); }
    }

    /// <inheritdoc />
    public async Task<CacheOperationResult> RemoveAsync(CacheKey key, CancellationToken cancellationToken = default)
    {
        try { await _inner.RemoveAsync(key.Value, cancellationToken).ConfigureAwait(false); return CacheOperationResult.Succeeded(); }
        catch (Exception exception) when (IsProviderFailure(exception)) { return Unavailable(); }
    }

    /// <inheritdoc />
    public async Task<CacheOperationResult> RemoveByTagAsync(string tag, CancellationToken cancellationToken = default)
    {
        if (string.IsNullOrWhiteSpace(tag) || tag.Any(char.IsWhiteSpace)) throw new ArgumentException("Tags must be non-empty and whitespace-free.", nameof(tag));
        try { await _inner.RemoveByTagAsync(tag, cancellationToken).ConfigureAwait(false); return CacheOperationResult.Succeeded(); }
        catch (Exception exception) when (IsProviderFailure(exception)) { return Unavailable(); }
    }

    private static bool IsProviderFailure(Exception exception) => exception is not OperationCanceledException;
    private static CacheOperationResult Unavailable() => CacheOperationResult.Unavailable(new CacheFailure("cache.provider_unavailable", "The cache provider is unavailable.", true));

    private sealed class FactoryState<T>(Func<CancellationToken, Task<T>> factory, Action markInvoked)
    {
        public Func<CancellationToken, Task<T>> Factory { get; } = factory;
        public void MarkInvoked() => markInvoked();
    }
}
