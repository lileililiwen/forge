namespace Platform.Caching.Contracts;

/// <summary>Write or invalidation outcome.</summary>
public sealed record CacheOperationResult(CacheOperationStatus Status, CacheFailure? Failure = null)
{
    /// <summary>Creates a successful result.</summary>
    public static CacheOperationResult Succeeded() => new(CacheOperationStatus.Succeeded);

    /// <summary>Creates a safe unavailable result.</summary>
    public static CacheOperationResult Unavailable(CacheFailure failure) => new(CacheOperationStatus.Unavailable, failure);
}

/// <summary>Operation states exposed by a cache store.</summary>
public enum CacheOperationStatus
{
    /// <summary>The operation completed.</summary>
    Succeeded,
    /// <summary>The provider could not complete the operation.</summary>
    Unavailable
}

/// <summary>Safe provider failure metadata; it must not contain secrets or payloads.</summary>
public sealed record CacheFailure
{
    /// <summary>Creates validated failure metadata.</summary>
    public CacheFailure(string code, string message, bool transient)
    {
        if (string.IsNullOrWhiteSpace(code)) throw new ArgumentException("A failure code is required.", nameof(code));
        if (string.IsNullOrWhiteSpace(message)) throw new ArgumentException("A failure message is required.", nameof(message));
        Code = code;
        Message = message;
        Transient = transient;
    }

    /// <summary>Failure code.</summary>
    public string Code { get; }
    /// <summary>Safe failure message.</summary>
    public string Message { get; }
    /// <summary>Whether the provider may recover without configuration changes.</summary>
    public bool Transient { get; }
}
