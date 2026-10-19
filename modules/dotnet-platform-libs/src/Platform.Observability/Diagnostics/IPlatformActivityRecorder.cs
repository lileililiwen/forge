using System.Diagnostics;
using Microsoft.Extensions.Options;
using Platform.Observability.Redaction;

namespace Platform.Observability.Diagnostics;

/// <summary>Records structured, redacted platform activities and metrics. Replaces ad-hoc <c>ILogger</c> use in the host layer.</summary>
public interface IPlatformActivityRecorder
{
    /// <summary>Starts a host lifecycle activity. Returns <c>null</c> when host lifecycle recording is disabled or the source has no listeners.</summary>
    Activity? StartHostLifecycle(string operation, string outcome = PlatformObservabilityNames.OutcomeSuccess);

    /// <summary>Starts a request correlation activity. Returns <c>null</c> when request enrichment is disabled or the source has no listeners.</summary>
    Activity? StartRequestCorrelation(string operation, string route, string method);

    /// <summary>Starts a provider call activity. Returns <c>null</c> when provider enrichment is disabled or the source has no listeners.</summary>
    Activity? StartProviderCall(string operation, string provider);

    /// <summary>Marks the activity as completed and records the outcome label plus the supplied error code (if any).</summary>
    void Complete(Activity? activity, string outcome, string? errorCode = null, double? durationMs = null);
}

/// <summary>Default <see cref="IPlatformActivityRecorder"/> that emits bounded, redacted tags through the platform's safe value policy.</summary>
public sealed class DefaultPlatformActivityRecorder : IPlatformActivityRecorder
{
    private readonly PlatformObservabilityOptions _options;
    private readonly PlatformObservabilitySafeValuePolicy _policy;
    private readonly bool _enableHostLifecycle;
    private readonly bool _enableRequestEnrichment;
    private readonly bool _enableProviderEnrichment;

    /// <summary>Initializes a new instance of the <see cref="DefaultPlatformActivityRecorder"/> class.</summary>
    public DefaultPlatformActivityRecorder(IOptions<PlatformObservabilityOptions> options, IPlatformObservabilityRedactor redactor)
    {
        ArgumentNullException.ThrowIfNull(options);
        ArgumentNullException.ThrowIfNull(redactor);
        _options = options.Value;
        _policy = new PlatformObservabilitySafeValuePolicy(redactor, _options.MaxTagLength, _options.MaxOperationLength, _options.TruncateOversizedValues);
        _enableHostLifecycle = _options.EnableHostLifecycle;
        _enableRequestEnrichment = _options.EnableRequestEnrichment;
        _enableProviderEnrichment = _options.EnableProviderEnrichment;
    }

    /// <inheritdoc />
    public Activity? StartHostLifecycle(string operation, string outcome = PlatformObservabilityNames.OutcomeSuccess)
    {
        if (!_enableHostLifecycle) return null;
        var activity = PlatformDiagnostics.HostSource.StartActivity(_policy.RequireOperation(operation));
        if (activity is null) return null;
        activity.SetTag(PlatformObservabilityNames.TagServiceName, _policy.BoundTag(_options.ApplicationName));
        activity.SetTag(PlatformObservabilityNames.TagServiceVersion, _policy.BoundTag(_options.ApplicationVersion ?? string.Empty));
        if (!string.IsNullOrEmpty(_options.EnvironmentName))
            activity.SetTag(PlatformObservabilityNames.TagEnvironment, _policy.BoundTag(_options.EnvironmentName));
        activity.SetTag(PlatformObservabilityNames.TagOutcome, _policy.BoundTag(outcome));
        return activity;
    }

    /// <inheritdoc />
    public Activity? StartRequestCorrelation(string operation, string route, string method)
    {
        if (!_enableRequestEnrichment) return null;
        var activity = PlatformDiagnostics.RequestSource.StartActivity(_policy.RequireOperation(operation));
        if (activity is null) return null;
        activity.SetTag(PlatformObservabilityNames.TagServiceName, _policy.BoundTag(_options.ApplicationName));
        activity.SetTag(PlatformObservabilityNames.TagRoute, _policy.BoundTag(route));
        activity.SetTag(PlatformObservabilityNames.TagMethod, _policy.BoundTag(method));
        return activity;
    }

    /// <inheritdoc />
    public Activity? StartProviderCall(string operation, string provider)
    {
        if (!_enableProviderEnrichment) return null;
        var activity = PlatformDiagnostics.ProviderSource.StartActivity(_policy.RequireOperation(operation));
        if (activity is null) return null;
        activity.SetTag(PlatformObservabilityNames.TagServiceName, _policy.BoundTag(_options.ApplicationName));
        activity.SetTag(PlatformObservabilityNames.TagOperation, _policy.BoundTag(operation));
        activity.SetTag(PlatformObservabilityNames.TagProvider, _policy.RedactTag(provider));
        return activity;
    }

    /// <inheritdoc />
    public void Complete(Activity? activity, string outcome, string? errorCode = null, double? durationMs = null)
    {
        if (activity is null) return;
        activity.SetTag(PlatformObservabilityNames.TagOutcome, _policy.BoundTag(outcome));
        if (!string.IsNullOrEmpty(errorCode))
            activity.SetTag(PlatformObservabilityNames.TagErrorCode, _policy.RedactTag(errorCode));
        activity.SetStatus(string.Equals(outcome, PlatformObservabilityNames.OutcomeSuccess, StringComparison.Ordinal)
            ? ActivityStatusCode.Ok
            : ActivityStatusCode.Error);
        activity.Stop();
    }
}
