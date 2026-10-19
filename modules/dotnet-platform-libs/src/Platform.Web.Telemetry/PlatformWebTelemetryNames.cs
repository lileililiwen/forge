namespace Platform.Web.Telemetry;

/// <summary>Stable, framework-neutral names for platform web instrumentation.</summary>
/// <remarks>Names are intentionally opaque; downstream exporters and dashboards rely on them. Do not rename existing entries without coordinating with consumers.</remarks>
public static class PlatformWebTelemetryNames
{
    /// <summary>Activity source for incoming HTTP requests handled by the platform runtime.</summary>
    public const string RequestActivitySource = "Platform.Web.Requests";

    /// <summary>Activity source for outbound provider calls (HTTP, storage, mail, AI, billing, etc.).</summary>
    public const string ProviderActivitySource = "Platform.Web.Providers";

    /// <summary>Operation name used for correlation when the platform logs an incoming request.</summary>
    public const string RequestOperation = "platform.web.request";

    /// <summary>Operation name used for correlation when the platform logs an outbound provider call.</summary>
    public const string ProviderOperation = "platform.web.provider";

    /// <summary>Operation name used when the platform logs a CORS policy decision.</summary>
    public const string CorsOperation = "platform.web.cors";

    /// <summary>Operation name used when the platform logs a resilience retry/timeout/breaker decision.</summary>
    public const string ResilienceOperation = "platform.web.resilience";

    /// <summary>Operation name used when the platform logs an OpenAPI document map.</summary>
    public const string OpenApiOperation = "platform.web.openapi";

    /// <summary>Counter metric name for handled requests.</summary>
    public const string RequestCountMetric = "platform.web.request.count";

    /// <summary>Histogram metric name for request duration in milliseconds.</summary>
    public const string RequestDurationMetric = "platform.web.request.duration";

    /// <summary>Counter metric name for provider call outcomes.</summary>
    public const string ProviderCountMetric = "platform.web.provider.count";

    /// <summary>Histogram metric name for provider call duration in milliseconds.</summary>
    public const string ProviderDurationMetric = "platform.web.provider.duration";

    /// <summary>Tag key for a stable provider name (never includes the secret, account id, or endpoint).</summary>
    public const string TagProvider = "platform.provider";

    /// <summary>Tag key for a stable operation name (verb + logical action, never includes raw URLs with secrets).</summary>
    public const string TagOperation = "platform.operation";

    /// <summary>Tag key for the stable request route template.</summary>
    public const string TagRoute = "platform.route";

    /// <summary>Tag key for the request method (GET, POST, ...).</summary>
    public const string TagMethod = "platform.method";

    /// <summary>Tag key for the HTTP status code returned.</summary>
    public const string TagStatus = "platform.status";

    /// <summary>Tag key for a safe, bounded error code (no stack trace, no message body, no exception type).</summary>
    public const string TagErrorCode = "platform.error_code";

    /// <summary>Tag key for the platform retry attempt number (1-based).</summary>
    public const string TagAttempt = "platform.attempt";
}
