namespace Platform.RateLimiting;

/// <summary>
/// Documented decision returned by <see cref="IRateLimiter.CheckAsync"/>.
/// </summary>
/// <param name="Allowed">A value indicating whether the request is allowed.</param>
/// <param name="Limit">The configured limit for the policy.</param>
/// <param name="Remaining">The number of requests remaining in the current window.</param>
/// <param name="RetryAfterSeconds">When <paramref name="Allowed"/> is <c>false</c>, the documented ceil of the time remaining in the current window.</param>
public sealed record RateLimitDecision(
    bool Allowed,
    int Limit,
    int Remaining,
    int RetryAfterSeconds);
