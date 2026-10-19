namespace Platform.RateLimiting;

/// <summary>
/// Documented per-policy options. The platform reads the catalog
/// from <see cref="RateLimitingOptions.Policies"/>.
/// </summary>
public sealed class RateLimitPolicyOptions
{
    /// <summary>
    /// Gets or sets the documented policy name. MUST be non-null
    /// and non-empty.
    /// </summary>
    public string Name { get; set; } = string.Empty;

    /// <summary>
    /// Gets or sets the documented request limit per window. MUST be
    /// positive.
    /// </summary>
    public int Limit { get; set; }

    /// <summary>
    /// Gets or sets the documented window length in seconds. MUST be
    /// positive.
    /// </summary>
    public int WindowSeconds { get; set; }
}
