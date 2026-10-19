namespace Platform.RateLimiting;

/// <summary>
/// Contract for the readiness surface. The default implementation
/// reports the configured provider; consumer-supplied backends can
/// override the status without changing the readiness check.
/// </summary>
public interface IRateLimiterBackendStatusProvider
{
    /// <summary>
    /// Returns the documented backend status.
    /// </summary>
    IRateLimiterBackendStatus GetStatus();
}
