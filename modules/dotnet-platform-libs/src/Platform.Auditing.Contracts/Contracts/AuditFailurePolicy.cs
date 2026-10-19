namespace Platform.Auditing.Contracts;

/// <summary>Controls what happens when an audit sink fails to accept an event.</summary>
public enum AuditFailurePolicy
{
    /// <summary>
    /// Log and drop the event (and optionally route it to the dead-letter sink). The business
    /// request is never failed because of an auditing problem.
    /// </summary>
    FailOpen = 0,

    /// <summary>
    /// Surface the failure so the caller can decide. Selected security events may choose this
    /// policy to avoid silently losing security-relevant audits.
    /// </summary>
    FailClosed = 1,
}
