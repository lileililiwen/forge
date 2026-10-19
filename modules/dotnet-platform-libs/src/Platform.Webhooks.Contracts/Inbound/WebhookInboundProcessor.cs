#pragma warning disable CS1591, CA1848
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Logging.Abstractions;
using Microsoft.Extensions.Options;
using Platform.Core.Time;
using Platform.Webhooks.Contracts.Common;

namespace Platform.Webhooks.Contracts.Inbound;

/// <summary>Default inbound processor that wires verification, replay suppression, and a handler delegate.</summary>
public sealed class WebhookInboundProcessor : IWebhookInboundProcessor
{
    private readonly IWebhookInboxStore _store;
    private readonly IWebhookSignatureVerifier _verifier;
    private readonly IWebhookSecretResolver _secretResolver;
    private readonly WebhookOptions _options;
    private readonly IClock _clock;
    private readonly ILogger<WebhookInboundProcessor> _logger;
    private readonly string _leaseOwnerId;
    private readonly TimeSpan _leaseDuration;

    /// <summary>Creates a default inbound processor.</summary>
    public WebhookInboundProcessor(
        IWebhookInboxStore store,
        IWebhookSignatureVerifier verifier,
        IWebhookSecretResolver secretResolver,
        IOptions<WebhookOptions> options,
        IClock clock,
        ILogger<WebhookInboundProcessor>? logger = null,
        string? leaseOwnerId = null,
        TimeSpan? leaseDuration = null)
    {
        _store = store ?? throw new ArgumentNullException(nameof(store));
        _verifier = verifier ?? throw new ArgumentNullException(nameof(verifier));
        _secretResolver = secretResolver ?? throw new ArgumentNullException(nameof(secretResolver));
        _options = (options ?? throw new ArgumentNullException(nameof(options))).Value;
        _options.Validate();
        _clock = clock ?? throw new ArgumentNullException(nameof(clock));
        _logger = logger ?? NullLogger<WebhookInboundProcessor>.Instance;
        _leaseOwnerId = string.IsNullOrWhiteSpace(leaseOwnerId) ? Environment.MachineName + ":" + Guid.NewGuid().ToString("N") : leaseOwnerId;
        _leaseDuration = leaseDuration ?? TimeSpan.FromMinutes(2);
    }

    /// <inheritdoc />
    public async ValueTask<WebhookInboundResult> ProcessAsync(WebhookVerificationRequest request, WebhookInboundHandler handler, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(request);
        ArgumentNullException.ThrowIfNull(handler);
        if (request.Provider != _verifier.Provider) return WebhookInboundResult.Rejected(new WebhookFailure("webhook.provider_mismatch", "The provider scope did not match the configured verifier.", false));
        if (string.IsNullOrWhiteSpace(request.EventId)) return WebhookInboundResult.Rejected(new WebhookFailure("webhook.event_id_missing", "The provider event identifier is required.", false));
        var secret = _secretResolver.ResolveSecret(request.Provider, ResolveSecretKey(request));
        if (secret is null) return WebhookInboundResult.Rejected(new WebhookFailure("webhook.secret_not_found", "The webhook secret could not be resolved.", false));
        var verification = _verifier.Verify(request, secret);
        if (!verification.IsVerified) return WebhookInboundResult.Rejected(verification.Failure!);
        var message = WebhookInboxMessage.Create(request.Provider, request.EventId, _clock.UtcNow);
        var claim = await _store.TryClaimAsync(message, _clock, _leaseOwnerId, _leaseDuration, cancellationToken).ConfigureAwait(false);
        switch (claim.Status)
        {
            case WebhookInboxClaimStatus.Duplicate:
                if (_logger.IsEnabled(LogLevel.Debug))
                {
                    _logger.LogDebug("Webhook {Provider} event {EventId} is a duplicate.", request.Provider.Value, request.EventId);
                }
                return WebhookInboundResult.Duplicate();
            case WebhookInboxClaimStatus.Busy:
                if (_logger.IsEnabled(LogLevel.Debug))
                {
                    _logger.LogDebug("Webhook {Provider} event {EventId} is busy.", request.Provider.Value, request.EventId);
                }
                return WebhookInboundResult.Busy();
        }
        var claimed = claim.Message!;
        try
        {
            var handlerResult = await handler(request, _clock, cancellationToken).ConfigureAwait(false);
            if (handlerResult == WebhookInboundHandlerResult.Succeeded)
            {
                await _store.MarkCompletedAsync(claimed.ReplayKey, _leaseOwnerId, cancellationToken).ConfigureAwait(false);
                return WebhookInboundResult.Accepted();
            }
            var failure = handlerResult == WebhookInboundHandlerResult.TransientFailure
                ? new WebhookFailure("webhook.handler_transient", "The handler reported a transient failure.", true)
                : new WebhookFailure("webhook.handler_permanent", "The handler reported a permanent failure.", false);
            var deadLettered = handlerResult == WebhookInboundHandlerResult.PermanentFailure;
            var nextAttemptAt = deadLettered ? (DateTimeOffset?)null : _clock.UtcNow.Add(DefaultRetryDelay());
            await _store.MarkFailedAsync(claimed.ReplayKey, _leaseOwnerId, failure, nextAttemptAt, deadLettered, cancellationToken).ConfigureAwait(false);
            return deadLettered ? WebhookInboundResult.Rejected(failure) : WebhookInboundResult.Busy();
        }
        catch (Exception ex) when (ex is not OperationCanceledException)
        {
            _logger.LogWarning(ex, "Webhook {Provider} event {EventId} handler threw.", request.Provider.Value, request.EventId);
            var failure = new WebhookFailure("webhook.handler_exception", "The webhook handler raised an exception.", true);
            var nextAttemptAt = _clock.UtcNow.Add(DefaultRetryDelay());
            await _store.MarkFailedAsync(claimed.ReplayKey, _leaseOwnerId, failure, nextAttemptAt, deadLettered: false, cancellationToken).ConfigureAwait(false);
            return WebhookInboundResult.Busy();
        }
    }

    private TimeSpan DefaultRetryDelay()
    {
        var baseDelay = _options.DefaultRetryBaseDelay;
        var max = _options.DefaultRetryMaxDelay;
        return baseDelay > max ? max : baseDelay;
    }

    private static string ResolveSecretKey(WebhookVerificationRequest request)
    {
        return request.Headers.TryGetValue("X-Webhook-Key", out var key) && !string.IsNullOrWhiteSpace(key) ? key : "default";
    }
}
