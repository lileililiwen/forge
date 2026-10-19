using Microsoft.Extensions.DependencyInjection;
using Platform.Identity.Contracts;

namespace Platform.Identity.AspNetCore;

/// <summary>Host-level session operations backed by an application-provided store.</summary>
public interface IIdentitySessionService
{
    /// <summary>Creates a session using the registered store.</summary>
    ValueTask<IdentityProviderResult<IdentitySession>> CreateAsync(string subjectId, DateTimeOffset expiresAt, CancellationToken cancellationToken = default);
    /// <summary>Revokes a session using the registered store.</summary>
    ValueTask<IdentityProviderResult<bool>> RevokeAsync(string sessionId, CancellationToken cancellationToken = default);
}

/// <summary>Default session service that delegates to the registered <see cref="ISessionStore"/> and preserves normalized outcomes.</summary>
public sealed class IdentitySessionService(IServiceProvider services) : IIdentitySessionService
{
    /// <inheritdoc />
    public ValueTask<IdentityProviderResult<IdentitySession>> CreateAsync(string subjectId, DateTimeOffset expiresAt, CancellationToken cancellationToken = default)
    {
        var store = services.GetService<ISessionStore>();
        if (store is null)
            return ValueTask.FromResult(IdentityProviderResults.Failed<IdentitySession>(IdentityFailureReason.ProviderUnavailable));
        return store.CreateAsync(subjectId, expiresAt, cancellationToken);
    }

    /// <inheritdoc />
    public ValueTask<IdentityProviderResult<bool>> RevokeAsync(string sessionId, CancellationToken cancellationToken = default)
    {
        var store = services.GetService<ISessionStore>();
        if (store is null)
            return ValueTask.FromResult(IdentityProviderResults.Failed<bool>(IdentityFailureReason.ProviderUnavailable));
        return store.RevokeAsync(sessionId, cancellationToken);
    }
}
