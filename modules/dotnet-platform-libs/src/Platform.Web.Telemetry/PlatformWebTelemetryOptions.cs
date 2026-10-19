namespace Platform.Web.Telemetry;

/// <summary>Configurable redaction-safe behavior for the platform web telemetry helpers.</summary>
public sealed class PlatformWebTelemetryOptions
{
    /// <summary>The default maximum length of a tag value that may be recorded.</summary>
    public const int DefaultMaxTagLength = 128;

    /// <summary>The default maximum length of an operation name that may be recorded.</summary>
    public const int DefaultMaxOperationLength = 64;

    /// <summary>Gets or sets the application name that is attached to every recorded event.</summary>
    public string ApplicationName { get; set; } = "platform.web";

    /// <summary>Gets or sets the maximum allowed length of a recorded tag value.</summary>
    public int MaxTagLength { get; set; } = DefaultMaxTagLength;

    /// <summary>Gets or sets the maximum allowed length of a recorded operation name.</summary>
    public int MaxOperationLength { get; set; } = DefaultMaxOperationLength;

    /// <summary>Gets or sets whether operations longer than the configured cap are truncated instead of dropped.</summary>
    public bool TruncateOversizedValues { get; set; } = true;

    /// <summary>Validates option values and returns human-readable failures.</summary>
    public IReadOnlyList<string> Validate()
    {
        var errors = new List<string>();
        if (string.IsNullOrWhiteSpace(ApplicationName))
            errors.Add("ApplicationName is required.");
        if (MaxTagLength is < 16 or > 1024)
            errors.Add("MaxTagLength must be between 16 and 1024.");
        if (MaxOperationLength is < 8 or > 256)
            errors.Add("MaxOperationLength must be between 8 and 256.");
        return errors;
    }
}
