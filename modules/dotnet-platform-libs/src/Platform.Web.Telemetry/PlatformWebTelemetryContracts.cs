namespace Platform.Web.Telemetry;

/// <summary>Redacts a value into a safe representation. Implementations must never return a secret verbatim.</summary>
public interface IPlatformWebTelemetryRedactor
{
    /// <summary>Returns a safe representation of a value.</summary>
    string Redact(string? value);
}

/// <summary>Default redactor that always returns a constant placeholder.</summary>
public sealed class DefaultPlatformWebTelemetryRedactor : IPlatformWebTelemetryRedactor
{
    /// <inheritdoc />
    public string Redact(string? value) => string.IsNullOrEmpty(value) ? string.Empty : "[REDACTED]";
}

/// <summary>Represents a single, redacted request event for instrumentation.</summary>
public sealed record PlatformWebRequestEvent(string Operation, string Method, string Route, int Status, TimeSpan Duration, string? ErrorCode = null);

/// <summary>Represents a single, redacted provider call event for instrumentation.</summary>
public sealed record PlatformWebProviderEvent(string Operation, string Provider, int Status, TimeSpan Duration, int Attempt, string? ErrorCode = null);

/// <summary>Describes a redaction-safe value policy applied to all instrumented values.</summary>
public sealed class PlatformWebTelemetrySafeValuePolicy
{
    private readonly IPlatformWebTelemetryRedactor _redactor;
    private readonly int _maxTagLength;
    private readonly int _maxOperationLength;
    private readonly bool _truncateOversizedValues;

    /// <summary>Initializes a new instance of the <see cref="PlatformWebTelemetrySafeValuePolicy"/> class.</summary>
    public PlatformWebTelemetrySafeValuePolicy(IPlatformWebTelemetryRedactor redactor, int maxTagLength, int maxOperationLength, bool truncateOversizedValues)
    {
        _redactor = redactor ?? throw new ArgumentNullException(nameof(redactor));
        _maxTagLength = maxTagLength > 0 ? maxTagLength : PlatformWebTelemetryOptions.DefaultMaxTagLength;
        _maxOperationLength = maxOperationLength > 0 ? maxOperationLength : PlatformWebTelemetryOptions.DefaultMaxOperationLength;
        _truncateOversizedValues = truncateOversizedValues;
    }

    /// <summary>Redacts a free-form value and bounds it to the configured tag length.</summary>
    public string RedactTag(string? value) => Bound(_redactor.Redact(value), _maxTagLength);

    /// <summary>Redacts an operation name and bounds it to the configured operation length.</summary>
    public string RedactOperation(string? value) => Bound(_redactor.Redact(value), _maxOperationLength);

    /// <summary>Validates an operation name (non-empty, bounded, no whitespace).</summary>
    public string RequireOperation(string? value)
    {
        if (string.IsNullOrWhiteSpace(value)) throw new ArgumentException("Operation is required.", nameof(value));
        return Bound(value.Trim(), _maxOperationLength);
    }

    private string Bound(string value, int limit)
    {
        if (string.IsNullOrEmpty(value)) return value;
        if (value.Length <= limit) return value;
        if (!_truncateOversizedValues) return string.Empty;
        return value[..limit];
    }
}

/// <summary>Structured, redaction-safe log sink shared by the platform web-edge packages.</summary>
/// <remarks>The helper never logs request bodies, response bodies, authorization headers, cookies, or unrestricted query strings. Tag values are bounded and validated against a configured redactor.</remarks>
public interface IPlatformWebTelemetry
{
    /// <summary>Records a stable request event.</summary>
    void RecordRequest(PlatformWebRequestEvent request);

    /// <summary>Records a stable provider call event.</summary>
    void RecordProvider(PlatformWebProviderEvent provider);

    /// <summary>Returns the currently configured safe value policy.</summary>
    PlatformWebTelemetrySafeValuePolicy SafeValuePolicy { get; }
}
