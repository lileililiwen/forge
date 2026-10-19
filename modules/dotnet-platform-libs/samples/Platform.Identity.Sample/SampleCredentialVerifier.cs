using Platform.Identity.Contracts;

namespace Platform.Identity.Sample;

/// <summary>
/// Application-owned credential store. The platform defines the
/// <see cref="ICredentialVerifier"/> boundary; user records, password
/// rules, and lockout policy stay in the application.
/// </summary>
public sealed class SampleCredentialVerifier : ICredentialVerifier
{
    private readonly Dictionary<string, CurrentUser> _users = new(StringComparer.OrdinalIgnoreCase);

    /// <summary>Adds one credential fixture. Production code replaces this with a hashed store.</summary>
    public SampleCredentialVerifier Add(string identifier, string secret, CurrentUser user)
    {
        _users[identifier + "\n" + secret] = user;
        return this;
    }

    /// <inheritdoc />
    public ValueTask<IdentityProviderResult<CurrentUser>> VerifyAsync(Credential credential, CancellationToken cancellationToken = default)
    {
        return ValueTask.FromResult(
            _users.TryGetValue(credential.Identifier + "\n" + credential.Secret, out var user)
                ? IdentityProviderResults.Success(user)
                : IdentityProviderResults.Failed<CurrentUser>(IdentityFailureReason.InvalidCredentials));
    }
}

/// <summary>Login request body.</summary>
/// <param name="Identifier">The supplied identifier.</param>
/// <param name="Secret">The supplied secret.</param>
public sealed record LoginRequest(string Identifier, string Secret);
