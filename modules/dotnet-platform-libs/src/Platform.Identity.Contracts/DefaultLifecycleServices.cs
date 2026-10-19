namespace Platform.Identity.Contracts;

/// <summary>Default composition that wires the lifecycle services to the application-supplied stores and policies.</summary>
public sealed class DefaultIdentityLifecycleCoordinator : IIdentityLifecycleCoordinator
{
    /// <summary>Creates a new default coordinator.</summary>
    public DefaultIdentityLifecycleCoordinator(
        IRefreshTokenService refreshTokens,
        IPasswordRecoveryService passwordRecovery,
        ITwoFactorService twoFactor,
        IImpersonationService impersonation)
    {
        RefreshTokens = refreshTokens;
        PasswordRecovery = passwordRecovery;
        TwoFactor = twoFactor;
        Impersonation = impersonation;
    }

    /// <inheritdoc />
    public IRefreshTokenService RefreshTokens { get; }
    /// <inheritdoc />
    public IPasswordRecoveryService PasswordRecovery { get; }
    /// <inheritdoc />
    public ITwoFactorService TwoFactor { get; }
    /// <inheritdoc />
    public IImpersonationService Impersonation { get; }
}

/// <summary>Default <see cref="IRefreshTokenService"/> implementation that delegates to the store and the audit hook.</summary>
public sealed class DefaultRefreshTokenService : IRefreshTokenService
{
    private readonly IRefreshTokenStore _store;
    private readonly IIdentityAuditHook? _audit;

    /// <summary>Creates a new default refresh-token service.</summary>
    public DefaultRefreshTokenService(IRefreshTokenStore store, IIdentityAuditHook? audit = null)
    {
        _store = store;
        _audit = audit;
    }

    /// <inheritdoc />
    public async ValueTask<IdentityLifecycleResult<RefreshTokenRotation>> IssueAsync(string subjectId, string sessionId, DateTimeOffset expiresAt, CancellationToken cancellationToken = default)
    {
        var issued = await _store.IssueAsync(subjectId, sessionId, expiresAt, cancellationToken).ConfigureAwait(false);
        if (!issued.Succeeded)
            return IdentityLifecycleResults.Failed<RefreshTokenRotation>(issued.Outcome);
        var rotation = new RefreshTokenRotation(issued.Value!.Handle, issued.Value.Handle, issued.Value.ExpiresAt);
        if (_audit is not null)
            await _audit.RecordAsync(new IdentityAuditEvent("identity.refresh.issued", subjectId, true, DateTimeOffset.UtcNow), cancellationToken).ConfigureAwait(false);
        return IdentityLifecycleResults.Success(rotation);
    }

    /// <inheritdoc />
    public async ValueTask<IdentityLifecycleResult<RefreshTokenRotation>> RotateAsync(string handle, CancellationToken cancellationToken = default)
    {
        var rotated = await _store.ConsumeAsync(handle, cancellationToken).ConfigureAwait(false);
        if (_audit is not null)
        {
            var subjectId = rotated.Value is null ? null : rotated.Value.Handle;
            await _audit.RecordAsync(new IdentityAuditEvent("identity.refresh.rotated", subjectId, rotated.Succeeded, DateTimeOffset.UtcNow), cancellationToken).ConfigureAwait(false);
        }
        return rotated;
    }

    /// <inheritdoc />
    public async ValueTask<IdentityLifecycleOutcome> RevokeAsync(string handle, CancellationToken cancellationToken = default)
    {
        var outcome = await _store.RevokeAsync(handle, cancellationToken).ConfigureAwait(false);
        if (_audit is not null)
            await _audit.RecordAsync(new IdentityAuditEvent("identity.refresh.revoked", null, outcome == IdentityLifecycleOutcome.Succeeded, DateTimeOffset.UtcNow), cancellationToken).ConfigureAwait(false);
        return outcome;
    }
}
