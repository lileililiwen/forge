namespace Platform.RateLimiting;

/// <summary>
/// Contract for a rate-limit decision. Implementations live in the
/// consumer; the platform ships an in-memory default
/// (<see cref="InMemoryRateLimiter"/>).
/// </summary>
public interface IRateLimiter
{
    /// <summary>
    /// Evaluates the supplied <paramref name="key"/> against the
    /// configured policy and returns the documented decision.
    /// Implementations MUST read the current time from an injected
    /// <see cref="Platform.Core.Time.IClock"/> so the bucket is
    /// deterministic in tests.
    /// </summary>
    /// <param name="key">The rate-limit key. MUST be non-default.</param>
    /// <param name="cancellationToken">The cancellation token.</param>
    /// <exception cref="ArgumentException">The key is default or its policy is unknown.</exception>
    Task<RateLimitDecision> CheckAsync(RateLimitKey key, CancellationToken cancellationToken = default);
}
