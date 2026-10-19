namespace Platform.Identity.Contracts;

/// <summary>Authorizes a requested impersonation grant.</summary>
public interface IImpersonationPolicy
{
    /// <summary>
    /// Decides whether the supplied caller may impersonate the target. The platform
    /// requires the application to register a policy; without one, every request is
    /// denied and no grant is created.
    /// </summary>
    ValueTask<IdentityLifecycleResult<bool>> AuthorizeAsync(ImpersonationAuthorizationRequest request, CancellationToken cancellationToken = default);
}

/// <summary>Authorization input supplied to <see cref="IImpersonationPolicy"/>.</summary>
public sealed record ImpersonationAuthorizationRequest(
    string CallerSubjectId,
    string TargetSubjectId,
    string? Reason,
    TimeSpan Duration)
{
    /// <summary>Validates the request shape.</summary>
    public bool IsValid =>
        !string.IsNullOrWhiteSpace(CallerSubjectId) &&
        !string.IsNullOrWhiteSpace(TargetSubjectId) &&
        !string.IsNullOrWhiteSpace(Reason) &&
        Duration > TimeSpan.Zero;
}

/// <summary>Impersonation grant stored in the application session or cache.</summary>
public sealed record ImpersonationGrant(
    string GrantId,
    string CallerSubjectId,
    string TargetSubjectId,
    DateTimeOffset StartedAt,
    DateTimeOffset ExpiresAt,
    string Reason)
{
    /// <summary>True when the current time is at or after <see cref="ExpiresAt"/>.</summary>
    public bool IsExpired(DateTimeOffset now) => now >= ExpiresAt;
}

/// <summary>Context attached to a request that is executing under an impersonation grant.</summary>
public sealed record ImpersonationContext(
    string GrantId,
    string CallerSubjectId,
    string TargetSubjectId)
{
    /// <summary>Empty context used when no impersonation is active.</summary>
    public static ImpersonationContext None { get; } = new(string.Empty, string.Empty, string.Empty);
    /// <summary>True when the context represents an active grant.</summary>
    public bool IsActive => !string.IsNullOrEmpty(GrantId);
}

/// <summary>
/// Orchestrates impersonation grants. The platform never decides who may impersonate
/// whom; the application supplies an <see cref="IImpersonationPolicy"/> and the audit
/// hook captures each lifecycle event.
/// </summary>
public interface IImpersonationService
{
    /// <summary>Starts an impersonation grant after consulting the registered policy.</summary>
    ValueTask<IdentityLifecycleResult<ImpersonationGrant>> StartAsync(ImpersonationAuthorizationRequest request, CancellationToken cancellationToken = default);

    /// <summary>Ends an active impersonation grant. Returns <see cref="IdentityLifecycleOutcome.Succeeded"/> when the grant was ended, <see cref="IdentityLifecycleOutcome.InvalidHandle"/> when unknown.</summary>
    ValueTask<IdentityLifecycleOutcome> EndAsync(string grantId, CancellationToken cancellationToken = default);

    /// <summary>Returns the current impersonation context, or <see cref="ImpersonationContext.None"/>.</summary>
    ValueTask<IdentityLifecycleResult<ImpersonationContext>> GetActiveAsync(string callerSubjectId, CancellationToken cancellationToken = default);
}
