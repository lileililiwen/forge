using Platform.Observability;

namespace Platform.Observability.Redaction;

/// <summary>Redacts a value into a safe representation. Implementations must never return a secret verbatim.</summary>
public interface IPlatformObservabilityRedactor
{
    /// <summary>Returns a safe representation of a value.</summary>
    string Redact(string? value);
}

/// <summary>Default redactor that always returns a constant placeholder.</summary>
public sealed class DefaultPlatformObservabilityRedactor : IPlatformObservabilityRedactor
{
    /// <inheritdoc />
    public string Redact(string? value) => string.IsNullOrEmpty(value) ? string.Empty : "[REDACTED]";
}

/// <summary>Describes a redaction-safe value policy applied to every tag, operation, and provider name.</summary>
public sealed class PlatformObservabilitySafeValuePolicy
{
    private readonly IPlatformObservabilityRedactor _redactor;
    private readonly int _maxTagLength;
    private readonly int _maxOperationLength;
    private readonly bool _truncateOversizedValues;

    /// <summary>Initializes a new instance of the <see cref="PlatformObservabilitySafeValuePolicy"/> class.</summary>
    public PlatformObservabilitySafeValuePolicy(IPlatformObservabilityRedactor redactor, int maxTagLength, int maxOperationLength, bool truncateOversizedValues)
    {
        ArgumentNullException.ThrowIfNull(redactor);
        _redactor = redactor;
        _maxTagLength = maxTagLength > 0 ? maxTagLength : PlatformObservabilityOptions.DefaultMaxTagLength;
        _maxOperationLength = maxOperationLength > 0 ? maxOperationLength : PlatformObservabilityOptions.DefaultMaxOperationLength;
        _truncateOversizedValues = truncateOversizedValues;
    }

    /// <summary>Redacts a free-form value (e.g. authorization header, account id, response body preview) and bounds it to the configured tag length.</summary>
    public string RedactTag(string? value) => Bound(_redactor.Redact(value), _maxTagLength);

    /// <summary>Bounds a tag value without redacting it. Use for already-safe values such as the request method or route template.</summary>
    public string BoundTag(string? value) => Bound(value ?? string.Empty, _maxTagLength);

    /// <summary>Redacts an operation name and bounds it to the configured operation length.</summary>
    public string RedactOperation(string? value) => Bound(_redactor.Redact(value), _maxOperationLength);

    /// <summary>Validates an operation name (non-empty, bounded, no whitespace).</summary>
    public string RequireOperation(string? value)
    {
        if (string.IsNullOrWhiteSpace(value)) throw new ArgumentException("Operation is required.", nameof(value));
        return Bound(value.Trim(), _maxOperationLength);
    }

    /// <summary>Validates a correlation identifier (non-empty, bounded, no whitespace).</summary>
    public string RequireCorrelationId(string? value)
    {
        if (string.IsNullOrWhiteSpace(value)) throw new ArgumentException("Correlation identifier is required.", nameof(value));
        var trimmed = value.Trim();
        return trimmed.Length <= _maxTagLength ? trimmed : trimmed[.._maxTagLength];
    }

    private string Bound(string value, int limit)
    {
        if (string.IsNullOrEmpty(value)) return value;
        if (value.Length <= limit) return value;
        if (!_truncateOversizedValues) return string.Empty;
        return value[..limit];
    }
}
