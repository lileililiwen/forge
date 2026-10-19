namespace Platform.Identity.Contracts;

/// <summary>Stable outcome codes for the identity lifecycle contracts.</summary>
public enum IdentityLifecycleOutcome
{
    /// <summary>The operation completed.</summary>
    Succeeded = 0,
    /// <summary>The supplied handle is unknown or malformed.</summary>
    InvalidHandle,
    /// <summary>The supplied handle is past its expiry timestamp.</summary>
    Expired,
    /// <summary>The supplied handle has been revoked.</summary>
    Revoked,
    /// <summary>The supplied handle was already consumed by an earlier request.</summary>
    Replayed,
    /// <summary>The application policy denied the operation.</summary>
    PolicyDenied,
    /// <summary>Lifecycle precondition (subject, factor enrollment, active session) is not satisfied.</summary>
    PreconditionNotMet,
    /// <summary>The provider or store is unavailable; the operation may be retried.</summary>
    ProviderUnavailable,
    /// <summary>The request shape was invalid.</summary>
    InvalidRequest,
    /// <summary>The provider returned an otherwise unclassified failure.</summary>
    Unknown,
}

/// <summary>Generic, safe failure outcome for identity lifecycle operations.</summary>
public sealed record IdentityLifecycleResult<T>(T? Value, IdentityLifecycleOutcome Outcome)
{
    /// <summary>Gets whether the operation succeeded.</summary>
    public bool Succeeded => Outcome == IdentityLifecycleOutcome.Succeeded && Value is not null;
    /// <summary>Gets the failed outcome (excluding the success code).</summary>
    public IdentityLifecycleOutcome Failure => Succeeded ? IdentityLifecycleOutcome.Succeeded : Outcome;
}

/// <summary>Factory methods for <see cref="IdentityLifecycleResult{T}"/>.</summary>
public static class IdentityLifecycleResults
{
    /// <summary>Creates a success outcome.</summary>
    public static IdentityLifecycleResult<T> Success<T>(T value) => new(value, IdentityLifecycleOutcome.Succeeded);
    /// <summary>Creates a failed outcome with the supplied code.</summary>
    public static IdentityLifecycleResult<T> Failed<T>(IdentityLifecycleOutcome outcome) => new(default, outcome);
}
