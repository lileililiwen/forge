namespace Platform.Domain;

/// <summary>
/// Opt-in marker that associates an entity with a tenant. The platform
/// exposes the marker data only; it applies no query filters, mappings,
/// or isolation behavior. Persistence and scope remain application-owned.
/// </summary>
public interface IHasTenant
{
    /// <summary>
    /// Gets the tenant identifier.
    /// </summary>
    string TenantId { get; }
}
