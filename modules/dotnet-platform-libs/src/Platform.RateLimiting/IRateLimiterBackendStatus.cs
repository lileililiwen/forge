namespace Platform.RateLimiting;

/// <summary>
/// Documented backend-status shape returned by
/// <see cref="IRateLimiterBackendStatusProvider.GetStatus"/>.
/// </summary>
/// <param name="Provider">The configured provider name (e.g. <c>"memory"</c>).</param>
/// <param name="Available">A value indicating whether the backend is currently available.</param>
/// <param name="Detail">A short human-readable detail string.</param>
public sealed record IRateLimiterBackendStatus(
    string Provider,
    bool Available,
    string? Detail = null);
