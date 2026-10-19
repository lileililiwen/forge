namespace Platform.Observability;

/// <summary>Stable, framework-neutral names for platform host observability instrumentation.</summary>
/// <remarks>Names are intentionally opaque; downstream exporters and dashboards rely on them. Do not rename existing entries without coordinating with consumers. The platform never emits product or module specific meter names; consumers register their own.</remarks>
public static class PlatformObservabilityNames
{
    /// <summary>Activity source for host lifecycle and runtime diagnostics owned by the platform.</summary>
    public const string HostActivitySource = "Platform.Observability.Host";

    /// <summary>Activity source for incoming HTTP request enrichment performed by the platform.</summary>
    public const string RequestActivitySource = "Platform.Observability.Requests";

    /// <summary>Activity source for outbound HTTP and provider call enrichment performed by the platform.</summary>
    public const string ProviderActivitySource = "Platform.Observability.Providers";

    /// <summary>Meter name for host lifecycle and runtime metrics owned by the platform.</summary>
    public const string HostMeter = "Platform.Observability.Host";

    /// <summary>Operation name for host startup.</summary>
    public const string HostStartupOperation = "platform.observability.host.startup";

    /// <summary>Operation name for host shutdown.</summary>
    public const string HostShutdownOperation = "platform.observability.host.shutdown";

    /// <summary>Operation name for correlation enrichment on an incoming request.</summary>
    public const string RequestCorrelationOperation = "platform.observability.request.correlation";

    /// <summary>Operation name for an outbound provider call observed through the safe diagnostics pipeline.</summary>
    public const string ProviderCallOperation = "platform.observability.provider.call";

    /// <summary>Counter metric for host lifecycle transitions.</summary>
    public const string HostLifecycleCountMetric = "platform.observability.host.lifecycle.count";

    /// <summary>Histogram metric for host startup duration in milliseconds.</summary>
    public const string HostStartupDurationMetric = "platform.observability.host.startup.duration";

    /// <summary>Counter metric for safe provider call outcomes.</summary>
    public const string ProviderCallCountMetric = "platform.observability.provider.call.count";

    /// <summary>Histogram metric for safe provider call duration in milliseconds.</summary>
    public const string ProviderCallDurationMetric = "platform.observability.provider.call.duration";

    /// <summary>Tag key for the platform service identity (application name).</summary>
    public const string TagServiceName = "platform.service.name";

    /// <summary>Tag key for the platform service version.</summary>
    public const string TagServiceVersion = "platform.service.version";

    /// <summary>Tag key for the host environment name (Development, Production, ...).</summary>
    public const string TagEnvironment = "platform.environment";

    /// <summary>Tag key for the bounded correlation identifier.</summary>
    public const string TagCorrelationId = "platform.correlation_id";

    /// <summary>Tag key for the stable, redacted operation name.</summary>
    public const string TagOperation = "platform.operation";

    /// <summary>Tag key for the stable provider name (never the secret, account id, or endpoint).</summary>
    public const string TagProvider = "platform.provider";

    /// <summary>Tag key for the stable request route template.</summary>
    public const string TagRoute = "platform.route";

    /// <summary>Tag key for the request method (GET, POST, ...).</summary>
    public const string TagMethod = "platform.method";

    /// <summary>Tag key for a stable, bounded outcome label (success, transient, permanent, ...).</summary>
    public const string TagOutcome = "platform.outcome";

    /// <summary>Tag key for the safe, bounded error code (never the message body or exception detail).</summary>
    public const string TagErrorCode = "platform.error_code";

    /// <summary>Outcome label used when an operation completes without a recorded error.</summary>
    public const string OutcomeSuccess = "success";

    /// <summary>Outcome label used when an operation fails with a transient error that may be retried.</summary>
    public const string OutcomeTransient = "transient";

    /// <summary>Outcome label used when an operation fails with a permanent error.</summary>
    public const string OutcomePermanent = "permanent";

    /// <summary>Outcome label used when the platform could not record a precise outcome.</summary>
    public const string OutcomeUnknown = "unknown";
}
