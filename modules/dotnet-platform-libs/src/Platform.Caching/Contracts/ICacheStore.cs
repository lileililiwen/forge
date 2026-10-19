using Platform.Caching.Keys;

namespace Platform.Caching.Contracts;

/// <summary>Small asynchronous provider-neutral cache boundary.</summary>
public interface ICacheStore
{
    /// <summary>Reads a typed value.</summary>
    Task<CacheReadResult<T>> GetAsync<T>(CacheKey key, CancellationToken cancellationToken = default);

    /// <summary>Reads a value or invokes the factory on a miss.</summary>
    Task<CacheReadResult<T>> GetOrCreateAsync<T>(CacheKey key, Func<CancellationToken, Task<T>> factory, CacheEntryOptions? options = null, CancellationToken cancellationToken = default);

    /// <summary>Stores a typed value.</summary>
    Task<CacheOperationResult> SetAsync<T>(CacheKey key, T value, CacheEntryOptions? options = null, CancellationToken cancellationToken = default);

    /// <summary>Removes one entry.</summary>
    Task<CacheOperationResult> RemoveAsync(CacheKey key, CancellationToken cancellationToken = default);

    /// <summary>Removes all entries associated with a portable tag.</summary>
    Task<CacheOperationResult> RemoveByTagAsync(string tag, CancellationToken cancellationToken = default);
}
