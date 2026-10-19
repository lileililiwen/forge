namespace Platform.Web.Resilience;

/// <summary>Configures the platform HTTP resilience conventions.</summary>
/// <remarks>The defaults are conservative: two retry attempts, a five-second per-attempt timeout, a 30-second circuit-breaker sampling window, and retries restricted to safe (idempotent) methods.</remarks>
public sealed class PlatformHttpResilienceOptions
{
    /// <summary>The default per-attempt timeout.</summary>
    public static readonly TimeSpan DefaultAttemptTimeout = TimeSpan.FromSeconds(5);

    /// <summary>The default total retry attempts (including the initial call).</summary>
    public const int DefaultMaxRetryAttempts = 3;

    /// <summary>The default circuit-breaker failure ratio (0..1).</summary>
    public const double DefaultCircuitBreakerFailureRatio = 0.5;

    /// <summary>The default circuit-breaker sampling duration.</summary>
    public static readonly TimeSpan DefaultCircuitBreakerSamplingDuration = TimeSpan.FromSeconds(30);

    /// <summary>The default circuit-breaker minimum throughput required to open.</summary>
    public const int DefaultCircuitBreakerMinimumThroughput = 10;

    /// <summary>The default circuit-breaker break duration.</summary>
    public static readonly TimeSpan DefaultCircuitBreakerBreakDuration = TimeSpan.FromSeconds(30);

    /// <summary>Gets or sets the per-attempt timeout. The total time is bounded by <c>AttemptTimeout * (MaxRetryAttempts + 1)</c>.</summary>
    public TimeSpan AttemptTimeout { get; set; } = DefaultAttemptTimeout;

    /// <summary>Gets or sets the maximum number of retry attempts. Set to <c>1</c> to disable retries.</summary>
    public int MaxRetryAttempts { get; set; } = DefaultMaxRetryAttempts;

    /// <summary>Gets or sets the base back-off used between retries. The handler applies linear back-off up to this value.</summary>
    public TimeSpan RetryBaseDelay { get; set; } = TimeSpan.FromMilliseconds(200);

    /// <summary>Gets or sets the circuit-breaker failure ratio (0..1).</summary>
    public double CircuitBreakerFailureRatio { get; set; } = DefaultCircuitBreakerFailureRatio;

    /// <summary>Gets or sets the circuit-breaker sampling window.</summary>
    public TimeSpan CircuitBreakerSamplingDuration { get; set; } = DefaultCircuitBreakerSamplingDuration;

    /// <summary>Gets or sets the circuit-breaker minimum throughput required to consider opening.</summary>
    public int CircuitBreakerMinimumThroughput { get; set; } = DefaultCircuitBreakerMinimumThroughput;

    /// <summary>Gets or sets how long the circuit stays open once tripped.</summary>
    public TimeSpan CircuitBreakerBreakDuration { get; set; } = DefaultCircuitBreakerBreakDuration;

    /// <summary>Gets or sets the additional HTTP methods treated as idempotent for retry purposes. The default safe set is always included.</summary>
    public ISet<string> IdempotentMethods { get; set; } = new HashSet<string>(StringComparer.OrdinalIgnoreCase);

    /// <summary>Gets or sets whether retries are emitted with a stable <c>X-Retry-Attempt</c> header. Defaults to <c>true</c>.</summary>
    public bool EmitRetryAttemptHeader { get; set; } = true;

    /// <summary>Validates option values and returns human-readable failures.</summary>
    public IReadOnlyList<string> Validate()
    {
        var errors = new List<string>();
        if (AttemptTimeout <= TimeSpan.Zero || AttemptTimeout > TimeSpan.FromMinutes(5))
            errors.Add("AttemptTimeout must be between 1ms and 5 minutes.");
        if (MaxRetryAttempts < 1 || MaxRetryAttempts > 10)
            errors.Add("MaxRetryAttempts must be between 1 and 10.");
        if (RetryBaseDelay < TimeSpan.Zero || RetryBaseDelay > TimeSpan.FromSeconds(10))
            errors.Add("RetryBaseDelay must be between 0 and 10 seconds.");
        if (CircuitBreakerFailureRatio <= 0 || CircuitBreakerFailureRatio > 1)
            errors.Add("CircuitBreakerFailureRatio must be in (0, 1].");
        if (CircuitBreakerSamplingDuration <= TimeSpan.Zero || CircuitBreakerSamplingDuration > TimeSpan.FromMinutes(10))
            errors.Add("CircuitBreakerSamplingDuration must be between 1ms and 10 minutes.");
        if (CircuitBreakerMinimumThroughput < 1 || CircuitBreakerMinimumThroughput > 10_000)
            errors.Add("CircuitBreakerMinimumThroughput must be between 1 and 10000.");
        if (CircuitBreakerBreakDuration <= TimeSpan.Zero || CircuitBreakerBreakDuration > TimeSpan.FromMinutes(10))
            errors.Add("CircuitBreakerBreakDuration must be between 1ms and 10 minutes.");
        foreach (var method in IdempotentMethods)
            if (string.IsNullOrWhiteSpace(method))
                errors.Add("Idempotent methods must be non-empty.");
        return errors;
    }
}
