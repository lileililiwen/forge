using System.Collections.Concurrent;
using System.Globalization;
using Platform.Identity.Contracts;

namespace Platform.Identity.Testing;

/// <summary>
/// Deterministic two-factor service. The fake issues challenges with a configurable
/// accepted code and reports <see cref="IdentityLifecycleOutcome.Succeeded"/> for
/// matching code submissions. Tests can inspect <see cref="IssuedChallenges"/>.
/// </summary>
public sealed class FakeTwoFactorService : ITwoFactorService
{
    private static readonly string[] DefaultChannels = { "email", "authenticator" };
    private readonly ConcurrentDictionary<string, ChallengeEntry> _challenges = new(StringComparer.Ordinal);
    private long _sequence;

    /// <summary>Codes the fake accepts on verification.</summary>
    public string AcceptedCode { get; set; } = "000000";

    /// <summary>Records every issued challenge.</summary>
    public ConcurrentBag<(string SubjectId, string ChallengeId)> IssuedChallenges { get; } = new();

    /// <inheritdoc />
    public ValueTask<IdentityLifecycleResult<TwoFactorChallenge>> IssueAsync(string subjectId, CancellationToken cancellationToken = default)
    {
        if (string.IsNullOrWhiteSpace(subjectId))
            return ValueTask.FromResult(IdentityLifecycleResults.Failed<TwoFactorChallenge>(IdentityLifecycleOutcome.InvalidRequest));
        var id = "tfc_" + Interlocked.Increment(ref _sequence).ToString("D20", CultureInfo.InvariantCulture);
        var challenge = new TwoFactorChallenge(id, DateTimeOffset.UtcNow.AddMinutes(5), DefaultChannels);
        _challenges[id] = new ChallengeEntry(subjectId, challenge.ExpiresAt);
        IssuedChallenges.Add((subjectId, id));
        return ValueTask.FromResult(IdentityLifecycleResults.Success(challenge));
    }

    /// <inheritdoc />
    public ValueTask<IdentityLifecycleResult<bool>> VerifyAsync(string challengeId, string code, CancellationToken cancellationToken = default)
    {
        if (string.IsNullOrWhiteSpace(challengeId) || string.IsNullOrEmpty(code))
            return ValueTask.FromResult(IdentityLifecycleResults.Failed<bool>(IdentityLifecycleOutcome.InvalidRequest));
        if (!_challenges.TryGetValue(challengeId, out var entry))
            return ValueTask.FromResult(IdentityLifecycleResults.Failed<bool>(IdentityLifecycleOutcome.InvalidHandle));
        if (entry.ExpiresAt <= DateTimeOffset.UtcNow)
            return ValueTask.FromResult(IdentityLifecycleResults.Failed<bool>(IdentityLifecycleOutcome.Expired));
        if (!string.Equals(code, AcceptedCode, StringComparison.Ordinal))
            return ValueTask.FromResult(IdentityLifecycleResults.Failed<bool>(IdentityLifecycleOutcome.PolicyDenied));
        return ValueTask.FromResult(IdentityLifecycleResults.Success(true));
    }

    private sealed record ChallengeEntry(string SubjectId, DateTimeOffset ExpiresAt);
}
