namespace Platform.Persistence.Multitenancy.ModelCustomization;

/// <summary>
/// Marks an entity that intentionally crosses tenants. Entities that
/// implement this marker are never wrapped in a tenant query filter
/// by the default model customizer. Entities that omit this marker
/// are filtered to the current tenant scope when default isolation
/// is enabled.
/// </summary>
public interface IGlobalTenantEntity;
