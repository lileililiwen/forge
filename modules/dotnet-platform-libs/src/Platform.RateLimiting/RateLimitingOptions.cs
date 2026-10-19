namespace Platform.RateLimiting;

/// <summary>
/// Configuration bound to the <c>RateLimiting</c> configuration
/// section by the
/// <c>Platform.RateLimiting.DependencyInjection.ServiceCollectionExtensions.AddPlatformRateLimiting</c>
/// extension. The <see cref="Policies"/> catalog is the documented
/// default set consumers can override.
/// </summary>
public sealed class RateLimitingOptions
{
    /// <summary>
    /// The configuration section name bound by
    /// <c>Platform.RateLimiting.DependencyInjection.ServiceCollectionExtensions.AddPlatformRateLimiting</c>.
    /// </summary>
    public const string SectionName = "RateLimiting";

    /// <summary>
    /// Gets or sets the documented default policy catalog. Defaults
    /// to <c>feed</c>, <c>search</c>, <c>uploads</c>,
    /// <c>downloads</c>, and <c>account-recovery</c> with the
    /// documented limits and windows.
    /// </summary>
    public RateLimitPolicies Policies { get; set; } = RateLimitPolicies.Default();

    /// <summary>
    /// Gets or sets the documented bypass tokens. A request whose
    /// <see cref="HttpContextAbstraction.BypassToken"/> matches one
    /// of these tokens short-circuits the limiter.
    /// </summary>
    public IReadOnlyList<string> BypassTokens { get; set; } = Array.Empty<string>();
}
