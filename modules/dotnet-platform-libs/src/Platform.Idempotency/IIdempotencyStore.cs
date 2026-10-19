namespace Platform.Idempotency;

/// <summary>
/// Contract for an idempotency store. Implementations live in the
/// consumer; the platform ships an in-memory default
/// (<see cref="InMemoryIdempotencyStore"/>).
/// </summary>
public interface IIdempotencyStore
{
    /// <summary>
    /// Returns the <see cref="IdempotencyRecord"/> stored under the
    /// supplied <paramref name="key"/>, or <c>null</c> when the key
    /// is unknown or the stored record has expired.
    /// </summary>
    /// <param name="key">The idempotency key. MUST be non-null and non-empty.</param>
    /// <param name="cancellationToken">The cancellation token.</param>
    Task<IdempotencyRecord?> TryGetAsync(string key, CancellationToken cancellationToken = default);

    /// <summary>
    /// Persists the supplied <paramref name="record"/> under its
    /// <see cref="IdempotencyRecord.Key"/>. Implementations MUST
    /// reject keys longer than the configured maximum and MUST read
    /// the current time from an injected
    /// <see cref="Platform.Core.Time.IClock"/> so the test path is
    /// deterministic.
    /// </summary>
    /// <param name="record">The record to store.</param>
    /// <param name="cancellationToken">The cancellation token.</param>
    /// <exception cref="ArgumentNullException"><paramref name="record"/> is <c>null</c>.</exception>
    /// <exception cref="ArgumentException">The key is null, empty, whitespace, or longer than the configured maximum.</exception>
    Task SaveAsync(IdempotencyRecord record, CancellationToken cancellationToken = default);

    /// <summary>
    /// Removes every record whose
    /// <see cref="IdempotencyRecord.CreatedAt"/> is older than the
    /// documented retention window. Returns the number of records
    /// removed.
    /// </summary>
    /// <param name="cancellationToken">The cancellation token.</param>
    Task<int> EvictExpiredAsync(CancellationToken cancellationToken = default);
}
