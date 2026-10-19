using Microsoft.EntityFrameworkCore;
using Platform.Persistence.EfCore.Deletion;
using Platform.Persistence.EfCore.Tenancy;

namespace Platform.Persistence.EfCore;

/// <summary>Explicit model configuration helpers.</summary>
public static class ModelBuilderExtensions
{
    /// <summary>Adds a soft-delete filter for one explicitly selected entity type.</summary>
    public static ModelBuilder ApplySoftDeleteFilter<TEntity>(this ModelBuilder modelBuilder)
        where TEntity : class, ISoftDeletable
    {
        ArgumentNullException.ThrowIfNull(modelBuilder);
        modelBuilder.Entity<TEntity>().HasQueryFilter(entity => !entity.IsDeleted);
        return modelBuilder;
    }

    /// <summary>Adds a tenant filter for one explicitly selected entity type and scope.</summary>
    public static ModelBuilder ApplyTenantFilter<TEntity>(this ModelBuilder modelBuilder, ITenantScope tenantScope)
        where TEntity : class, ITenantScoped
    {
        ArgumentNullException.ThrowIfNull(modelBuilder);
        ArgumentNullException.ThrowIfNull(tenantScope);
        modelBuilder.Entity<TEntity>().HasQueryFilter(entity => entity.TenantId == tenantScope.TenantId);
        return modelBuilder;
    }
}
