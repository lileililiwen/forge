using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Platform.Identity.Contracts;

namespace Platform.Identity.AspNetCore;

/// <summary>ASP.NET Core integration seams for the platform identity lifecycle contracts.</summary>
public static class IdentityLifecycleServiceCollectionExtensions
{
    /// <summary>Registers the platform identity lifecycle composition without configuring concrete stores.</summary>
    public static IServiceCollection AddPlatformIdentityLifecycle(this IServiceCollection services)
    {
        ArgumentNullException.ThrowIfNull(services);
        services.TryAddSingleton<IRefreshTokenService>(sp =>
        {
            var store = sp.GetService<IRefreshTokenStore>();
            if (store is null)
                return new MissingRefreshTokenService();
            return new DefaultRefreshTokenService(store, sp.GetService<IIdentityAuditHook>());
        });
        services.TryAddSingleton<IIdentityLifecycleCoordinator>(sp => new DefaultIdentityLifecycleCoordinator(
            sp.GetRequiredService<IRefreshTokenService>(),
            sp.GetService<IPasswordRecoveryService>() ?? new MissingPasswordRecoveryService(),
            sp.GetService<ITwoFactorService>() ?? new MissingTwoFactorService(),
            sp.GetService<IImpersonationService>() ?? new MissingImpersonationService()));
        return services;
    }

    private sealed class MissingRefreshTokenService : IRefreshTokenService
    {
        public ValueTask<IdentityLifecycleResult<RefreshTokenRotation>> IssueAsync(string subjectId, string sessionId, DateTimeOffset expiresAt, CancellationToken cancellationToken = default) =>
            ValueTask.FromResult(IdentityLifecycleResults.Failed<RefreshTokenRotation>(IdentityLifecycleOutcome.ProviderUnavailable));
        public ValueTask<IdentityLifecycleResult<RefreshTokenRotation>> RotateAsync(string handle, CancellationToken cancellationToken = default) =>
            ValueTask.FromResult(IdentityLifecycleResults.Failed<RefreshTokenRotation>(IdentityLifecycleOutcome.ProviderUnavailable));
        public ValueTask<IdentityLifecycleOutcome> RevokeAsync(string handle, CancellationToken cancellationToken = default) =>
            ValueTask.FromResult(IdentityLifecycleOutcome.ProviderUnavailable);
    }

    private sealed class MissingPasswordRecoveryService : IPasswordRecoveryService
    {
        public ValueTask<IdentityLifecycleResult<PasswordRecoveryChallenge>> InitiateAsync(string subjectIdentifier, CancellationToken cancellationToken = default) =>
            ValueTask.FromResult(IdentityLifecycleResults.Failed<PasswordRecoveryChallenge>(IdentityLifecycleOutcome.ProviderUnavailable));
        public ValueTask<IdentityLifecycleResult<bool>> CompleteAsync(string challengeId, string code, string newPassword, CancellationToken cancellationToken = default) =>
            ValueTask.FromResult(IdentityLifecycleResults.Failed<bool>(IdentityLifecycleOutcome.ProviderUnavailable));
    }

    private sealed class MissingTwoFactorService : ITwoFactorService
    {
        public ValueTask<IdentityLifecycleResult<TwoFactorChallenge>> IssueAsync(string subjectId, CancellationToken cancellationToken = default) =>
            ValueTask.FromResult(IdentityLifecycleResults.Failed<TwoFactorChallenge>(IdentityLifecycleOutcome.ProviderUnavailable));
        public ValueTask<IdentityLifecycleResult<bool>> VerifyAsync(string challengeId, string code, CancellationToken cancellationToken = default) =>
            ValueTask.FromResult(IdentityLifecycleResults.Failed<bool>(IdentityLifecycleOutcome.ProviderUnavailable));
    }

    private sealed class MissingImpersonationService : IImpersonationService
    {
        public ValueTask<IdentityLifecycleResult<ImpersonationGrant>> StartAsync(ImpersonationAuthorizationRequest request, CancellationToken cancellationToken = default) =>
            ValueTask.FromResult(IdentityLifecycleResults.Failed<ImpersonationGrant>(IdentityLifecycleOutcome.PolicyDenied));
        public ValueTask<IdentityLifecycleOutcome> EndAsync(string grantId, CancellationToken cancellationToken = default) =>
            ValueTask.FromResult(IdentityLifecycleOutcome.PolicyDenied);
        public ValueTask<IdentityLifecycleResult<ImpersonationContext>> GetActiveAsync(string callerSubjectId, CancellationToken cancellationToken = default) =>
            ValueTask.FromResult(IdentityLifecycleResults.Failed<ImpersonationContext>(IdentityLifecycleOutcome.PolicyDenied));
    }
}
