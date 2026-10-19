using StackExchange.Redis;
using Platform.Caching.Contracts;
using Platform.Caching.Keys;
using Platform.Caching.Telemetry;

namespace Platform.Caching.Redis;

/// <summary>Redis cache adapter with bounded operations and safe transient failures.</summary>
public sealed class RedisCacheStore : ICacheStore, ICacheProviderStatus
{
    private readonly IDatabase _database;
    private readonly IConnectionMultiplexer _multiplexer;
    private readonly ICacheValueSerializer _serializer;
    private readonly RedisCacheOptions _options;
    private volatile CacheProviderStatus _status;

    /// <summary>Creates an adapter over a shared Redis multiplexer.</summary>
    public RedisCacheStore(IConnectionMultiplexer multiplexer, ICacheValueSerializer serializer, RedisCacheOptions? options = null)
    {
        _multiplexer = multiplexer ?? throw new ArgumentNullException(nameof(multiplexer));
        _serializer = serializer ?? throw new ArgumentNullException(nameof(serializer));
        _options = options ?? new RedisCacheOptions();
        _options.Validate();
        _database = multiplexer.GetDatabase(_options.Database);
        _status = multiplexer.IsConnected ? CacheProviderStatus.Healthy("redis") : CacheProviderStatus.Unavailable("redis", "cache.provider_unavailable");
    }

    /// <inheritdoc />
    public CacheProviderStatus Status => _status;

    /// <inheritdoc />
    public async Task<CacheReadResult<T>> GetAsync<T>(CacheKey key, CancellationToken cancellationToken = default)
    {
        try
        {
            var value = await AwaitBounded(_database.StringGetAsync(Physical(key)), cancellationToken).ConfigureAwait(false);
            if (!value.HasValue) { CacheTelemetry.Misses.Add(1); return new CacheReadResult<T>(CacheReadStatus.Miss); }
            var decoded = _serializer.Deserialize<T>((byte[])value!);
            if (decoded is null) { CacheTelemetry.Misses.Add(1); return new CacheReadResult<T>(CacheReadStatus.Miss); }
            _status = CacheProviderStatus.Healthy("redis");
            CacheTelemetry.Hits.Add(1);
            return new CacheReadResult<T>(CacheReadStatus.Hit, decoded);
        }
        catch (Exception exception) when (IsProviderFailure(exception)) { return Unavailable<T>(); }
    }

    /// <inheritdoc />
    public async Task<CacheReadResult<T>> GetOrCreateAsync<T>(CacheKey key, Func<CancellationToken, Task<T>> factory, CacheEntryOptions? options = null, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(factory);
        var existing = await GetAsync<T>(key, cancellationToken).ConfigureAwait(false);
        if (existing.Status == CacheReadStatus.Hit) return existing;
        var value = await factory(cancellationToken).ConfigureAwait(false);
        await SetAsync(key, value, options, cancellationToken).ConfigureAwait(false);
        return new CacheReadResult<T>(CacheReadStatus.Hit, value);
    }

    /// <inheritdoc />
    public async Task<CacheOperationResult> SetAsync<T>(CacheKey key, T value, CacheEntryOptions? options = null, CancellationToken cancellationToken = default)
    {
        options ??= new CacheEntryOptions();
        options.Validate();
        try
        {
            var stored = await AwaitBounded(_database.StringSetAsync(Physical(key), _serializer.Serialize(value), options.AbsoluteExpirationRelativeToNow), cancellationToken).ConfigureAwait(false);
            if (!stored) return Unavailable();
            foreach (var tag in options.Tags)
                await AwaitBounded(_database.SetAddAsync(TagKey(tag), Physical(key).ToString()), cancellationToken).ConfigureAwait(false);
            _status = CacheProviderStatus.Healthy("redis");
            return CacheOperationResult.Succeeded();
        }
        catch (Exception exception) when (IsProviderFailure(exception)) { return Unavailable(); }
    }

    /// <inheritdoc />
    public async Task<CacheOperationResult> RemoveAsync(CacheKey key, CancellationToken cancellationToken = default)
    {
        try { await AwaitBounded(_database.KeyDeleteAsync(Physical(key)), cancellationToken).ConfigureAwait(false); return CacheOperationResult.Succeeded(); }
        catch (Exception exception) when (IsProviderFailure(exception)) { return Unavailable(); }
    }

    /// <inheritdoc />
    public async Task<CacheOperationResult> RemoveByTagAsync(string tag, CancellationToken cancellationToken = default)
    {
        if (string.IsNullOrWhiteSpace(tag) || tag.Any(char.IsWhiteSpace)) throw new ArgumentException("Tags must be non-empty and whitespace-free.", nameof(tag));
        try
        {
            var keys = await AwaitBounded(_database.SetMembersAsync(TagKey(tag)), cancellationToken).ConfigureAwait(false);
            if (keys.Length > 0)
            {
                var physicalKeys = keys.Select(key => (RedisKey)key.ToString()).ToArray();
                await AwaitBounded(_database.KeyDeleteAsync(physicalKeys), cancellationToken).ConfigureAwait(false);
            }
            await AwaitBounded(_database.KeyDeleteAsync(TagKey(tag)), cancellationToken).ConfigureAwait(false);
            return CacheOperationResult.Succeeded();
        }
        catch (Exception exception) when (IsProviderFailure(exception)) { return Unavailable(); }
    }

    private RedisKey Physical(CacheKey key) => _options.KeyPrefix + key.Value;
    private RedisKey TagKey(string tag) => _options.KeyPrefix + "tag:" + tag;
    private static bool IsProviderFailure(Exception exception) => exception is RedisException or TimeoutException or RedisTimeoutException;
    private CacheOperationResult Unavailable()
    {
        _status = CacheProviderStatus.Unavailable("redis", "cache.provider_unavailable");
        CacheTelemetry.Failures.Add(1);
        return CacheOperationResult.Unavailable(new CacheFailure("cache.provider_unavailable", "The cache provider is unavailable.", true));
    }
    private CacheReadResult<T> Unavailable<T>()
    {
        Unavailable();
        return new CacheReadResult<T>(CacheReadStatus.Unavailable, default, new CacheFailure("cache.provider_unavailable", "The cache provider is unavailable.", true));
    }
    private async Task<T> AwaitBounded<T>(Task<T> task, CancellationToken cancellationToken) => await task.WaitAsync(_options.OperationTimeout, cancellationToken).ConfigureAwait(false);
}
