namespace Platform.Caching.Keys;

/// <summary>A validated physical cache key.</summary>
public readonly record struct CacheKey
{
    /// <summary>Creates a validated key.</summary>
    public CacheKey(string value)
    {
        if (string.IsNullOrWhiteSpace(value) || value.Any(char.IsWhiteSpace)) throw new ArgumentException("Cache keys must be non-empty and whitespace-free.", nameof(value));
        if (value.Length > 1024) throw new ArgumentOutOfRangeException(nameof(value));
        Value = value;
    }

    /// <summary>The physical key value.</summary>
    public string Value { get; }
    /// <inheritdoc />
    public override string ToString() => Value;
}
