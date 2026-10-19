namespace Platform.Caching.Contracts;

/// <summary>Read outcome for a cache operation.</summary>
public sealed record CacheReadResult<T>(CacheReadStatus Status, T? Value = default, CacheFailure? Failure = null)
{
}

/// <summary>Read states exposed by a cache store.</summary>
public enum CacheReadStatus
{
    /// <summary>A value was found.</summary>
    Hit,
    /// <summary>No value was found.</summary>
    Miss,
    /// <summary>The provider could not complete the read.</summary>
    Unavailable
}
