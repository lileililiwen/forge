namespace Platform.Storage.Contracts;

/// <summary>Download result containing a provider-owned readable stream.</summary>
public sealed record StorageReadResult(Stream Content, StorageObjectMetadata Metadata) : IAsyncDisposable
{
    /// <inheritdoc />
    public ValueTask DisposeAsync() => Content.DisposeAsync();
}

/// <summary>Download outcome with an optional readable object.</summary>
public sealed record StorageDownloadResult(StorageOutcomeStatus Status, StorageReadResult? Value = null, StorageFailure? Failure = null)
{
    /// <summary>Creates a successful download result.</summary>
    public static StorageDownloadResult Succeeded(StorageReadResult value) => new(StorageOutcomeStatus.Succeeded, value);
    /// <summary>Creates a not-found result.</summary>
    public static StorageDownloadResult NotFound() => new(StorageOutcomeStatus.NotFound);
    /// <summary>Creates an unavailable result.</summary>
    public static StorageDownloadResult Unavailable(StorageFailure failure) => new(StorageOutcomeStatus.Unavailable, null, failure);
}
