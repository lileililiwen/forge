namespace Platform.Identity.Contracts;

/// <summary>Opaque two-factor challenge.</summary>
public sealed record TwoFactorChallenge(string ChallengeId, DateTimeOffset ExpiresAt, IReadOnlyCollection<string> Channels);

/// <summary>
/// Initiates and verifies a two-factor challenge. The platform never knows the
/// subject's enrolled factors; the application supplies the channel selection and
/// the verification code.
/// </summary>
public interface ITwoFactorService
{
    /// <summary>Issues a two-factor challenge for the supplied subject.</summary>
    ValueTask<IdentityLifecycleResult<TwoFactorChallenge>> IssueAsync(string subjectId, CancellationToken cancellationToken = default);

    /// <summary>Verifies a two-factor challenge.</summary>
    ValueTask<IdentityLifecycleResult<bool>> VerifyAsync(string challengeId, string code, CancellationToken cancellationToken = default);
}
