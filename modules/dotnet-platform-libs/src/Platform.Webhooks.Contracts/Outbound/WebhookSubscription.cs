using Platform.Webhooks.Contracts.Common;

namespace Platform.Webhooks.Contracts.Outbound;

/// <summary>Lifecycle states for an outbound delivery attempt.</summary>
public enum WebhookDeliveryStatus
{
    /// <summary>Delivery is queued for the first attempt.</summary>
    Pending,
    /// <summary>Delivery request is in-flight to the target.</summary>
    InFlight,
    /// <summary>Target returned a 2xx response.</summary>
    Succeeded,
    /// <summary>Delivery failed and is scheduled for retry.</summary>
    RetryScheduled,
    /// <summary>Delivery exhausted its retry budget.</summary>
    DeadLettered,
    /// <summary>Delivery was rejected before an HTTP request was issued.</summary>
    Rejected
}

/// <summary>Application-owned outbound subscription.</summary>
public sealed record WebhookSubscription
{
    private WebhookSubscription(
        WebhookSubscriptionId id,
        Uri target,
        string secretKey,
        IReadOnlySet<string> eventTypes,
        bool isEnabled,
        WebhookRetryPolicy retryPolicy)
    {
        Id = id; Target = target; SecretKey = secretKey; EventTypes = eventTypes; IsEnabled = isEnabled; RetryPolicy = retryPolicy;
    }
    /// <summary>Creates a validated subscription.</summary>
    public static WebhookSubscription Create(
        WebhookSubscriptionId id,
        Uri target,
        string secretKey,
        IEnumerable<string> eventTypes,
        bool isEnabled,
        WebhookRetryPolicy retryPolicy)
    {
        ArgumentNullException.ThrowIfNull(target);
        if (string.IsNullOrWhiteSpace(secretKey)) throw new ArgumentException("A secret key is required.", nameof(secretKey));
        ArgumentNullException.ThrowIfNull(retryPolicy);
        var set = new HashSet<string>(eventTypes ?? Array.Empty<string>(), StringComparer.OrdinalIgnoreCase);
        if (set.Any(value => string.IsNullOrWhiteSpace(value))) throw new ArgumentException("Event types must be non-empty.", nameof(eventTypes));
        return new WebhookSubscription(id, target, secretKey, set, isEnabled, retryPolicy);
    }
    /// <summary>Subscription identifier.</summary>
    public WebhookSubscriptionId Id { get; }
    /// <summary>Absolute delivery target.</summary>
    public Uri Target { get; }
    /// <summary>Application-owned secret key.</summary>
    public string SecretKey { get; }
    /// <summary>Subscribed event types.</summary>
    public IReadOnlySet<string> EventTypes { get; }
    /// <summary>Whether the subscription currently accepts deliveries.</summary>
    public bool IsEnabled { get; }
    /// <summary>Configured retry policy.</summary>
    public WebhookRetryPolicy RetryPolicy { get; }
    /// <summary>Rehydrates a subscription with the supplied persisted enabled flag.</summary>
    public WebhookSubscription WithEnabled(bool enabled) => new(Id, Target, SecretKey, EventTypes, enabled, RetryPolicy);
}

/// <summary>Retry policy for outbound webhook deliveries.</summary>
public sealed record WebhookRetryPolicy
{
    /// <summary>Creates a validated retry policy.</summary>
    public WebhookRetryPolicy(int maxAttempts, TimeSpan baseDelay, TimeSpan maxDelay)
    {
        ArgumentOutOfRangeException.ThrowIfNegativeOrZero(maxAttempts);
        ArgumentOutOfRangeException.ThrowIfGreaterThan(maxAttempts, 32);
        if (baseDelay <= TimeSpan.Zero || baseDelay > TimeSpan.FromHours(1)) throw new ArgumentOutOfRangeException(nameof(baseDelay));
        if (maxDelay <= TimeSpan.Zero || maxDelay > TimeSpan.FromHours(24)) throw new ArgumentOutOfRangeException(nameof(maxDelay));
        if (baseDelay > maxDelay) throw new ArgumentOutOfRangeException(nameof(baseDelay), "Base delay must not exceed max delay.");
        MaxAttempts = maxAttempts; BaseDelay = baseDelay; MaxDelay = maxDelay;
    }
    /// <summary>Maximum number of attempts including the initial request.</summary>
    public int MaxAttempts { get; }
    /// <summary>Base delay used to compute the next-attempt time.</summary>
    public TimeSpan BaseDelay { get; }
    /// <summary>Maximum delay between retries.</summary>
    public TimeSpan MaxDelay { get; }
    /// <summary>Returns a delay using capped exponential back-off.</summary>
    public TimeSpan DelayFor(int attempt)
    {
        if (attempt < 1) return TimeSpan.Zero;
        var multiplier = Math.Pow(2, Math.Min(attempt - 1, 10));
        var computed = TimeSpan.FromMilliseconds(BaseDelay.TotalMilliseconds * multiplier);
        return computed > MaxDelay ? MaxDelay : computed;
    }
}
