namespace Platform.RateLimiting;

/// <summary>
/// Default <see cref="IRateLimiterBackendStatusProvider"/> that
/// always reports the in-memory backend as available. Consumer
/// adapters can replace the registration with a provider-specific
/// implementation.
/// </summary>
public sealed class InMemoryRateLimiterBackendStatusProvider : IRateLimiterBackendStatusProvider
{
    /// <summary>
    /// The documented provider name returned by the default status
    /// provider.
    /// </summary>
    public const string ProviderName = "memory";

    /// <inheritdoc />
    public IRateLimiterBackendStatus GetStatus() =>
        new(Provider: ProviderName, Available: true, Detail: "in-memory default backend");
}
