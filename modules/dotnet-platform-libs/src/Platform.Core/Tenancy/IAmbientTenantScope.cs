namespace Platform.Core.Tenancy;

/// <summary>
/// Ambient tenant scope available for the lifetime of a logical
/// operation. The scope can represent a resolved tenant, an explicit
/// global operation, or an unresolved state. Consumers read the
/// <see cref="Status"/> before performing tenant-scoped work.
/// </summary>
public interface IAmbientTenantScope
{
    /// <summary>Gets the current resolution status.</summary>
    TenantResolutionStatus Status { get; }

    /// <summary>
    /// Gets the resolved tenant when <see cref="Status"/> is
    /// <see cref="TenantResolutionStatus.Resolved"/>; otherwise <c>null</c>.
    /// </summary>
    ITenantInfo? Tenant { get; }

    /// <summary>
    /// Gets the short, stable reason for <see cref="TenantResolutionStatus.GlobalOperation"/>
    /// or <see cref="TenantResolutionStatus.Unresolved"/>. Never contains sensitive data.
    /// </summary>
    string? Reason { get; }

    /// <summary>
    /// Gets a value indicating whether tenant-scoped access is currently
    /// permitted. Returns <c>true</c> when a tenant is resolved or an
    /// explicit global operation is in effect.
    /// </summary>
    bool AllowsTenantAccess { get; }
}
