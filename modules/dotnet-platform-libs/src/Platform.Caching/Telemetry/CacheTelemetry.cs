using System.Diagnostics;
using System.Diagnostics.Metrics;

namespace Platform.Caching.Telemetry;

/// <summary>Stable, redaction-safe cache telemetry names.</summary>
public static class CacheTelemetry
{
    /// <summary>Activity source name.</summary>
    public const string ActivitySourceName = "platform.cache";
    /// <summary>Get-or-create operation name.</summary>
    public const string GetOrCreateOperation = "cache.get_or_create";
    /// <summary>Set operation name.</summary>
    public const string SetOperation = "cache.set";
    /// <summary>Remove operation name.</summary>
    public const string RemoveOperation = "cache.remove";
    /// <summary>Tag invalidation operation name.</summary>
    public const string RemoveByTagOperation = "cache.remove_by_tag";
    /// <summary>Activity source.</summary>
    public static readonly ActivitySource ActivitySource = new(ActivitySourceName);
    /// <summary>Meter.</summary>
    public static readonly Meter Meter = new(ActivitySourceName);
    /// <summary>Cache hit counter.</summary>
    public static readonly Counter<long> Hits = Meter.CreateCounter<long>("platform.cache.hits");
    /// <summary>Cache miss counter.</summary>
    public static readonly Counter<long> Misses = Meter.CreateCounter<long>("platform.cache.misses");
    /// <summary>Provider failure counter.</summary>
    public static readonly Counter<long> Failures = Meter.CreateCounter<long>("platform.cache.failures");
}
