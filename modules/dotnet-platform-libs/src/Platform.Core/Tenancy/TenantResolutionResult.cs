namespace Platform.Core.Tenancy;

/// <summary>Describes the outcome of a tenant resolution attempt.</summary>
public enum TenantResolutionStatus
{
    /// <summary>The resolution succeeded and a tenant is in effect.</summary>
    Resolved = 0,

    /// <summary>The application explicitly opened a global operation scope.</summary>
    GlobalOperation = 1,

    /// <summary>The resolver could not determine a tenant for the request.</summary>
    Unresolved = 2,
}

/// <summary>
/// Result of resolving the ambient tenant for the current operation.
/// Either a resolved tenant, an intentional global operation, or an
/// unresolved state accompanied by a stable reason for diagnostics.
/// </summary>
/// <param name="Status">The documented resolution status.</param>
/// <param name="Tenant">The resolved tenant when <paramref name="Status"/> is
/// <see cref="TenantResolutionStatus.Resolved"/>; otherwise <c>null</c>.</param>
/// <param name="Reason">A short, stable reason code used by logs and the
/// adapter diagnostics surface. Never contains sensitive data.</param>
public readonly record struct TenantResolutionResult(
    TenantResolutionStatus Status,
    ITenantInfo? Tenant,
    string? Reason)
{
    /// <summary>Builds a successful resolution outcome.</summary>
    public static TenantResolutionResult Resolved(ITenantInfo tenant) =>
        new(TenantResolutionStatus.Resolved, tenant, null);

    /// <summary>Builds an explicit global-operation outcome.</summary>
    public static TenantResolutionResult GlobalOperation(string reason) =>
        new(TenantResolutionStatus.GlobalOperation, null, reason);

    /// <summary>Builds an unresolved outcome with a stable reason.</summary>
    public static TenantResolutionResult Unresolved(string reason) =>
        new(TenantResolutionStatus.Unresolved, null, reason);
}
