namespace Platform.Storage.Contracts;

/// <summary>Portable metadata returned for a stored object.</summary>
public sealed record StorageObjectMetadata
{
    /// <summary>Creates validated object metadata.</summary>
    public StorageObjectMetadata(long lengthBytes, string contentType, DateTimeOffset lastModified, string? eTag)
    {
        ArgumentOutOfRangeException.ThrowIfNegative(lengthBytes);
        if (string.IsNullOrWhiteSpace(contentType)) throw new ArgumentException("Content type is required.", nameof(contentType));
        LengthBytes = lengthBytes;
        ContentType = contentType;
        LastModified = lastModified;
        ETag = eTag;
    }

    /// <summary>Length in bytes.</summary>
    public long LengthBytes { get; }
    /// <summary>Content type.</summary>
    public string ContentType { get; }
    /// <summary>Last modification time.</summary>
    public DateTimeOffset LastModified { get; }
    /// <summary>Optional provider ETag.</summary>
    public string? ETag { get; }
}
