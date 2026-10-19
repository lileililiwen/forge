namespace Platform.Caching.Contracts;

/// <summary>Safe provider health state.</summary>
public sealed record CacheProviderStatus(CacheProviderState State, string Provider, string? Code = null)
{
    /// <summary>Creates a healthy status.</summary>
    public static CacheProviderStatus Healthy(string provider) => new(CacheProviderState.Healthy, provider);

    /// <summary>Creates an unavailable status.</summary>
    public static CacheProviderStatus Unavailable(string provider, string code) => new(CacheProviderState.Unavailable, provider, code);
}

/// <summary>Provider availability states.</summary>
public enum CacheProviderState
{
    /// <summary>Provider operations are expected to succeed.</summary>
    Healthy,
    /// <summary>Provider operations may fail intermittently.</summary>
    Degraded,
    /// <summary>Provider operations are unavailable.</summary>
    Unavailable
}

/// <summary>Exposes provider health without exposing connection details.</summary>
public interface ICacheProviderStatus
{
    /// <summary>Returns the current safe provider status.</summary>
    CacheProviderStatus Status { get; }
}
