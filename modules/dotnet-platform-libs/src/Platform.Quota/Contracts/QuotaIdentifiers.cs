namespace Platform.Quota.Contracts;

/// <summary>Opaque subject identifier supplied by an application.</summary>
public readonly record struct QuotaSubject
{
    /// <summary>Creates a validated subject.</summary>
    public QuotaSubject(string value)
    {
        if (string.IsNullOrWhiteSpace(value) || value.Any(char.IsControl) || value.Length > 256) throw new ArgumentException("Quota subjects must be non-empty, bounded, and contain no control characters.", nameof(value));
        Value = value;
    }
    /// <summary>Subject value.</summary>
    public string Value { get; }
    /// <inheritdoc />
    public override string ToString() => Value;
}

/// <summary>Opaque application-defined quota resource.</summary>
public readonly record struct QuotaResource
{
    /// <summary>Creates a validated resource.</summary>
    public QuotaResource(string value)
    {
        if (string.IsNullOrWhiteSpace(value) || value.Any(char.IsWhiteSpace) || value.Any(char.IsControl) || value.Length > 128) throw new ArgumentException("Quota resources must be non-empty, bounded, and whitespace-free.", nameof(value));
        Value = value;
    }
    /// <summary>Resource value.</summary>
    public string Value { get; }
    /// <inheritdoc />
    public override string ToString() => Value;
}

/// <summary>Stable operation identifier used for idempotency.</summary>
public readonly record struct QuotaOperationKey
{
    /// <summary>Creates a validated operation key.</summary>
    public QuotaOperationKey(string value)
    {
        if (string.IsNullOrWhiteSpace(value) || value.Any(char.IsControl) || value.Length > 256) throw new ArgumentException("Operation keys must be non-empty, bounded, and contain no control characters.", nameof(value));
        Value = value;
    }
    /// <summary>Operation key value.</summary>
    public string Value { get; }
    /// <inheritdoc />
    public override string ToString() => Value;
}
