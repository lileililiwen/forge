namespace Platform.Storage.Keys;

/// <summary>A validated portable object key.</summary>
public readonly record struct StorageObjectKey
{
    /// <summary>Creates a safe object key.</summary>
    public StorageObjectKey(string value)
    {
        if (string.IsNullOrWhiteSpace(value) || value.Any(char.IsControl)) throw new ArgumentException("Object keys must be non-empty and contain no control characters.", nameof(value));
        var normalized = value.Replace('\\', '/');
        if (normalized.StartsWith('/') || normalized.EndsWith('/') || normalized.Contains("//", StringComparison.Ordinal) || normalized.Split('/').Any(segment => segment is "." or ".."))
            throw new ArgumentException("Object keys must not contain traversal or empty path segments.", nameof(value));
        if (normalized.Length > 1024) throw new ArgumentOutOfRangeException(nameof(value));
        Value = normalized;
    }
    /// <summary>Key value.</summary>
    public string Value { get; }
    /// <inheritdoc />
    public override string ToString() => Value;
}
