namespace Platform.Web.Resilience;

/// <summary>Thrown when the platform HTTP circuit breaker is open.</summary>
public sealed class PlatformHttpCircuitOpenException : HttpRequestException
{
    /// <summary>The operation that was attempted.</summary>
    public string Operation { get; }

    /// <summary>Initializes a new instance of the <see cref="PlatformHttpCircuitOpenException"/> class.</summary>
    public PlatformHttpCircuitOpenException(string operation)
        : base($"The HTTP circuit breaker is open for operation '{operation}'.")
    {
        Operation = operation;
    }
}
