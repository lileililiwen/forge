using Microsoft.AspNetCore.Http;
using Microsoft.Extensions.Options;
using Platform.Core.Tenancy;

namespace Platform.Persistence.Multitenancy.Http;

/// <summary>
/// ASP.NET Core middleware that resolves the tenant for the current
/// request and installs an <see cref="AmbientTenantScope"/> before the
/// request pipeline reaches tenant-scoped services. The middleware
/// never throws on missing tenants; it stores an
/// <see cref="Platform.Core.Tenancy.TenantResolutionStatus.Unresolved"/>
/// scope and lets the EF adapter enforce the documented
/// fail-closed behavior.
/// </summary>
public sealed class TenantScopeMiddleware
{
    private readonly RequestDelegate _next;

    /// <summary>Creates a new middleware.</summary>
    public TenantScopeMiddleware(RequestDelegate next)
    {
        _next = next ?? throw new ArgumentNullException(nameof(next));
    }

    /// <summary>Invokes the middleware for the current request.</summary>
    public async Task InvokeAsync(
        HttpContext context,
        AmbientTenantScopeStore store,
        ITenantResolver resolver,
        IOptions<MultitenancyOptions> optionsAccessor)
    {
        ArgumentNullException.ThrowIfNull(context);
        ArgumentNullException.ThrowIfNull(store);
        ArgumentNullException.ThrowIfNull(resolver);
        ArgumentNullException.ThrowIfNull(optionsAccessor);

        var options = optionsAccessor.Value;
        if (!options.EnableHttpScopeInstallation)
        {
            await _next(context);
            return;
        }
        var resolution = await resolver.ResolveAsync(context, context.RequestAborted);
        var scope = new AmbientTenantScope
        {
            Status = resolution.Status,
            Tenant = resolution.Tenant,
            Reason = resolution.Reason,
        };
        if (scope.Tenant is { } tenant && !IsAcceptable(tenant.Id, options))
        {
            scope = new AmbientTenantScope
            {
                Status = TenantResolutionStatus.Unresolved,
                Reason = "tenant_id_invalid",
            };
        }
        store.Replace(scope);

        await _next(context);
    }

    private static bool IsAcceptable(string tenantId, MultitenancyOptions options) =>
        !string.IsNullOrWhiteSpace(tenantId) && tenantId.Length <= options.MaxTenantIdLength;
}
