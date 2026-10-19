using Platform.Core.Tenancy;

namespace Platform.Persistence.Multitenancy;

/// <summary>
/// Default <see cref="IAmbientTenantScope"/> that is backed by the
/// <see cref="AmbientTenantScopeStore"/>. The instance is replaced on
/// every <see cref="ITenantScopeFactory"/> call, so its <see cref="Status"/>,
/// <see cref="Tenant"/>, and <see cref="Reason"/> reflect the most
/// recent installation.
/// </summary>
public sealed class AmbientTenantScope : IAmbientTenantScope
{
    /// <inheritdoc />
    public TenantResolutionStatus Status { get; set; } = TenantResolutionStatus.Unresolved;

    /// <inheritdoc />
    public ITenantInfo? Tenant { get; set; }

    /// <inheritdoc />
    public string? Reason { get; set; }

    /// <inheritdoc />
    public bool AllowsTenantAccess =>
        Status is TenantResolutionStatus.Resolved or TenantResolutionStatus.GlobalOperation;

    /// <summary>
    /// Monotonically increasing generation token. Consumers that cache
    /// data against the current scope observe this counter to detect
    /// replacement; when the generation changes the cached data is
    /// discarded and re-resolved. Set by <see cref="AmbientTenantScopeStore.Replace"/>
    /// on every installation; do not mutate it directly from application code.
    /// </summary>
    public int Generation { get; internal set; }

    /// <summary>Resets the scope to the unresolved state and bumps the generation.</summary>
    public void Reset()
    {
        Status = TenantResolutionStatus.Unresolved;
        Tenant = null;
        Reason = null;
        Generation++;
    }

    /// <summary>Bumps the generation token to invalidate cached consumers.</summary>
    public void BumpGeneration() => Generation++;
}
