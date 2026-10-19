namespace Platform.Mailing;

/// <summary>
/// Documented outcome of a single mail send attempt. The platform never
/// translates these into HTTP status codes; the consumer is responsible
/// for mapping the outcome to its own delivery semantics.
/// </summary>
public enum MailSendOutcome
{
    /// <summary>The provider accepted the message for delivery.</summary>
    Sent,

    /// <summary>A transient failure occurred; a retry is appropriate.</summary>
    TransientFailure,

    /// <summary>A permanent failure occurred; retries will not succeed.</summary>
    PermanentFailure,

    /// <summary>The provider accepted the message but reported a delivery bounce later.</summary>
    Bounced,
}
