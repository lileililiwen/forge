namespace Platform.RateLimiting;

/// <summary>
/// Documented value type for a rate-limit key. The key is the
/// opaque, framework-neutral identifier the limiter uses to bucket
/// requests.
/// </summary>
/// <param name="Policy">The documented policy name. MUST be non-null and non-empty.</param>
/// <param name="Subject">The opaque subject identifier (typically a user, tenant, or token id).</param>
public readonly record struct RateLimitKey(string Policy, string Subject)
{
    /// <summary>
    /// Returns the documented composite form of the key, used as the
    /// dictionary key in the in-memory backend.
    /// </summary>
    public string Composite => string.Concat(Policy, "|", Subject);
}
