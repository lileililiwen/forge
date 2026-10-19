namespace Platform.Http.Resilience;

/// <summary>Identifies the resilience policy stage that produced an event.</summary>
public enum PlatformHttpResilienceDecision
{
    /// <summary>No resilience decision was recorded for the attempt.</summary>
    None = 0,

    /// <summary>The request was retried after a transient failure.</summary>
    Retried = 1,

    /// <summary>The request was short-circuited because the circuit breaker is open.</summary>
    CircuitOpen = 2,

    /// <summary>The request was rejected because the concurrency limiter queue was full.</summary>
    ConcurrencyRejected = 3,

    /// <summary>The request was cancelled by the caller and was not retried.</summary>
    Cancelled = 4,
}

/// <summary>Records a single bounded resilience event for telemetry consumers.</summary>
/// <param name="Decision">The decision that triggered the record.</param>
/// <param name="Operation">A stable operation name (verb + logical action). Never includes secrets, headers, or the target URL.</param>
/// <param name="Attempt">The current attempt number (1-based).</param>
/// <param name="StatusCode">The HTTP status code, or 0 for transport failures.</param>
public sealed record PlatformHttpResilienceEvent(
    PlatformHttpResilienceDecision Decision,
    string Operation,
    int Attempt,
    int StatusCode);

/// <summary>Receives bounded resilience telemetry without ever observing request bodies or authorization values.</summary>
public interface IPlatformHttpResilienceTelemetry
{
    /// <summary>Records a single resilience event.</summary>
    /// <param name="resilienceEvent">The bounded event to record.</param>
    void Record(PlatformHttpResilienceEvent resilienceEvent);
}

/// <summary>Default no-op telemetry sink. Replace with an application-owned sink (for example one forwarding to the platform observability meter).</summary>
public sealed class DefaultPlatformHttpResilienceTelemetry : IPlatformHttpResilienceTelemetry
{
    /// <inheritdoc/>
    public void Record(PlatformHttpResilienceEvent resilienceEvent) { }
}
