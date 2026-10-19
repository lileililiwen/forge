using Platform.Core.Time;
using Platform.Webhooks.Contracts.Common;

namespace Platform.Webhooks.Contracts.Inbound;

/// <summary>Outcome of an inbound processor call.</summary>
public sealed record WebhookInboundResult(WebhookInboundOutcome Outcome, WebhookFailure? Failure = null)
{
    /// <summary>Accepted and processed successfully.</summary>
    public static WebhookInboundResult Accepted() => new(WebhookInboundOutcome.Accepted);
    /// <summary>Verified and deduplicated; the handler was not executed.</summary>
    public static WebhookInboundResult Duplicate() => new(WebhookInboundOutcome.Duplicate);
    /// <summary>Verification or replay failed safely.</summary>
    public static WebhookInboundResult Rejected(WebhookFailure failure) => new(WebhookInboundOutcome.Rejected, failure);
    /// <summary>Recorded but lease was not granted; worker should retry.</summary>
    public static WebhookInboundResult Busy() => new(WebhookInboundOutcome.Busy);
}

/// <summary>Outcome states for an inbound processor call.</summary>
public enum WebhookInboundOutcome
{
    /// <summary>Request passed verification and replay checks.</summary>
    Accepted,
    /// <summary>Request was a duplicate of an already-processed event.</summary>
    Duplicate,
    /// <summary>Request failed verification or replay checks safely.</summary>
    Rejected,
    /// <summary>Request is currently leased by another worker.</summary>
    Busy
}

/// <summary>Application-supplied delegate invoked after verification and replay protection succeed.</summary>
public delegate ValueTask<WebhookInboundHandlerResult> WebhookInboundHandler(WebhookVerificationRequest request, IClock clock, CancellationToken cancellationToken);

/// <summary>Result returned by an inbound handler.</summary>
public enum WebhookInboundHandlerResult
{
    /// <summary>Handler completed; the message is marked as completed.</summary>
    Succeeded,
    /// <summary>Handler reported a transient failure; the message is scheduled for retry.</summary>
    TransientFailure,
    /// <summary>Handler reported a permanent failure; the message is moved to a terminal failure state.</summary>
    PermanentFailure
}

/// <summary>Provider-neutral inbound processing boundary.</summary>
public interface IWebhookInboundProcessor
{
    /// <summary>Verifies, deduplicates, and dispatches a verified request to the supplied handler.</summary>
    ValueTask<WebhookInboundResult> ProcessAsync(WebhookVerificationRequest request, WebhookInboundHandler handler, CancellationToken cancellationToken = default);
}
