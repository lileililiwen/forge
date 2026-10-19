using Platform.Webhooks.Contracts.Common;

namespace Platform.Webhooks.Contracts.Inbound;

/// <summary>Lifecycle states for an inbound webhook message.</summary>
public enum WebhookInboxState
{
    /// <summary>Message is recorded but not yet processed.</summary>
    Pending,
    /// <summary>Message was processed successfully.</summary>
    Completed,
    /// <summary>Message is currently leased by a worker.</summary>
    Leased,
    /// <summary>Message exhausted its retry budget and was moved to a terminal failure state.</summary>
    DeadLettered
}

/// <summary>Verdict for an inbound verification attempt.</summary>
public enum WebhookVerificationStatus
{
    /// <summary>Signature and metadata are valid.</summary>
    Verified,
    /// <summary>Signature was missing or did not match.</summary>
    SignatureInvalid,
    /// <summary>Headers were missing or malformed.</summary>
    HeaderMalformed,
    /// <summary>Timestamp was outside the tolerated skew window.</summary>
    TimestampOutOfRange,
    /// <summary>Secret could not be resolved for the supplied key.</summary>
    SecretNotFound
}

/// <summary>Decision returned by an inbound replay store claim attempt.</summary>
public enum WebhookInboxClaimStatus
{
    /// <summary>Message was newly recorded or accepted for processing.</summary>
    Claimed,
    /// <summary>An already-completed message with the same provider-scoped identifier was found.</summary>
    Duplicate,
    /// <summary>Message is currently leased by another worker.</summary>
    Busy
}

/// <summary>Stable verification request carrying the original request bytes and headers.</summary>
public sealed record WebhookVerificationRequest
{
    /// <summary>Creates a verification request.</summary>
    public WebhookVerificationRequest(WebhookProviderId provider, string eventId, IReadOnlyDictionary<string, string> headers, ReadOnlyMemory<byte> body)
    {
        if (string.IsNullOrWhiteSpace(eventId)) throw new ArgumentException("Event identifiers are required.", nameof(eventId));
        Provider = provider; EventId = eventId; Headers = headers ?? throw new ArgumentNullException(nameof(headers)); Body = body;
    }
    /// <summary>Provider scope.</summary>
    public WebhookProviderId Provider { get; }
    /// <summary>Provider-scoped event identifier.</summary>
    public string EventId { get; }
    /// <summary>Header values, indexed by header name (case-insensitive).</summary>
    public IReadOnlyDictionary<string, string> Headers { get; }
    /// <summary>Raw request body bytes.</summary>
    public ReadOnlyMemory<byte> Body { get; }
}

/// <summary>Result of an inbound verification attempt. Excludes secrets, signatures, and payload bodies.</summary>
public sealed record WebhookVerificationResult
{
    private WebhookVerificationResult(WebhookVerificationStatus status, WebhookFailure? failure)
    {
        Status = status; Failure = failure;
    }
    /// <summary>Verified.</summary>
    public static WebhookVerificationResult Verified() => new(WebhookVerificationStatus.Verified, null);
    /// <summary>Failure with safe metadata only.</summary>
    public static WebhookVerificationResult FailureResult(WebhookVerificationStatus status, WebhookFailure failure) => new(status, failure);
    /// <summary>Verification status.</summary>
    public WebhookVerificationStatus Status { get; }
    /// <summary>Safe failure information, when verification did not succeed.</summary>
    public WebhookFailure? Failure { get; }
    /// <summary>Whether verification succeeded.</summary>
    public bool IsVerified => Status == WebhookVerificationStatus.Verified;
}
