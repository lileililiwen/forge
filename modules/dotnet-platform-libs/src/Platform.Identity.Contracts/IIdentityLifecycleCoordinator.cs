namespace Platform.Identity.Contracts;

/// <summary>
/// Default composition of the identity lifecycle services. The platform provides
/// the orchestration; the application supplies a store, a verification provider,
/// an impersonation policy, and an audit hook.
/// </summary>
public interface IIdentityLifecycleCoordinator
{
    /// <summary>The refresh-token service.</summary>
    IRefreshTokenService RefreshTokens { get; }
    /// <summary>The password-recovery service.</summary>
    IPasswordRecoveryService PasswordRecovery { get; }
    /// <summary>The two-factor service.</summary>
    ITwoFactorService TwoFactor { get; }
    /// <summary>The impersonation service.</summary>
    IImpersonationService Impersonation { get; }
}
