namespace Platform.Persistence.EfCore.Tenancy;

/// <summary>Marks an entity carrying an application-owned tenant identifier.</summary>
public interface ITenantScoped
{
    /// <summary>Gets or sets the tenant identifier.</summary>
    string? TenantId { get; set; }
}

/// <summary>Supplies the tenant used by an explicitly configured query filter.</summary>
public interface ITenantScope
{
    /// <summary>Gets the current tenant identifier, or <c>null</c> for an unscoped operation.</summary>
    string? TenantId { get; }
}
