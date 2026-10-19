using System.Net;

namespace Platform.Web.Resilience;

/// <summary>Identifies the resilience policy stage an event is emitted from.</summary>
public enum HttpResilienceDecision
{
    /// <summary>The request was not retried.</summary>
    None = 0,
    /// <summary>The request was retried after a transient failure.</summary>
    Retried = 1,
    /// <summary>The request was cancelled because the per-attempt timeout elapsed.</summary>
    TimedOut = 2,
    /// <summary>The request was short-circuited because the circuit breaker is open.</summary>
    CircuitBroken = 3,
}

/// <summary>Records a single resilience event for telemetry consumers.</summary>
/// <param name="Decision">The decision that triggered the record.</param>
/// <param name="Operation">A stable operation name (verb + logical action). Never includes secrets or the target URL.</param>
/// <param name="Attempt">The current attempt number (1-based).</param>
/// <param name="StatusCode">The HTTP status code, or 0 for transport failures.</param>
public sealed record HttpResilienceEvent(HttpResilienceDecision Decision, string Operation, int Attempt, int StatusCode);
