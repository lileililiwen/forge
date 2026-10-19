namespace Platform.Idempotency;

/// <summary>
/// Configuration bound to the <c>Idempotency</c> configuration
/// section by the
/// <c>Platform.Idempotency.DependencyInjection.ServiceCollectionExtensions.AddPlatformIdempotency</c>
/// extension. All properties have safe defaults; consumers override
/// only what they need.
/// </summary>
public sealed class IdempotencyOptions
{
    /// <summary>
    /// The configuration section name bound by
    /// <c>Platform.Idempotency.DependencyInjection.ServiceCollectionExtensions.AddPlatformIdempotency</c>.
    /// </summary>
    public const string SectionName = "Idempotency";

    /// <summary>
    /// The documented metric name for the
    /// <c>idempotency.hit</c> counter.
    /// </summary>
    public const string HitMetric = "idempotency.hit";

    /// <summary>
    /// The documented metric name for the
    /// <c>idempotency.miss</c> counter.
    /// </summary>
    public const string MissMetric = "idempotency.miss";

    /// <summary>
    /// The documented metric name for the
    /// <c>idempotency.fingerprint_mismatch</c> counter.
    /// </summary>
    public const string FingerprintMismatchMetric = "idempotency.fingerprint_mismatch";

    /// <summary>
    /// Gets or sets a value indicating whether idempotency is
    /// enabled. When <c>false</c>,
    /// <c>AddPlatformIdempotency</c> is a no-op and the in-memory
    /// store is not registered. Defaults to <c>true</c>.
    /// </summary>
    public bool Enabled { get; set; } = true;

    /// <summary>
    /// Gets or sets the storage backend name. The platform
    /// recognises <c>"memory"</c>; consumers may register their own
    /// stores when the value is anything else. Defaults to
    /// <c>"memory"</c>.
    /// </summary>
    public string Storage { get; set; } = "memory";

    /// <summary>
    /// Gets or sets the documented retention window in seconds. A
    /// record whose <see cref="IdempotencyRecord.CreatedAt"/> is
    /// older than the window is removed by
    /// <see cref="IIdempotencyStore.EvictExpiredAsync"/>. Defaults
    /// to <c>86400</c> (24 hours).
    /// </summary>
    public int RetentionSeconds { get; set; } = 86400;

    /// <summary>
    /// Gets or sets the documented maximum idempotency key length.
    /// Keys longer than this value are rejected by
    /// <see cref="IIdempotencyStore.SaveAsync"/>. Defaults to
    /// <c>256</c>.
    /// </summary>
    public int MaxKeyLength { get; set; } = 256;

    /// <summary>
    /// Gets or sets the documented header name the platform ASP.NET
    /// Core integration reads to extract the idempotency key.
    /// Defaults to <c>Idempotency-Key</c>.
    /// </summary>
    public string HeaderName { get; set; } = "Idempotency-Key";
}
