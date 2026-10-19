using Platform.Storage.Keys;

namespace Platform.Storage.Contracts;

/// <summary>Supported presigned operations.</summary>
public enum StorageOperation
{
    /// <summary>Upload operation.</summary>
    Upload,
    /// <summary>Download operation.</summary>
    Download
}

/// <summary>Validated upload request.</summary>
public sealed record StorageUploadRequest(StorageObjectKey Key, Stream Content, string ContentType, long ContentLength)
{
    /// <summary>Validates the request against storage limits.</summary>
    public void Validate(StorageOptions options)
    {
        ArgumentNullException.ThrowIfNull(options);
        options.Validate();
        if (!Content.CanRead) throw new ArgumentException("Upload content must be readable.");
        if (string.IsNullOrWhiteSpace(ContentType) || ContentType.Any(char.IsControl)) throw new ArgumentException("A safe content type is required.");
        if (ContentLength < 0 || ContentLength > options.MaximumObjectBytes) throw new ArgumentOutOfRangeException(nameof(options));
    }
}

/// <summary>Validated presigned operation request.</summary>
public sealed record PresignRequest(StorageObjectKey Key, StorageOperation Operation, TimeSpan Lifetime, string? ContentType = null, long? MaximumBytes = null)
{
    /// <summary>Validates request bounds.</summary>
    public void Validate(StorageOptions options)
    {
        ArgumentNullException.ThrowIfNull(options);
        options.Validate();
        if (Lifetime <= TimeSpan.Zero || Lifetime > options.MaximumPresignLifetime) throw new ArgumentOutOfRangeException(nameof(options));
        if (Operation == StorageOperation.Upload && (string.IsNullOrWhiteSpace(ContentType) || ContentType.Any(char.IsControl))) throw new ArgumentException("Upload presigns require a safe content type.");
        if (MaximumBytes is <= 0 || MaximumBytes > options.MaximumObjectBytes) throw new ArgumentOutOfRangeException(nameof(options));
    }
}

/// <summary>Presigned operation result.</summary>
public sealed record PresignedOperation(Uri Url, StorageObjectKey Key, StorageOperation Operation, DateTimeOffset ExpiresAt, string? ContentType, long? MaximumBytes);

/// <summary>Provider-neutral object storage boundary.</summary>
public interface IObjectStorage
{
    /// <summary>Uploads an object.</summary>
    Task<StorageOutcome> UploadAsync(StorageUploadRequest request, CancellationToken cancellationToken = default);
    /// <summary>Downloads an object or returns not found.</summary>
    Task<StorageDownloadResult> DownloadAsync(StorageObjectKey key, CancellationToken cancellationToken = default);
    /// <summary>Reads object metadata or returns not found.</summary>
    Task<StorageObjectMetadata?> GetMetadataAsync(StorageObjectKey key, CancellationToken cancellationToken = default);
    /// <summary>Deletes an object.</summary>
    Task<StorageOutcome> DeleteAsync(StorageObjectKey key, CancellationToken cancellationToken = default);
    /// <summary>Issues a bounded presigned operation.</summary>
    Task<(StorageOutcome Outcome, PresignedOperation? Operation)> PresignAsync(PresignRequest request, CancellationToken cancellationToken = default);
}
