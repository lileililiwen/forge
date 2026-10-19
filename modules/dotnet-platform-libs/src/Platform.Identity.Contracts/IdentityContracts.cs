namespace Platform.Identity.Contracts;

/// <summary>Provider-neutral description of the current subject.</summary>
public sealed record CurrentUser(
    string? SubjectId = null,
    string? Email = null,
    string? TenantId = null,
    IReadOnlyCollection<string>? Roles = null,
    IReadOnlyCollection<string>? Permissions = null)
{
    /// <summary>Gets whether a subject was authenticated.</summary>
    public bool IsAuthenticated => !string.IsNullOrWhiteSpace(SubjectId);
    /// <summary>Gets the roles, or an empty collection.</summary>
    public IReadOnlyCollection<string> RoleSet => Roles ?? Array.Empty<string>();
    /// <summary>Gets the permissions, or an empty collection.</summary>
    public IReadOnlyCollection<string> PermissionSet => Permissions ?? Array.Empty<string>();
    /// <summary>Anonymous current-user value.</summary>
    public static CurrentUser Anonymous { get; } = new();
}

/// <summary>Obtains the current user without prescribing a token or user entity.</summary>
public interface ICurrentUserAccessor
{
    /// <summary>Gets the current user.</summary>
    CurrentUser GetCurrentUser();
}

/// <summary>Normalized credentials supplied by an application.</summary>
public sealed record Credential(string Identifier, string Secret)
{
    /// <summary>Validates the credential shape.</summary>
    public bool IsValid => !string.IsNullOrWhiteSpace(Identifier) && !string.IsNullOrEmpty(Secret);
}

/// <summary>Normalized external identity information.</summary>
public sealed record ExternalIdentity(string Provider, string Subject, string? Email = null, string? DisplayName = null);

/// <summary>Authentication failure categories safe to expose to callers.</summary>
public enum IdentityFailureReason
{
    /// <summary>Credentials did not match.</summary>
    InvalidCredentials,
    /// <summary>Required verification has not completed.</summary>
    Unverified,
    /// <summary>The provider could not be reached.</summary>
    ProviderUnavailable,
    /// <summary>The provider rejected the request.</summary>
    ProviderRejected,
    /// <summary>The provider asked the caller to retry later.</summary>
    RateLimited,
    /// <summary>The request shape was invalid.</summary>
    InvalidRequest,
    /// <summary>The provider returned an otherwise unclassified failure.</summary>
    Unknown,
}

/// <summary>Normalized outcome from an identity provider.</summary>
public sealed record IdentityProviderResult<T>(T? Value, IdentityFailureReason? Failure = null)
{
    /// <summary>Gets whether the operation succeeded.</summary>
    public bool Succeeded => Failure is null && Value is not null;
}

/// <summary>Factories for normalized provider outcomes.</summary>
public static class IdentityProviderResults
{
    /// <summary>Creates a successful outcome.</summary>
    public static IdentityProviderResult<T> Success<T>(T value) => new(value);
    /// <summary>Creates a safe failure without provider response details.</summary>
    public static IdentityProviderResult<T> Failed<T>(IdentityFailureReason reason) => new(default, reason);
}

/// <summary>Verifies application-owned credentials.</summary>
public interface ICredentialVerifier
{
    /// <summary>Verifies credentials and returns a normalized identity result.</summary>
    ValueTask<IdentityProviderResult<CurrentUser>> VerifyAsync(Credential credential, CancellationToken cancellationToken = default);
}

/// <summary>Exchanges an external authentication response for a normalized identity.</summary>
public interface IExternalIdentityProvider
{
    /// <summary>Gets the provider name.</summary>
    string Name { get; }
    /// <summary>Verifies an external identity.</summary>
    ValueTask<IdentityProviderResult<ExternalIdentity>> AuthenticateAsync(string authorizationCode, CancellationToken cancellationToken = default);
}

/// <summary>Verification challenge sent through an application-selected channel.</summary>
public sealed record VerificationChallenge(string ChallengeId, string Destination, DateTimeOffset ExpiresAt);

/// <summary>Verification provider contract for email, SMS, or another channel.</summary>
public interface IVerificationProvider
{
    /// <summary>Starts a verification challenge.</summary>
    ValueTask<IdentityProviderResult<VerificationChallenge>> StartAsync(string destination, CancellationToken cancellationToken = default);
    /// <summary>Completes a verification challenge.</summary>
    ValueTask<IdentityProviderResult<bool>> VerifyAsync(string challengeId, string code, CancellationToken cancellationToken = default);
}

/// <summary>Session value returned after successful authentication.</summary>
public sealed record IdentitySession(string SessionId, string SubjectId, DateTimeOffset ExpiresAt);

/// <summary>Replaceable session persistence contract.</summary>
public interface ISessionStore
{
    /// <summary>Creates a session.</summary>
    ValueTask<IdentityProviderResult<IdentitySession>> CreateAsync(string subjectId, DateTimeOffset expiresAt, CancellationToken cancellationToken = default);
    /// <summary>Revokes a session.</summary>
    ValueTask<IdentityProviderResult<bool>> RevokeAsync(string sessionId, CancellationToken cancellationToken = default);
}

/// <summary>Security-sensitive identity mutation audit event.</summary>
public sealed record IdentityAuditEvent(string Action, string? SubjectId, bool Succeeded, DateTimeOffset OccurredAt);

/// <summary>Receives normalized audit events without prescribing an audit store.</summary>
public interface IIdentityAuditHook
{
    /// <summary>Records an identity audit event.</summary>
    ValueTask RecordAsync(IdentityAuditEvent auditEvent, CancellationToken cancellationToken = default);
}
