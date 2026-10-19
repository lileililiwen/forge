using Platform.Storage.Contracts;

namespace Platform.Storage.S3;

/// <summary>S3-compatible adapter configuration.</summary>
public sealed class S3StorageOptions
{
    /// <summary>Bucket name owned by the consuming application.</summary>
    public string BucketName { get; init; } = string.Empty;
    /// <summary>Optional key prefix for this application.</summary>
    public string KeyPrefix { get; init; } = string.Empty;
    /// <summary>Portable limits.</summary>
    public StorageOptions Limits { get; init; } = new();

    /// <summary>Validates configuration.</summary>
    public void Validate()
    {
        if (string.IsNullOrWhiteSpace(BucketName) || BucketName.Any(char.IsWhiteSpace)) throw new ArgumentException("A safe bucket name is required.", nameof(BucketName));
        if (KeyPrefix.Any(char.IsControl) || KeyPrefix.Any(char.IsWhiteSpace)) throw new ArgumentException("The key prefix must contain no whitespace or control characters.", nameof(KeyPrefix));
        Limits.Validate();
    }
}
