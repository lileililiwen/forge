using Microsoft.Extensions.Options;
using Platform.Observability.Redaction;

namespace Platform.Observability.Diagnostics;

/// <summary>Describes the safe availability of an observability provider. Implementations must never include secrets, account ids, or raw response bodies.</summary>
public sealed record PlatformObservabilityProviderStatus(string Name, bool Available, string? Detail = null);

/// <summary>Supplies observability provider health without requiring a specific exporter package.</summary>
public interface IPlatformObservabilityProviderStatusSource
{
    /// <summary>Gets the current set of observability provider statuses.</summary>
    IReadOnlyList<PlatformObservabilityProviderStatus> GetStatuses();
}

/// <summary>Default <see cref="IPlatformObservabilityProviderStatusSource"/> that reports the platform host sources and meters as available when at least one listener is attached.</summary>
public sealed class DefaultPlatformObservabilityProviderStatusSource : IPlatformObservabilityProviderStatusSource
{
    /// <inheritdoc />
    public IReadOnlyList<PlatformObservabilityProviderStatus> GetStatuses() => new[]
    {
        new PlatformObservabilityProviderStatus(PlatformObservabilityNames.HostActivitySource, Available: true, Detail: "framework"),
        new PlatformObservabilityProviderStatus(PlatformObservabilityNames.RequestActivitySource, Available: true, Detail: "framework"),
        new PlatformObservabilityProviderStatus(PlatformObservabilityNames.ProviderActivitySource, Available: true, Detail: "framework"),
        new PlatformObservabilityProviderStatus(PlatformObservabilityNames.HostMeter, Available: true, Detail: "framework"),
    };
}

/// <summary>Safe outcome for a provider call enrichment. The platform never captures the raw response body or account id.</summary>
public sealed record PlatformObservabilityProviderCall(
    string Operation,
    string Provider,
    string Outcome,
    TimeSpan Duration,
    string? ErrorCode = null);

/// <summary>Bridge that lets host code record safe provider call enrichment through the platform.</summary>
public interface IPlatformObservabilityProviderRecorder
{
    /// <summary>Records a single, redacted provider call event. The platform emits a stable activity and counter, never a raw response body.</summary>
    void RecordCall(PlatformObservabilityProviderCall providerCall);
}

/// <summary>Default <see cref="IPlatformObservabilityProviderRecorder"/> that delegates to the platform activity recorder.</summary>
public sealed class DefaultPlatformObservabilityProviderRecorder : IPlatformObservabilityProviderRecorder
{
    private readonly IPlatformActivityRecorder _activity;
    private readonly PlatformObservabilityOptions _options;
    private readonly PlatformObservabilitySafeValuePolicy _policy;

    /// <summary>Initializes a new instance of the <see cref="DefaultPlatformObservabilityProviderRecorder"/> class.</summary>
    public DefaultPlatformObservabilityProviderRecorder(IPlatformActivityRecorder activity, Microsoft.Extensions.Options.IOptions<PlatformObservabilityOptions> options, IPlatformObservabilityRedactor redactor)
    {
        ArgumentNullException.ThrowIfNull(activity);
        ArgumentNullException.ThrowIfNull(options);
        ArgumentNullException.ThrowIfNull(redactor);
        _activity = activity;
        _options = options.Value;
        _policy = new PlatformObservabilitySafeValuePolicy(redactor, _options.MaxTagLength, _options.MaxOperationLength, _options.TruncateOversizedValues);
    }

    /// <inheritdoc />
    public void RecordCall(PlatformObservabilityProviderCall providerCall)
    {
        ArgumentNullException.ThrowIfNull(providerCall);
        using var activity = _activity.StartProviderCall(providerCall.Operation, providerCall.Provider);
        var durationMs = providerCall.Duration.TotalMilliseconds;
        PlatformDiagnostics.SetLastProviderCallDuration(durationMs);
        PlatformDiagnostics.RecordProviderCall(providerCall.Outcome);
        _activity.Complete(activity, providerCall.Outcome, providerCall.ErrorCode, durationMs);
    }
}
