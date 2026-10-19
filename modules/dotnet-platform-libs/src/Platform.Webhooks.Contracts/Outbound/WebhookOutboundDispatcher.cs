using System.Collections.Concurrent;
using System.Security.Cryptography;
using System.Text;
using Microsoft.Extensions.Options;
using Platform.Core.Time;
using Platform.Webhooks.Contracts.Security;
using Platform.Webhooks.Contracts.Common;

namespace Platform.Webhooks.Contracts.Outbound;

/// <summary>Default outbound dispatcher. Validates the target, signs the request, and records the delivery attempt.</summary>
public sealed class WebhookOutboundDispatcher : IWebhookOutboundDispatcher
{
    private readonly IWebhookSubscriptionStore _subscriptions;
    private readonly IWebhookDeliveryStore _deliveries;
    private readonly IWebhookSigningSecretResolver _secrets;
    private readonly ISsrfTargetValidator _ssrfValidator;
    private readonly IWebhookHttpSender _sender;
    private readonly WebhookOptions _options;
    private readonly IClock _clock;
    private readonly Func<WebhookDeliveryId> _idFactory;

    /// <summary>Creates a dispatcher with the default id factory.</summary>
    public WebhookOutboundDispatcher(
        IWebhookSubscriptionStore subscriptions,
        IWebhookDeliveryStore deliveries,
        IWebhookSigningSecretResolver secrets,
        ISsrfTargetValidator ssrfValidator,
        IWebhookHttpSender sender,
        IOptions<WebhookOptions> options,
        IClock clock) : this(subscriptions, deliveries, secrets, ssrfValidator, sender, options, clock, () => new WebhookDeliveryId(Guid.NewGuid().ToString("N")))
    {
    }

    /// <summary>Creates a dispatcher with the supplied id factory.</summary>
    public WebhookOutboundDispatcher(
        IWebhookSubscriptionStore subscriptions,
        IWebhookDeliveryStore deliveries,
        IWebhookSigningSecretResolver secrets,
        ISsrfTargetValidator ssrfValidator,
        IWebhookHttpSender sender,
        IOptions<WebhookOptions> options,
        IClock clock,
        Func<WebhookDeliveryId> idFactory)
    {
        _subscriptions = subscriptions ?? throw new ArgumentNullException(nameof(subscriptions));
        _deliveries = deliveries ?? throw new ArgumentNullException(nameof(deliveries));
        _secrets = secrets ?? throw new ArgumentNullException(nameof(secrets));
        _ssrfValidator = ssrfValidator ?? throw new ArgumentNullException(nameof(ssrfValidator));
        _sender = sender ?? throw new ArgumentNullException(nameof(sender));
        _options = (options ?? throw new ArgumentNullException(nameof(options))).Value;
        _options.Validate();
        _clock = clock ?? throw new ArgumentNullException(nameof(clock));
        _idFactory = idFactory ?? throw new ArgumentNullException(nameof(idFactory));
    }

