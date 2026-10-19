namespace Platform.Web;

/// <summary>Configures the opt-in platform web runtime.</summary>
public sealed class PlatformWebOptions
{
    /// <summary>The default maximum request body size.</summary>
    public const long DefaultMaxRequestBodyBytes = 1_048_576;

    /// <summary>Gets or sets the correlation header.</summary>
    public string CorrelationHeader { get; set; } = "X-Correlation-Id";

    /// <summary>Gets or sets whether trusted hosts may supply correlation IDs.</summary>
    public bool AcceptIncomingCorrelationHeader { get; set; }

    /// <summary>Gets or sets the maximum accepted correlation identifier length.</summary>
    public int MaxCorrelationIdLength { get; set; } = 128;

    /// <summary>Gets or sets whether security headers are emitted.</summary>
    public bool EnableSecurityHeaders { get; set; } = true;

    /// <summary>Gets or sets the maximum request body size in bytes.</summary>
    public long MaxRequestBodyBytes { get; set; } = DefaultMaxRequestBodyBytes;

    /// <summary>Gets or sets the request cancellation timeout.</summary>
    public TimeSpan RequestTimeout { get; set; } = TimeSpan.FromSeconds(30);

    /// <summary>Gets or sets the liveness endpoint path.</summary>
    public string LivePath { get; set; } = "/live";

    /// <summary>Gets or sets the readiness endpoint path.</summary>
    public string ReadinessPath { get; set; } = "/ready";

    /// <summary>Validates option values and returns human-readable failures.</summary>
    public IReadOnlyList<string> Validate()
    {
        var errors = new List<string>();
        if (string.IsNullOrWhiteSpace(CorrelationHeader)) errors.Add("CorrelationHeader is required.");
        if (MaxCorrelationIdLength is < 16 or > 1024) errors.Add("MaxCorrelationIdLength must be between 16 and 1024.");
        if (MaxRequestBodyBytes <= 0) errors.Add("MaxRequestBodyBytes must be positive.");
        if (RequestTimeout <= TimeSpan.Zero) errors.Add("RequestTimeout must be positive.");
        if (!IsPath(LivePath)) errors.Add("LivePath must be an absolute application path.");
        if (!IsPath(ReadinessPath)) errors.Add("ReadinessPath must be an absolute application path.");
        if (string.Equals(LivePath, ReadinessPath, StringComparison.OrdinalIgnoreCase)) errors.Add("LivePath and ReadinessPath must differ.");
        return errors;
    }

    private static bool IsPath(string? value) => !string.IsNullOrWhiteSpace(value) && value.StartsWith('/');
}
