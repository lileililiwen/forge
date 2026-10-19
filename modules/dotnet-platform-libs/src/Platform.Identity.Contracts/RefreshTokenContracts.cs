namespace Platform.Identity.Contracts;

/// <summary>Opaque, single-use refresh token issued by the application store.</summary>
public sealed record RefreshToken(
    string Handle,
    string SubjectId,
    string SessionId,
    DateTimeOffset IssuedAt,
    DateTimeOffset ExpiresAt);

/// <summary>Consume-and-replace outcome returned to callers.</summary>
public sealed record RefreshTokenRotation(string Token, string Handle, DateTimeOffset ExpiresAt);

/// <summary>Application-owned store for refresh tokens. Implementations MUST enforce single-use semantics.</summary>
public interface IRefreshTokenStore
{
    /// <summary>Issues a new refresh token bound to the supplied session.</summary>
    ValueTask<IdentityLifecycleResult<RefreshToken>> IssueAsync(string subjectId, string sessionId, DateTimeOffset expiresAt, CancellationToken cancellationToken = default);

    /// <summary>
    /// Atomically consumes the supplied refresh handle and returns a replacement token. The
    /// operation MUST be linearizable across concurrent callers presenting the same handle:
    /// at most one caller observes <see cref="IdentityLifecycleOutcome.Succeeded"/>; the
    /// others observe <see cref="IdentityLifecycleOutcome.Replayed"/>.
    /// </summary>
    ValueTask<IdentityLifecycleResult<RefreshTokenRotation>> ConsumeAsync(string handle, CancellationToken cancellationToken = default);

    /// <summary>Revokes a refresh handle and any descendants it issued.</summary>
    ValueTask<IdentityLifecycleOutcome> RevokeAsync(string handle, CancellationToken cancellationToken = default);
}

/// <summary>
/// Orchestrates refresh-token rotation on top of an application-owned
/// <see cref="IRefreshTokenStore"/>. The platform never persists or hashes the handle on
/// behalf of the application.
/// </summary>
public interface IRefreshTokenService
{
    /// <summary>Issues a new refresh token bound to the supplied session.</summary>
    ValueTask<IdentityLifecycleResult<RefreshTokenRotation>> IssueAsync(string subjectId, string sessionId, DateTimeOffset expiresAt, CancellationToken cancellationToken = default);

    /// <summary>Consumes the supplied handle and returns a rotated token. Replay is reported as <see cref="IdentityLifecycleOutcome.Replayed"/>.</summary>
    ValueTask<IdentityLifecycleResult<RefreshTokenRotation>> RotateAsync(string handle, CancellationToken cancellationToken = default);

    /// <summary>Revokes a refresh handle.</summary>
    ValueTask<IdentityLifecycleOutcome> RevokeAsync(string handle, CancellationToken cancellationToken = default);
}
