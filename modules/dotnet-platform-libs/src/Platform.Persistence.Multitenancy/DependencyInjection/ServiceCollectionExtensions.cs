using Microsoft.AspNetCore.Builder;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Diagnostics.HealthChecks;
using Platform.Core.Tenancy;
using Platform.Persistence.Multitenancy.Http;
using Platform.Persistence.Multitenancy.Readiness;

namespace Platform.Persistence.Multitenancy.DependencyInjection;

/// <summary>
/// Registration helpers for the multitenancy adapter. Every default
/// is registered with <c>TryAdd</c> so applications can replace
/// the resolver, connection resolver, ambient scope, or readiness
/// probe before calling the registration method on this class.
/// </summary>
public static class ServiceCollectionExtensions
{
    /// <summary>Registers the multitenancy adapter with safe defaults.</summary>
    public static IServiceCollection AddPlatformPersistenceMultitenancy(this IServiceCollection services) =>
        services.AddPlatformPersistenceMultitenancy(_ => { });

    /// <summary>Registers the multitenancy adapter and configures its options.</summary>
    public static IServiceCollection AddPlatformPersistenceMultitenancy(
        this IServiceCollection services,
        Action<MultitenancyOptions> configure)
    {
        ArgumentNullException.ThrowIfNull(services);
        ArgumentNullException.ThrowIfNull(configure);
        services.AddOptions<MultitenancyOptions>()
            .Configure(configure)
            .Validate(o => o.Validate().Count == 0, "Multitenancy options are invalid.");
        services.TryAddSingleton<AmbientTenantScopeStore>();
        services.TryAddSingleton<ITenantScopeAccessor, TenantScopeAccessor>();
        services.TryAddSingleton<ITenantScopeFactory, TenantScopeFactory>();
        return services;
    }

    /// <summary>Adds the multitenancy HTTP middleware to the request pipeline.</summary>
    public static IApplicationBuilder UsePlatformMultitenancy(this IApplicationBuilder app) =>
        TenantScopeApplicationBuilderExtensions.UsePlatformMultitenancy(app);

    /// <summary>Adds the tenant connection readiness check to the supplied health checks builder.</summary>
    /// <param name="builder">The health checks builder.</param>
    /// <param name="tenants">The tenants whose connections will be probed.</param>
    /// <param name="probe">The application-owned probe implementation.</param>
    /// <param name="name">The name of the registration.</param>
    public static IHealthChecksBuilder AddPlatformTenantConnectionReadinessCheck(
        this IHealthChecksBuilder builder,
        IEnumerable<ITenantInfo> tenants,
        ITenantConnectionReadinessProbe probe,
        string name = "platform.tenant-connections")
    {
        ArgumentNullException.ThrowIfNull(builder);
        ArgumentNullException.ThrowIfNull(tenants);
        ArgumentNullException.ThrowIfNull(probe);
        return builder.AddCheck(
            name,
            new TenantConnectionReadinessCheck(tenants, probe),
            tags: TenantConnectionReadinessCheck.Tag);
    }
}
