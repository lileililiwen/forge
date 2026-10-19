using Platform.Web.Telemetry;

namespace Platform.Http.Resilience;

/// <summary>Configures the platform outbound HTTP resilience pipeline (retry, timeout, circuit breaker, concurrency).</summary>
/// <remarks>Defaults are conservative and bounded. Retries are restricted to idempotent methods by default so non-idempotent writes are never replayed automatically.</remarks>
public sealed class PlatformHttpResilienceOptions : IValidatablePlatformOptions
{
    /// <summary>The default per-attempt timeout.</summary>
    public static readonly TimeSpan DefaultAttemptTimeout = TimeSpan.FromSeconds(5);

    /// <summary>The default total timeout for the entire request including all retries.</summary>
    public static readonly TimeSpan DefaultTotalTimeout = TimeSpan.FromSeconds(30);

    /// <summary>The default retry attempts (including the initial call).</summary>
    public const int DefaultMaxRetryAttempts = 3;

    /// <summary>The default retry back-off base delay.</summary>
    public static readonly TimeSpan DefaultRetryBaseDelay = TimeSpan.FromMilliseconds(500);

    /// <summary>The default circuit-breaker failure ratio (0..1).</summary>
    public const double DefaultCircuitBreakerFailureRatio = 0.5;

    /// <summary>The default circuit-breaker sampling duration.</summary>
    public static readonly TimeSpan DefaultCircuitBreakerSamplingDuration = TimeSpan.FromSeconds(30);

    /// <summary>The default circuit-breaker minimum throughput required to open.</summary>
    public const int DefaultCircuitBreakerMinimumThroughput = 10;

    /// <summary>The default circuit-breaker break duration.</summary>
    public static readonly TimeSpan DefaultCircuitBreakerBreakDuration = TimeSpan.FromSeconds(30);

    /// <summary>The default maximum concurrent calls permitted by the concurrency limiter.</summary>
    public const int DefaultMaxConcurrentCalls = 100;

    /// <summary>The default maximum queued calls while at the concurrency limit.</summary>
    public const int DefaultMaxQueueLength = 1000;

    /// <summary>Gets or sets whether the resilience pipeline is enabled. When disabled the handler becomes a pass-through. Default: <c>true</c>.</summary>
    public bool Enabled { get; set; } = true;

    /// <summary>Gets or sets the maximum number of retry attempts (including the initial call).</summary>
    public int MaxRetryAttempts { get; set; } = DefaultMaxRetryAttempts;

    /// <summary>Gets or sets the base delay used for exponential back-off between retries.</summary>
    public TimeSpan RetryBaseDelay { get; set; } = DefaultRetryBaseDelay;

    /// <summary>Gets or sets the per-attempt timeout.</summary>
    public TimeSpan AttemptTimeout { get; set; } = DefaultAttemptTimeout;

    /// <summary>Gets or sets the total timeout for the entire request including all retries.</summary>
    public TimeSpan TotalTimeout { get; set; } = DefaultTotalTimeout;

    /// <summary>Gets or sets the circuit-breaker failure ratio (0..1).</summary>
    public double CircuitBreakerFailureRatio { get; set; } = DefaultCircuitBreakerFailureRatio;

    /// <summary>Gets or sets the circuit-breaker sampling window.</summary>
    public TimeSpan CircuitBreakerSamplingDuration { get; set; } = DefaultCircuitBreakerSamplingDuration;

    /// <summary>Gets or sets the circuit-breaker minimum throughput required to consider opening.</summary>
    public int CircuitBreakerMinimumThroughput { get; set; } = DefaultCircuitBreakerMinimumThroughput;

    /// <summary>Gets or sets how long the circuit stays open once tripped.</summary>
    public TimeSpan CircuitBreakerBreakDuration { get; set; } = DefaultCircuitBreakerBreakDuration;

    /// <summary>Gets or sets the maximum number of concurrent calls permitted.</summary>
    public int MaxConcurrentCalls { get; set; } = DefaultMaxConcurrentCalls;

    /// <summary>Gets or sets the maximum number of calls queued while at the concurrency limit.</summary>
    public int MaxQueueLength { get; set; } = DefaultMaxQueueLength;

    /// <summary>Gets or sets the HTTP methods treated as idempotent for retry purposes. The default safe set is always included.</summary>
    public ISet<string> IdempotentMethods { get; set; } = new HashSet<string>(StringComparer.OrdinalIgnoreCase);

    /// <inheritdoc/>
    public IReadOnlyList<string> Validate()
    {
        var errors = new List<string>();
        if (MaxRetryAttempts < 1 || MaxRetryAttempts > 10)
            errors.Add("MaxRetryAttempts must be between 1 and 10.");
        if (RetryBaseDelay < TimeSpan.Zero || RetryBaseDelay > TimeSpan.FromSeconds(10))
            errors.Add("RetryBaseDelay must be between 0 and 10 seconds.");
        if (AttemptTimeout <= TimeSpan.Zero || AttemptTimeout > TimeSpan.FromMinutes(5))
            errors.Add("AttemptTimeout must be between 1ms and 5 minutes.");
        if (TotalTimeout <= TimeSpan.Zero || TotalTimeout > TimeSpan.FromMinutes(10))
            errors.Add("TotalTimeout must be between 1ms and 10 minutes.");
        if (CircuitBreakerFailureRatio <= 0 || CircuitBreakerFailureRatio > 1)
            errors.Add("CircuitBreakerFailureRatio must be in (0, 1].");
        if (CircuitBreakerSamplingDuration <= TimeSpan.Zero || CircuitBreakerSamplingDuration > TimeSpan.FromMinutes(10))
            errors.Add("CircuitBreakerSamplingDuration must be between 1ms and 10 minutes.");
        if (CircuitBreakerMinimumThroughput < 1 || CircuitBreakerMinimumThroughput > 10_000)
            errors.Add("CircuitBreakerMinimumThroughput must be between 1 and 10000.");
        if (CircuitBreakerBreakDuration <= TimeSpan.Zero || CircuitBreakerBreakDuration > TimeSpan.FromMinutes(10))
            errors.Add("CircuitBreakerBreakDuration must be between 1ms and 10 minutes.");
        if (MaxConcurrentCalls < 1 || MaxConcurrentCalls > 100_000)
            errors.Add("MaxConcurrentCalls must be between 1 and 100000.");
        if (MaxQueueLength < 0 || MaxQueueLength > 1_000_000)
            errors.Add("MaxQueueLength must be between 0 and 1000000.");
        foreach (var method in IdempotentMethods)
            if (string.IsNullOrWhiteSpace(method))
                errors.Add("Idempotent methods must be non-empty.");
        return errors;
    }
}
