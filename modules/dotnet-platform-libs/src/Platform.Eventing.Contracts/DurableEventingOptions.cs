namespace Platform.Eventing.Contracts;

/// <summary>Options controlling durable message claiming and retry behavior.</summary>
public sealed class DurableEventingOptions
{
    /// <summary>Configuration section name.</summary>
    public const string SectionName = "DurableEventing";

    /// <summary>Maximum number of dispatch attempts, including the first attempt.</summary>
    public int MaxAttempts { get; set; } = 5;

    /// <summary>Initial retry delay.</summary>
    public TimeSpan InitialRetryDelay { get; set; } = TimeSpan.FromSeconds(5);

    /// <summary>Maximum retry delay.</summary>
    public TimeSpan MaxRetryDelay { get; set; } = TimeSpan.FromMinutes(5);

    /// <summary>Maximum number of messages claimed per dispatch batch.</summary>
    public int BatchSize { get; set; } = 50;

    /// <summary>Duration for which a worker owns a claimed message.</summary>
    public TimeSpan LeaseDuration { get; set; } = TimeSpan.FromMinutes(5);

    /// <summary>Delay between empty or completed dispatch batches.</summary>
    public TimeSpan PollInterval { get; set; } = TimeSpan.FromSeconds(5);

    /// <summary>Validates option values.</summary>
    public void Validate()
    {
        if (MaxAttempts <= 0) throw new ArgumentOutOfRangeException(nameof(MaxAttempts));
        if (InitialRetryDelay <= TimeSpan.Zero) throw new ArgumentOutOfRangeException(nameof(InitialRetryDelay));
        if (MaxRetryDelay < InitialRetryDelay) throw new ArgumentOutOfRangeException(nameof(MaxRetryDelay));
        ArgumentOutOfRangeException.ThrowIfNegativeOrZero(BatchSize, nameof(BatchSize));
        ArgumentOutOfRangeException.ThrowIfLessThanOrEqual(LeaseDuration, TimeSpan.Zero, nameof(LeaseDuration));
        ArgumentOutOfRangeException.ThrowIfLessThanOrEqual(PollInterval, TimeSpan.Zero, nameof(PollInterval));
    }

    /// <summary>Calculates the bounded exponential delay for an attempt.</summary>
    public TimeSpan GetRetryDelay(int attempt)
    {
        Validate();
        var multiplier = Math.Pow(2, Math.Max(0, attempt - 1));
        var seconds = Math.Min(MaxRetryDelay.TotalSeconds, InitialRetryDelay.TotalSeconds * multiplier);
        return TimeSpan.FromSeconds(seconds);
    }
}
