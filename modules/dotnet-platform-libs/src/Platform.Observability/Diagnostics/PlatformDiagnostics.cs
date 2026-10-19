using System.Diagnostics;
using System.Diagnostics.Metrics;

namespace Platform.Observability.Diagnostics;

/// <summary>Centralized, framework-owned <see cref="ActivitySource"/>s and <see cref="Meter"/>s with stable names.</summary>
/// <remarks>The platform registers its own sources and meters. Consumers add their own exporters (OpenTelemetry, OTLP, console, ...). The platform never emits product or module specific meter names; consumers register their own.</remarks>
public static class PlatformDiagnostics
{
    private static readonly ActivitySource HostSourceInstance = new(PlatformObservabilityNames.HostActivitySource);
    private static readonly ActivitySource RequestSourceInstance = new(PlatformObservabilityNames.RequestActivitySource);
    private static readonly ActivitySource ProviderSourceInstance = new(PlatformObservabilityNames.ProviderActivitySource);
    private static readonly Meter HostMeterInstance = new(PlatformObservabilityNames.HostMeter);
    private static long _lifecycleCount;
    private static long _providerCallCount;
    private static double _lastStartupDurationMs;
    private static double _lastShutdownDurationMs;
    private static double _lastProviderCallDurationMs;

    /// <summary>Activity source for host lifecycle events (startup, shutdown, ...).</summary>
    public static ActivitySource HostSource => HostSourceInstance;

    /// <summary>Activity source for incoming request enrichment events.</summary>
    public static ActivitySource RequestSource => RequestSourceInstance;

    /// <summary>Activity source for outbound provider call enrichment events.</summary>
    public static ActivitySource ProviderSource => ProviderSourceInstance;

    /// <summary>Meter for host observability counters and histograms.</summary>
    public static Meter HostMeter => HostMeterInstance;

    /// <summary>Increments the host lifecycle counter for the supplied outcome and returns the new value.</summary>
    public static long RecordLifecycle(string outcome)
    {
        ArgumentException.ThrowIfNullOrEmpty(outcome);
        return Interlocked.Increment(ref _lifecycleCount);
    }

    /// <summary>Increments the provider call counter and returns the new value.</summary>
    public static long RecordProviderCall(string outcome)
    {
        ArgumentException.ThrowIfNullOrEmpty(outcome);
        return Interlocked.Increment(ref _providerCallCount);
    }

    /// <summary>Records the most recent host startup duration in milliseconds.</summary>
    public static double LastStartupDurationMs => Volatile.Read(ref _lastStartupDurationMs);

    /// <summary>Records the most recent host shutdown duration in milliseconds.</summary>
    public static double LastShutdownDurationMs => Volatile.Read(ref _lastShutdownDurationMs);

    /// <summary>Records the most recent provider call duration in milliseconds.</summary>
    public static double LastProviderCallDurationMs => Volatile.Read(ref _lastProviderCallDurationMs);

    /// <summary>Updates the most recent host startup duration in milliseconds.</summary>
    public static void SetLastStartupDuration(double milliseconds) => Volatile.Write(ref _lastStartupDurationMs, milliseconds);

    /// <summary>Updates the most recent host shutdown duration in milliseconds.</summary>
    public static void SetLastShutdownDuration(double milliseconds) => Volatile.Write(ref _lastShutdownDurationMs, milliseconds);

    /// <summary>Updates the most recent provider call duration in milliseconds.</summary>
    public static void SetLastProviderCallDuration(double milliseconds) => Volatile.Write(ref _lastProviderCallDurationMs, milliseconds);
}
