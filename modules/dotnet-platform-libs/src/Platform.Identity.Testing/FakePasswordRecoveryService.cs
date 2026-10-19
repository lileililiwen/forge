using System.Collections.Concurrent;
using System.Globalization;
using Platform.Identity.Contracts;

namespace Platform.Identity.Testing;

/// <summary>
/// Deterministic password-recovery service. The fake records the same
/// <see cref="IdentityLifecycleOutcome.Succeeded"/> outcome for unknown and known
/// subject identifiers to prevent user enumeration; tests that need to assert
/// the side effect can inspect <see cref="InitiatedChallenges"/>.
/// </summary>
public sealed class FakePasswordRecoveryService : IPasswordRecoveryService
{
    private readonly ConcurrentDictionary<string, ChallengeEntry> _challenges = new(StringComparer.Ordinal);
    private long _sequence;

    /// <summary>Codes the fake accepts on completion.</summary>
    public string AcceptedCode { get; set; } = "000000";

    /// <summary>Records every initiated challenge.</summary>
    public ConcurrentBag<(string SubjectIdentifier, string ChallengeId)> InitiatedChallenges { get; } = new();

    /// <inheritdoc />
    public ValueTask<IdentityLifecycleResult<PasswordRecoveryChallenge>> InitiateAsync(string subjectIdentifier, CancellationToken cancellationToken = default)
    {
        if (string.IsNullOrWhiteSpace(subjectIdentifier))
            return ValueTask.FromResult(IdentityLifecycleResults.Failed<PasswordRecoveryChallenge>(IdentityLifecycleOutcome.InvalidRequest));
        var id = "prc_" + Interlocked.Increment(ref _sequence).ToString("D20", CultureInfo.InvariantCulture);
        var challenge = new PasswordRecoveryChallenge(id, DateTimeOffset.UtcNow.AddMinutes(15));
        _challenges[id] = new ChallengeEntry(subjectIdentifier, challenge.ExpiresAt, false);
        InitiatedChallenges.Add((subjectIdentifier, id));
        return ValueTask.FromResult(IdentityLifecycleResults.Success(challenge));
    }

    /// <inheritdoc />
    public ValueTask<IdentityLifecycleResult<bool>> CompleteAsync(string challengeId, string code, string newPassword, CancellationToken cancellationToken = default)
    {
        if (string.IsNullOrWhiteSpace(challengeId) || string.IsNullOrEmpty(code) || string.IsNullOrEmpty(newPassword))
            return ValueTask.FromResult(IdentityLifecycleResults.Failed<bool>(IdentityLifecycleOutcome.InvalidRequest));
        if (!_challenges.TryGetValue(challengeId, out var entry))
            return ValueTask.FromResult(IdentityLifecycleResults.Failed<bool>(IdentityLifecycleOutcome.InvalidHandle));
        if (entry.Completed)
            return ValueTask.FromResult(IdentityLifecycleResults.Failed<bool>(IdentityLifecycleOutcome.Replayed));
        if (entry.ExpiresAt <= DateTimeOffset.UtcNow)
            return ValueTask.FromResult(IdentityLifecycleResults.Failed<bool>(IdentityLifecycleOutcome.Expired));
        if (!string.Equals(code, AcceptedCode, StringComparison.Ordinal))
            return ValueTask.FromResult(IdentityLifecycleResults.Failed<bool>(IdentityLifecycleOutcome.PolicyDenied));
        entry.Completed = true;
        return ValueTask.FromResult(IdentityLifecycleResults.Success(true));
    }

    private sealed class ChallengeEntry
    {
        public ChallengeEntry(string subjectIdentifier, DateTimeOffset expiresAt, bool completed)
        {
            SubjectIdentifier = subjectIdentifier;
            ExpiresAt = expiresAt;
            Completed = completed;
        }
        public string SubjectIdentifier { get; }
        public DateTimeOffset ExpiresAt { get; }
        public bool Completed { get; set; }
    }
}
