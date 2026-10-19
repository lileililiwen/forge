namespace Platform.Eventing.Contracts;

/// <summary>Processing states shared by durable outbox and inbox records.</summary>
public enum DurableMessageState
{
    /// <summary>Eligible for initial processing.</summary>
    Pending,
    /// <summary>Owned by a worker until the lease expires.</summary>
    Leased,
    /// <summary>Processed successfully.</summary>
    Succeeded,
    /// <summary>Failed and eligible for a later retry.</summary>
    RetryableFailure,
    /// <summary>Failed permanently or exhausted retry attempts.</summary>
    DeadLetter,
}
