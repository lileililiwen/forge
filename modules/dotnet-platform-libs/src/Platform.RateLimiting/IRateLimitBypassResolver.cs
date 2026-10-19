namespace Platform.RateLimiting;

/// <summary>
/// Documented result returned by <see cref="IRateLimitBypassResolver"/>.
/// </summary>
/// <param name="Allowed">A value indicating whether the request bypasses the limiter.</param>
/// <param name="Label">A short human-readable label describing the bypass reason (e.g. the matched token).</param>
public sealed record RateLimitBypassDecision(bool Allowed, string? Label = null);

/// <summary>
/// Contract for a rate-limit bypass resolver. Implementations live in
/// the consumer; the platform ships a default
/// <see cref="ConfigurationRateLimitBypassResolver"/>.
/// </summary>
public interface IRateLimitBypassResolver
{
    /// <summary>
    /// Evaluates the supplied <paramref name="context"/> and returns
    /// the documented bypass decision. Callers MUST honour the
    /// decision before invoking the limiter.
    /// </summary>
    /// <param name="context">The framework-neutral context.</param>
    /// <returns>The bypass decision.</returns>
    /// <exception cref="ArgumentNullException"><paramref name="context"/> is <c>null</c>.</exception>
    RateLimitBypassDecision Evaluate(HttpContextAbstraction context);
}
