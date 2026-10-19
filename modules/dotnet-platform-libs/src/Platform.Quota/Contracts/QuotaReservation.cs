namespace Platform.Quota.Contracts;

/// <summary>Reservation lifecycle state.</summary>
public enum QuotaReservationStatus
{
    /// <summary>Capacity is held.</summary>
    Reserved,
    /// <summary>Capacity was settled into consumption.</summary>
    Settled,
    /// <summary>Capacity was released.</summary>
    Released,
    /// <summary>Reservation expired before settlement.</summary>
    Expired
}

/// <summary>Reservation details retained for idempotent lifecycle operations.</summary>
public sealed record QuotaReservation(QuotaOperationKey OperationKey, QuotaSubject Subject, QuotaResource Resource, QuotaWindow Window, long Amount, DateTimeOffset ExpiresAt, QuotaReservationStatus Status);

/// <summary>Lifecycle operation result.</summary>
public sealed record QuotaLifecycleResult(QuotaLifecycleStatus Status, QuotaReservation? Reservation = null, QuotaFailure? Failure = null, QuotaDecision? Decision = null);

/// <summary>Lifecycle operation states.</summary>
public enum QuotaLifecycleStatus
{
    /// <summary>Operation changed state.</summary>
    Reserved,
    /// <summary>Operation settled.</summary>
    Settled,
    /// <summary>Operation released.</summary>
    Released,
    /// <summary>Repeated settlement.</summary>
    AlreadySettled,
    /// <summary>Repeated release.</summary>
    AlreadyReleased,
    /// <summary>Operation was not found.</summary>
    NotFound,
    /// <summary>Transition is invalid.</summary>
    InvalidTransition,
    /// <summary>Capacity was exhausted.</summary>
    Denied
}

/// <summary>Safe failure information.</summary>
public sealed record QuotaFailure
{
    /// <summary>Creates validated failure information.</summary>
    public QuotaFailure(string code, string message)
    {
        if (string.IsNullOrWhiteSpace(code) || string.IsNullOrWhiteSpace(message)) throw new ArgumentException("Quota failure code and message are required.");
        Code = code; Message = message;
    }
    /// <summary>Failure code.</summary>
    public string Code { get; }
    /// <summary>Safe failure message.</summary>
    public string Message { get; }
}
