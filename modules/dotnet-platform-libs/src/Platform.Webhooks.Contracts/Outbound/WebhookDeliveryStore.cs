using Platform.Webhooks.Contracts.Common;

namespace Platform.Webhooks.Contracts.Outbound;

/// <summary>Verdict for an outbound delivery attempt.</summary>
public enum WebhookDeliveryOutcome
{
    /// <summary>Target returned a 2xx response.</summary>
    Succeeded,
    /// <summary>Target returned a retryable status (5xx, 408, 429).</summary>
    RetryableResponse,
    /// <summary>Target returned a permanent failure (4xx other than 408/429).</summary>
    PermanentResponse,
    /// <summary>Request was rejected before an HTTP call (SSRF, subscription disabled, missing target).</summary>
    Rejected,
    /// <summary>Transport error (DNS, TCP, TLS, timeout).</summary>
    TransportError
}

/// <summary>Outcome of an outbound delivery attempt.</summary>
public sealed record WebhookDeliveryResult
{
    private WebhookDeliveryResult(WebhookDeliveryOutcome outcome, WebhookFailure? failure, int? responseStatus, DateTimeOffset? nextAttemptAt)
    {
        Outcome = outcome; Failure = failure; ResponseStatus = responseStatus; NextAttemptAt = nextAttemptAt;
    }
    /// <summary>Successful delivery.</summary>
    public static WebhookDeliveryResult Success(int? responseStatus = null) => new(WebhookDeliveryOutcome.Succeeded, null, responseStatus, null);
    /// <summary>Retryable response or transport error.</summary>
    public static WebhookDeliveryResult Retryable(WebhookFailure failure, int? responseStatus, DateTimeOffset nextAttemptAt) => new(WebhookDeliveryOutcome.RetryableResponse, failure, responseStatus, nextAttemptAt);
    /// <summary>Retryable transport error.</summary>
    public static WebhookDeliveryResult Transport(WebhookFailure failure, DateTimeOffset nextAttemptAt) => new(WebhookDeliveryOutcome.TransportError, failure, null, nextAttemptAt);
    /// <summary>Permanent response.</summary>
    public static WebhookDeliveryResult Permanent(WebhookFailure failure, int? responseStatus) => new(WebhookDeliveryOutcome.PermanentResponse, failure, responseStatus, null);
    /// <summary>Pre-flight rejection.</summary>
    public static WebhookDeliveryResult Reject(WebhookFailure failure) => new(WebhookDeliveryOutcome.Rejected, failure, null, null);
    /// <summary>Outcome category.</summary>
    public WebhookDeliveryOutcome Outcome { get; }
    /// <summary>Safe failure metadata.</summary>
    public WebhookFailure? Failure { get; }
    /// <summary>Observed response status code when available.</summary>
    public int? ResponseStatus { get; }
    /// <summary>Next eligible attempt time when a retry is scheduled.</summary>
    public DateTimeOffset? NextAttemptAt { get; }
}

/// <summary>Application-replaceable subscription store.</summary>
public interface IWebhookSubscriptionStore
{
    /// <summary>Gets a subscription by identifier.</summary>
    Task<WebhookSubscription?> GetAsync(WebhookSubscriptionId id, CancellationToken cancellationToken = default);
    /// <summary>Lists all enabled subscriptions for an event type.</summary>
    Task<IReadOnlyList<WebhookSubscription>> ListEnabledAsync(string eventType, CancellationToken cancellationToken = default);
}

/// <summary>Application-replaceable delivery store.</summary>
public interface IWebhookDeliveryStore
{
    /// <summary>Persists a new delivery record.</summary>
    Task<WebhookDelivery> CreateAsync(WebhookDelivery delivery, CancellationToken cancellationToken = default);
    /// <summary>Updates a delivery record with the supplied state.</summary>
    Task UpdateAsync(WebhookDelivery delivery, CancellationToken cancellationToken = default);
    /// <summary>Gets a delivery by identifier.</summary>
    Task<WebhookDelivery?> GetAsync(WebhookDeliveryId id, CancellationToken cancellationToken = default);
}
