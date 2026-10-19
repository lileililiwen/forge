using Microsoft.AspNetCore.Builder;

namespace Platform.Persistence.Multitenancy.Http;

/// <summary>Convenience helpers for wiring the multitenancy middleware into the pipeline.</summary>
public static class TenantScopeApplicationBuilderExtensions
{
    /// <summary>Installs the tenant scope middleware for the current request pipeline.</summary>
    public static IApplicationBuilder UsePlatformMultitenancy(this IApplicationBuilder app)
    {
        ArgumentNullException.ThrowIfNull(app);
        return app.UseMiddleware<TenantScopeMiddleware>();
    }
}
