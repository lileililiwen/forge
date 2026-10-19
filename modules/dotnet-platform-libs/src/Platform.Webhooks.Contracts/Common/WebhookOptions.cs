namespace Platform.Webhooks.Contracts.Common;

/// <summary>Validation and runtime settings for inbound verification and outbound delivery.</summary>
public sealed class WebhookOptions
{
    /// <summary>Configuration section name.</summary>
    public const string SectionName = "Webhooks";
    /// <summary>Maximum accepted request body size in bytes for inbound webhooks.</summary>
    public long MaximumInboundBodyBytes { get; init; } = 1_048_576;
    /// <summary>Maximum tolerated clock skew for inbound timestamps in seconds.</summary>
    public int MaximumInboundClockSkewSeconds { get; init; } = 300;
    /// <summary>Maximum outbound payload size in bytes.</summary>
    public long MaximumOutboundBodyBytes { get; init; } = 1_048_576;
    /// <summary>Default outbound request timeout.</summary>
    public TimeSpan DefaultOutboundTimeout { get; init; } = TimeSpan.FromSeconds(10);
    /// <summary>Default maximum retry attempts.</summary>
    public int DefaultMaxRetryAttempts { get; init; } = 5;
    /// <summary>Default retry backoff base.</summary>
    public TimeSpan DefaultRetryBaseDelay { get; init; } = TimeSpan.FromSeconds(5);
    /// <summary>Default retry backoff cap.</summary>
    public TimeSpan DefaultRetryMaxDelay { get; init; } = TimeSpan.FromMinutes(15);
    /// <summary>Whether outbound SSRF resolution should accept loopback destinations. Default <c>false</c>.</summary>
    public bool AllowLoopbackTargets { get; init; }
    /// <summary>Optional allow-list of CIDR ranges or literal hosts permitted for outbound delivery.</summary>
    public IReadOnlyList<string> TargetAllowList { get; init; } = Array.Empty<string>();
    /// <summary>Validates options.</summary>
    public void Validate()
    {
        if (MaximumInboundBodyBytes <= 0 || MaximumInboundBodyBytes > 16L * 1024 * 1024) throw new ArgumentOutOfRangeException(nameof(MaximumInboundBodyBytes));
        if (MaximumInboundClockSkewSeconds < 0 || MaximumInboundClockSkewSeconds > 86_400) throw new ArgumentOutOfRangeException(nameof(MaximumInboundClockSkewSeconds));
        if (MaximumOutboundBodyBytes <= 0 || MaximumOutboundBodyBytes > 16L * 1024 * 1024) throw new ArgumentOutOfRangeException(nameof(MaximumOutboundBodyBytes));
        if (DefaultOutboundTimeout <= TimeSpan.Zero || DefaultOutboundTimeout > TimeSpan.FromMinutes(5)) throw new ArgumentOutOfRangeException(nameof(DefaultOutboundTimeout));
        if (DefaultMaxRetryAttempts < 0 || DefaultMaxRetryAttempts > 32) throw new ArgumentOutOfRangeException(nameof(DefaultMaxRetryAttempts));
        if (DefaultRetryBaseDelay <= TimeSpan.Zero || DefaultRetryBaseDelay > DefaultRetryMaxDelay) throw new ArgumentOutOfRangeException(nameof(DefaultRetryBaseDelay));
        if (DefaultRetryMaxDelay <= TimeSpan.Zero || DefaultRetryMaxDelay > TimeSpan.FromHours(24)) throw new ArgumentOutOfRangeException(nameof(DefaultRetryMaxDelay));
    }
}
