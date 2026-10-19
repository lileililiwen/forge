using System.Linq.Expressions;
using Microsoft.EntityFrameworkCore;
using Platform.Core.Tenancy;
using Platform.Persistence.EfCore.Tenancy;

namespace Platform.Persistence.Multitenancy.ModelCustomization;

/// <summary>
/// Explicit model configuration helpers that apply default tenant
/// isolation to <see cref="ITenantScoped"/> entities that do not opt
/// out via <see cref="IGlobalTenantEntity"/>. Applications call
/// <c>ApplyDefaultTenantFilters</c> from their
/// <c>OnModelCreating</c> when they want the default-on behavior.
/// </summary>
public static class TenantModelBuilderExtensions
{
    /// <summary>
    /// Applies the default tenant query filter to every
    /// <see cref="ITenantScoped"/> entity that does not opt out via
    /// <see cref="IGlobalTenantEntity"/>.
    /// </summary>
    /// <param name="modelBuilder">The model builder.</param>
    /// <param name="scope">The current ambient tenant scope.</param>
    /// <returns>The same <paramref name="modelBuilder"/> for chaining.</returns>
    public static ModelBuilder ApplyDefaultTenantFilters(
        this ModelBuilder modelBuilder,
        IAmbientTenantScope scope)
    {
        ArgumentNullException.ThrowIfNull(modelBuilder);
        ArgumentNullException.ThrowIfNull(scope);
        return modelBuilder.ApplyDefaultTenantFilters(scope, GlobalFilterBehavior.Apply);
    }

    /// <summary>
    /// Applies the default tenant query filter with the supplied
    /// behavior override. When
    /// <see cref="GlobalFilterBehavior.IgnoreGlobalScope"/> is
    /// supplied the customizer still filters tenant-scoped entities
    /// to the explicit <paramref name="overrideTenantId"/>; when
    /// <see cref="GlobalFilterBehavior.Skip"/> is supplied the
    /// customizer is a no-op and lets the application configure
    /// filters manually.
    /// </summary>
    public static ModelBuilder ApplyDefaultTenantFilters(
        this ModelBuilder modelBuilder,
        IAmbientTenantScope scope,
        GlobalFilterBehavior behavior,
        string? overrideTenantId = null)
    {
        ArgumentNullException.ThrowIfNull(modelBuilder);
        ArgumentNullException.ThrowIfNull(scope);
        if (behavior == GlobalFilterBehavior.Skip) return modelBuilder;

        var tenantId = behavior switch
        {
            GlobalFilterBehavior.Apply when scope.Status == TenantResolutionStatus.Resolved
                => scope.Tenant!.Id,
            GlobalFilterBehavior.Apply when scope.Status == TenantResolutionStatus.GlobalOperation
                => null,
            GlobalFilterBehavior.IgnoreGlobalScope => overrideTenantId,
            _ => scope.Tenant?.Id,
        };
        var isGlobal = behavior == GlobalFilterBehavior.Apply
            && scope.Status == TenantResolutionStatus.GlobalOperation;

        foreach (var entityType in modelBuilder.Model.GetEntityTypes())
        {
            if (typeof(IGlobalTenantEntity).IsAssignableFrom(entityType.ClrType)) continue;
            if (!typeof(ITenantScoped).IsAssignableFrom(entityType.ClrType)) continue;
            BuildFilter(modelBuilder, entityType.ClrType, tenantId, isGlobal);
        }
        return modelBuilder;
    }

    private static void BuildFilter(ModelBuilder modelBuilder, Type entityType, string? tenantId, bool isGlobal)
    {
        var parameter = Expression.Parameter(entityType, "e");
        var property = Expression.Property(parameter, nameof(ITenantScoped.TenantId));
        var constant = Expression.Constant(tenantId, typeof(string));
        Expression comparison = isGlobal
            ? Expression.NotEqual(property, constant)
            : Expression.Equal(property, constant);
        var lambda = Expression.Lambda(comparison, parameter);
        modelBuilder.Entity(entityType).HasQueryFilter(lambda);
    }
}

/// <summary>Controls how the <c>ApplyDefaultTenantFilters</c> helpers handle global operations.</summary>
public enum GlobalFilterBehavior
{
    /// <summary>Apply the current ambient scope: resolved tenant filters to its id; global operation allows all rows.</summary>
    Apply = 0,

    /// <summary>Apply the supplied override tenant id regardless of the ambient scope.</summary>
    IgnoreGlobalScope = 1,

    /// <summary>Skip the customizer entirely; the application configures filters manually.</summary>
    Skip = 2,
}
