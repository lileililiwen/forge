namespace Platform.Identity.Contracts;

/// <summary>Opaque challenge returned by a password-recovery request.</summary>
public sealed record PasswordRecoveryChallenge(string ChallengeId, DateTimeOffset ExpiresAt);

/// <summary>
/// Initiates and completes a password-recovery workflow. The implementation MUST
/// return the same outcome for unknown and known subjects to avoid
/// user-enumeration signals.
/// </summary>
public interface IPasswordRecoveryService
{
    /// <summary>Initiates a recovery for the supplied subject identifier.</summary>
    ValueTask<IdentityLifecycleResult<PasswordRecoveryChallenge>> InitiateAsync(string subjectIdentifier, CancellationToken cancellationToken = default);

    /// <summary>Completes a recovery by submitting a one-time code.</summary>
    ValueTask<IdentityLifecycleResult<bool>> CompleteAsync(string challengeId, string code, string newPassword, CancellationToken cancellationToken = default);
}
