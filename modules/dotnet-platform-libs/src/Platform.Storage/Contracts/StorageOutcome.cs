namespace Platform.Storage.Contracts;

/// <summary>Safe outcome for a storage operation.</summary>
public sealed record StorageOutcome(StorageOutcomeStatus Status, StorageFailure? Failure = null)
{
    /// <summary>Creates a successful outcome.</summary>
    public static StorageOutcome Succeeded() => new(StorageOutcomeStatus.Succeeded);
    /// <summary>Creates a not-found outcome.</summary>
    public static StorageOutcome NotFound() => new(StorageOutcomeStatus.NotFound);
    /// <summary>Creates an unavailable outcome.</summary>
    public static StorageOutcome Unavailable(StorageFailure failure) => new(StorageOutcomeStatus.Unavailable, failure);
}

/// <summary>Storage operation states.</summary>
public enum StorageOutcomeStatus
{
    /// <summary>Operation completed.</summary>
    Succeeded,
    /// <summary>Object does not exist.</summary>
    NotFound,
    /// <summary>Provider rejected the operation.</summary>
    Rejected,
    /// <summary>Provider is unavailable.</summary>
    Unavailable
}

/// <summary>Redacted storage failure metadata.</summary>
public sealed record StorageFailure
{
    /// <summary>Creates validated safe failure metadata.</summary>
    public StorageFailure(string code, string message, bool transient)
    {
        if (string.IsNullOrWhiteSpace(code)) throw new ArgumentException("A failure code is required.", nameof(code));
        if (string.IsNullOrWhiteSpace(message)) throw new ArgumentException("A failure message is required.", nameof(message));
        Code = code; Message = message; Transient = transient;
    }
    /// <summary>Failure category code.</summary>
    public string Code { get; }
    /// <summary>Safe failure message.</summary>
    public string Message { get; }
    /// <summary>Whether retrying may succeed.</summary>
    public bool Transient { get; }
}
