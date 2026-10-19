namespace Platform.Persistence.EfCore;

/// <summary>Opt-in switches for provider-neutral persistence conventions.</summary>
public sealed class PersistenceOptions
{
    /// <summary>Enables audit values during save interception.</summary>
    public bool EnableAuditInterception { get; set; }

    /// <summary>Enables soft-delete conversion during save interception.</summary>
    public bool EnableSoftDeleteInterception { get; set; }

    /// <summary>Indicates that consumers configured tenant query filters explicitly.</summary>
    public bool EnableTenantQueryFilters { get; set; }
}