    /// <inheritdoc />
    public async ValueTask<WebhookDeliveryResult> DispatchAsync(WebhookSubscriptionId subscriptionId, string eventType, string eventId, string payload, CancellationToken cancellationToken = default)
    {
        if (string.IsNullOrWhiteSpace(eventType)) throw new ArgumentException("Event types are required.", nameof(eventType));
        if (string.IsNullOrWhiteSpace(eventId)) throw new ArgumentException("Event identifiers are required.", nameof(eventId));
        if (string.IsNullOrEmpty(payload)) throw new ArgumentException("Payload is required.", nameof(payload));
        if (Encoding.UTF8.GetByteCount(payload) > _options.MaximumOutboundBodyBytes) throw new ArgumentOutOfRangeException(nameof(payload));
        var subscription = await _subscriptions.GetAsync(subscriptionId, cancellationToken).ConfigureAwait(false);
        if (subscription is null)
        {
            var missing = new WebhookFailure("webhook.subscription_not_found", "The subscription does not exist.", false);
            return WebhookDeliveryResult.Reject(missing);
        }
        if (!subscription.IsEnabled)
        {
            var disabled = new WebhookFailure("webhook.subscription_disabled", "The subscription is disabled.", false);
            return WebhookDeliveryResult.Reject(disabled);
        }
        var validation = _ssrfValidator.Validate(subscription.Target);
        if (!validation.Allowed)
        {
            var ssrfFailure = new WebhookFailure("webhook.ssrf_rejected", "The delivery target was rejected by the SSRF validator.", false);
            var record = WebhookDelivery.Create(_idFactory(), subscription.Id, eventType, eventId, _clock.UtcNow).WithPersistedState(WebhookDeliveryStatus.Rejected, 0, null, ssrfFailure, null);
            await _deliveries.CreateAsync(record, cancellationToken).ConfigureAwait(false);
            await _deliveries.UpdateAsync(record, cancellationToken).ConfigureAwait(false);
            return WebhookDeliveryResult.Reject(ssrfFailure);
        }
        var secret = _secrets.ResolveSecret(subscription.SecretKey);
        if (secret is null)
        {
            var secretFailure = new WebhookFailure("webhook.signing_secret_not_found", "The signing secret could not be resolved.", false);
            return WebhookDeliveryResult.Reject(secretFailure);
        }
        var delivery = WebhookDelivery.Create(_idFactory(), subscription.Id, eventType, eventId, _clock.UtcNow);
        await _deliveries.CreateAsync(delivery, cancellationToken).ConfigureAwait(false);
        var signature = ComputeSignature(secret, payload);
        var http = new WebhookHttpSendRequest(subscription.Target, payload, "X-Webhook-Signature", signature, "X-Webhook-Event", eventId, "X-Webhook-Event-Type", eventType, _options.DefaultOutboundTimeout);
        var sendResult = await _sender.SendAsync(http, cancellationToken).ConfigureAwait(false);
        return await RecordAsync(delivery, sendResult, subscription, cancellationToken).ConfigureAwait(false);
    }

    private async ValueTask<WebhookDeliveryResult> RecordAsync(WebhookDelivery delivery, WebhookHttpSendResult sendResult, WebhookSubscription subscription, CancellationToken cancellationToken)
    {
        WebhookDelivery updated;
        WebhookDeliveryResult result;
        if (sendResult.Failure is null)
        {
            updated = delivery.WithPersistedState(WebhookDeliveryStatus.Succeeded, delivery.AttemptCount + 1, null, null, sendResult.ResponseStatus);
            result = WebhookDeliveryResult.Success(sendResult.ResponseStatus);
        }
        else
        {
            var attempts = delivery.AttemptCount + 1;
            var isTransport = sendResult.ResponseStatus is null;
            var isRetryable = isTransport || sendResult.ResponseStatus is { } status && IsRetryableStatus(status);
            if (isRetryable && attempts < subscription.RetryPolicy.MaxAttempts)
            {
                var next = _clock.UtcNow.Add(subscription.RetryPolicy.DelayFor(attempts));
                updated = delivery.WithPersistedState(WebhookDeliveryStatus.RetryScheduled, attempts, next, sendResult.Failure, sendResult.ResponseStatus);
                result = isTransport ? WebhookDeliveryResult.Transport(sendResult.Failure, next) : WebhookDeliveryResult.Retryable(sendResult.Failure, sendResult.ResponseStatus, next);
            }
            else
            {
                var finalStatus = isRetryable ? WebhookDeliveryStatus.DeadLettered : WebhookDeliveryStatus.Rejected;
                updated = delivery.WithPersistedState(finalStatus, attempts, null, sendResult.Failure, sendResult.ResponseStatus);
                result = isTransport ? WebhookDeliveryResult.Transport(sendResult.Failure, _clock.UtcNow) : WebhookDeliveryResult.Permanent(sendResult.Failure, sendResult.ResponseStatus);
            }
        }
        await _deliveries.UpdateAsync(updated, cancellationToken).ConfigureAwait(false);
        return result;
    }

    private static bool IsRetryableStatus(int status) => status is >= 500 or 408 or 429;

    private static string ComputeSignature(byte[] secret, string payload)
    {
        using var hmac = new HMACSHA256(secret);
        var hash = hmac.ComputeHash(Encoding.UTF8.GetBytes(payload));
        return Convert.ToHexString(hash).ToLowerInvariant();
    }
}
