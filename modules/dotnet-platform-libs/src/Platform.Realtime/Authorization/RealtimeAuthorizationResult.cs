namespace Platform.Realtime.Authorization;

/// <summary>
/// The outcome of an application authorization decision for a realtime
/// connection. The platform treats any non-allowed result as a rejection and
/// never exposes the reason to the transport client unless the application
/// chooses to.
/// </summary>
public sealed class RealtimeAuthorizationResult
{
    /// <summary>Gets a value indicating whether the connection is allowed.</summary>
    public bool Allowed { get; init; }

    /// <summary>
    /// Gets an optional, application-controlled rejection reason. This value
    /// is for logs and diagnostics only and must not leak secrets.
    /// </summary>
    public string? RejectionReason { get; init; }

    private RealtimeAuthorizationResult(bool allowed, string? reason)
    {
        Allowed = allowed;
        RejectionReason = reason;
    }

    /// <summary>Returns an allowed result.</summary>
    public static RealtimeAuthorizationResult Allow() => new(true, null);

    /// <summary>Returns a denied result with the supplied reason.</summary>
    public static RealtimeAuthorizationResult Deny(string? reason = null) => new(false, reason);
}
