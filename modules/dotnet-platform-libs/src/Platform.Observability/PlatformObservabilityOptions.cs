namespace Platform.Observability;

/// <summary>Configures opt-in host observability registration.</summary>
public sealed class PlatformObservabilityOptions
{
    /// <summary>The default maximum length of a tag value emitted by the platform.</summary>
    public const int DefaultMaxTagLength = 128;

    /// <summary>The default maximum length of an operation name emitted by the platform.</summary>
    public const int DefaultMaxOperationLength = 64;

    /// <summary>The default maximum length of a correlation identifier accepted from the environment.</summary>
    public const int DefaultMaxCorrelationIdLength = 128;

    /// <summary>Gets or sets the application name attached to every activity and metric. Required.</summary>
    public string ApplicationName { get; set; } = "platform.host";

    /// <summary>Gets or sets the application version attached to every activity and metric. Optional.</summary>
    public string? ApplicationVersion { get; set; }

    /// <summary>Gets or sets the host environment name. Optional; the registered <c>IHostEnvironment</c> overrides this when present.</summary>
    public string? EnvironmentName { get; set; }

    /// <summary>Gets or sets the header used to read an incoming correlation identifier. The platform echoes the value on outgoing logs and never logs the raw header value verbatim.</summary>
    public string CorrelationHeader { get; set; } = "X-Correlation-Id";

    /// <summary>Gets or sets whether the host accepts correlation identifiers supplied by incoming requests. When <c>false</c> the platform always generates a new identifier.</summary>
    public bool AcceptIncomingCorrelationHeader { get; set; }

    /// <summary>Gets or sets the maximum length of an accepted correlation identifier.</summary>
    public int MaxCorrelationIdLength { get; set; } = DefaultMaxCorrelationIdLength;

    /// <summary>Gets or sets the maximum length of an emitted tag value.</summary>
    public int MaxTagLength { get; set; } = DefaultMaxTagLength;

    /// <summary>Gets or sets the maximum length of an emitted operation name.</summary>
    public int MaxOperationLength { get; set; } = DefaultMaxOperationLength;

    /// <summary>Gets or sets whether tags longer than the configured cap are truncated instead of dropped.</summary>
    public bool TruncateOversizedValues { get; set; } = true;

    /// <summary>Gets or sets whether the platform records ASP.NET Core request enrichment activities. When <c>false</c> the request middleware is a no-op.</summary>
    public bool EnableRequestEnrichment { get; set; } = true;

    /// <summary>Gets or sets whether the platform records outbound provider call enrichment activities.</summary>
    public bool EnableProviderEnrichment { get; set; } = true;

    /// <summary>Gets or sets whether host lifecycle activities and metrics are recorded.</summary>
    public bool EnableHostLifecycle { get; set; } = true;

    /// <summary>Gets or sets whether a redacted request body preview is recorded in the request enrichment scope. The platform only ever stores a bounded, redacted preview, never the raw body.</summary>
    public bool RecordRequestBodyPreview { get; set; }

    /// <summary>Validates option values and returns human-readable failures.</summary>
    public IReadOnlyList<string> Validate()
    {
        var errors = new List<string>();
        if (string.IsNullOrWhiteSpace(ApplicationName)) errors.Add("ApplicationName is required.");
        if (string.IsNullOrWhiteSpace(CorrelationHeader)) errors.Add("CorrelationHeader is required.");
        if (MaxCorrelationIdLength is < 16 or > 1024) errors.Add("MaxCorrelationIdLength must be between 16 and 1024.");
        if (MaxTagLength is < 16 or > 1024) errors.Add("MaxTagLength must be between 16 and 1024.");
        if (MaxOperationLength is < 16 or > 256) errors.Add("MaxOperationLength must be between 16 and 256.");
        return errors;
    }
}
