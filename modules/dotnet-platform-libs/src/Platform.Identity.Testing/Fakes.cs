using Platform.Authorization;
using Platform.Identity.Contracts;

namespace Platform.Identity.Testing;

/// <summary>Mutable deterministic current-user accessor for tests.</summary>
public sealed class FakeCurrentUserAccessor(CurrentUser? user = null) : ICurrentUserAccessor
{
    /// <summary>Gets or sets the returned user.</summary>
    public CurrentUser User { get; set; } = user ?? CurrentUser.Anonymous;
    /// <inheritdoc />
    public CurrentUser GetCurrentUser() => User;
}

/// <summary>Deterministic credential verifier keyed by identifier.</summary>
public sealed class FakeCredentialVerifier : ICredentialVerifier
{
    private readonly Dictionary<string, CurrentUser> users = new(StringComparer.OrdinalIgnoreCase);
    /// <summary>Adds a credential fixture.</summary>
    public FakeCredentialVerifier Add(string identifier, string secret, CurrentUser user)
    {
        users[identifier + "\n" + secret] = user;
        return this;
    }
    /// <inheritdoc />
    public ValueTask<IdentityProviderResult<CurrentUser>> VerifyAsync(Credential credential, CancellationToken cancellationToken = default) =>
        ValueTask.FromResult(users.TryGetValue(credential.Identifier + "\n" + credential.Secret, out var user)
            ? IdentityProviderResults.Success(user)
            : IdentityProviderResults.Failed<CurrentUser>(IdentityFailureReason.InvalidCredentials));
}

/// <summary>Deterministic external provider that returns configured identities.</summary>
public sealed class FakeExternalIdentityProvider(string name = "fake") : IExternalIdentityProvider
{
    private readonly Dictionary<string, ExternalIdentity> identities = new(StringComparer.Ordinal);
    /// <summary>Provider name.</summary>
    public string Name { get; } = name;
    /// <summary>Adds an authorization-code fixture.</summary>
    public FakeExternalIdentityProvider Add(string code, ExternalIdentity identity) { identities[code] = identity; return this; }
    /// <inheritdoc />
    public ValueTask<IdentityProviderResult<ExternalIdentity>> AuthenticateAsync(string authorizationCode, CancellationToken cancellationToken = default) =>
        ValueTask.FromResult(identities.TryGetValue(authorizationCode, out var identity)
            ? IdentityProviderResults.Success(identity)
            : IdentityProviderResults.Failed<ExternalIdentity>(IdentityFailureReason.ProviderRejected));
}

/// <summary>In-memory verification provider for deterministic tests.</summary>
public sealed class FakeVerificationProvider : IVerificationProvider
{
    private readonly Dictionary<string, string> challenges = new(StringComparer.Ordinal);
    /// <summary>Creates a challenge with a fixed code.</summary>
    public IdentityProviderResult<VerificationChallenge> Add(string challengeId, string destination, string code, DateTimeOffset expiresAt)
    {
        challenges[challengeId] = code;
        return IdentityProviderResults.Success(new VerificationChallenge(challengeId, destination, expiresAt));
    }
    /// <inheritdoc />
    public ValueTask<IdentityProviderResult<VerificationChallenge>> StartAsync(string destination, CancellationToken cancellationToken = default)
    {
        var id = Guid.NewGuid().ToString("N");
        return ValueTask.FromResult(Add(id, destination, "000000", DateTimeOffset.UtcNow.AddMinutes(5)));
    }
    /// <inheritdoc />
    public ValueTask<IdentityProviderResult<bool>> VerifyAsync(string challengeId, string code, CancellationToken cancellationToken = default) =>
        ValueTask.FromResult(challenges.TryGetValue(challengeId, out var expected) && expected == code
            ? IdentityProviderResults.Success(true)
            : IdentityProviderResults.Failed<bool>(IdentityFailureReason.InvalidCredentials));
}

/// <summary>Collects authorization decisions for assertions.</summary>
public sealed class RecordingAuthorizationDecisionAuditor : IAuthorizationDecisionAuditor
{
    /// <summary>Recorded decisions.</summary>
    public List<(AuthorizationDecision Decision, string? SubjectId)> Decisions { get; } = [];
    /// <inheritdoc />
    public ValueTask RecordAsync(AuthorizationDecision decision, string? subjectId, CancellationToken cancellationToken = default)
    { Decisions.Add((decision, subjectId)); return ValueTask.CompletedTask; }
}
