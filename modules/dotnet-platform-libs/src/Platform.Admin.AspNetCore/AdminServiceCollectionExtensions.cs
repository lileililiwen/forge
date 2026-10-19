using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Platform.Admin.Contracts;
using Platform.Identity.AspNetCore;

namespace Platform.Admin.AspNetCore;

/// <summary>Registers the optional administrative capability.</summary>
public static class AdminServiceCollectionExtensions
{
    /// <summary>Registers administration options and authorization policies.</summary>
    public static IServiceCollection AddPlatformAdmin(this IServiceCollection services) => services.AddPlatformAdmin(_ => { });

    /// <summary>Registers administration options and authorization policies.</summary>
    public static IServiceCollection AddPlatformAdmin(this IServiceCollection services, Action<AdminOptions> configure)
    {
        ArgumentNullException.ThrowIfNull(services);
        ArgumentNullException.ThrowIfNull(configure);
        services.AddOptions<AdminOptions>().Configure(configure).Validate(o => o.IsValid(), "Platform admin options are invalid.");
        services.AddPlatformIdentity();
        services.TryAddSingleton<IAdminTenantScope, DefaultAdminTenantScope>();
        services.TryAddSingleton<IAdminAuditSink, NullAdminAuditSink>();
        foreach (var permission in new[] { AdminPermissions.UsersRead, AdminPermissions.UsersManage, AdminPermissions.RolesRead, AdminPermissions.RolesManage, AdminPermissions.PermissionsRead, AdminPermissions.SessionsRead, AdminPermissions.SessionsManage, AdminPermissions.AuditRead, AdminPermissions.ProvidersRead, AdminPermissions.SubscriptionsRead, AdminPermissions.Impersonation })
            services.RequirePlatformPermission(permission);
        return services;
    }
}

internal sealed class DefaultAdminTenantScope : IAdminTenantScope
{
    public bool CanAccess(string? operatorTenantId, string? targetTenantId) => targetTenantId is null
        || string.Equals(operatorTenantId, targetTenantId, StringComparison.Ordinal);
}

internal sealed class NullAdminAuditSink : IAdminAuditSink
{
    public ValueTask RecordAsync(AdminAuditEntry auditEntry, CancellationToken cancellationToken = default) => ValueTask.CompletedTask;
}
