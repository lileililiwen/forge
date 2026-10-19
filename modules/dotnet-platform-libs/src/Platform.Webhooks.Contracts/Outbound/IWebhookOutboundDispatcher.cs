using Platform.Webhooks.Contracts.Security;
using Platform.Webhooks.Contracts.Common;

namespace Platform.Webhooks.Contracts.Outbound;

/// <summary>Sends a single signed outbound HTTP request to a validated target.</summary>
public interface IWebhookHttpSender
{
    /// <summary>Sends a signed request and returns the response status code or a transport failure.</summary>
    ValueTask<WebhookHttpSendResult> SendAsync(WebhookHttpSendRequest request, CancellationToken cancellationToken = default);
}

/// <summary>Outbound HTTP request.</summary>
public sealed record WebhookHttpSendRequest(Uri Target, string Body, string SignatureHeaderName, string SignatureHeaderValue, string EventIdHeaderName, string EventIdHeaderValue, string EventTypeHeaderName, string EventTypeHeaderValue, TimeSpan Timeout);

/// <summary>Outbound HTTP send result.</summary>
public sealed record WebhookHttpSendResult(int? ResponseStatus, WebhookFailure? Failure)
{
    /// <summary>Successful send with a response status.</summary>
    public static WebhookHttpSendResult Success(int responseStatus) => new(responseStatus, null);
    /// <summary>Transport-level failure with no response status.</summary>
    public static WebhookHttpSendResult TransportFailure(WebhookFailure failure) => new(null, failure);
    /// <summary>Application-level failure with a response status.</summary>
    public static WebhookHttpSendResult ResponseFailure(int responseStatus, WebhookFailure failure) => new(responseStatus, failure);
}

/// <summary>Resolves signing secrets for outbound deliveries.</summary>
public interface IWebhookSigningSecretResolver
{
    /// <summary>Returns the secret used to sign the delivery, or <c>null</c> when the key is unknown.</summary>
    byte[]? ResolveSecret(string secretKey);
}

/// <summary>Provider-neutral outbound dispatcher.</summary>
public interface IWebhookOutboundDispatcher
{
    /// <summary>Enqueues and attempts to deliver a single event to a specific subscription.</summary>
    ValueTask<WebhookDeliveryResult> DispatchAsync(WebhookSubscriptionId subscriptionId, string eventType, string eventId, string payload, CancellationToken cancellationToken = default);
}
