using Microsoft.AspNetCore.Builder;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Platform.Core.Time;
using Platform.Quota.AspNetCore.Contracts;
using Platform.Quota.AspNetCore.Enforcement;
using Platform.Quota.AspNetCore.Resolvers;

namespace Platform.Quota.AspNetCore.DependencyInjection;

/// <summary>Registration and pipeline helpers for ASP.NET Core quota enforcement.</summary>
public static class ServiceCollectionExtensions
{
    /// <summary>
    /// Registers quota enforcement for ASP.NET Core. Requires <c>AddPlatformQuota</c> (or another
    /// <see cref="Platform.Quota.Contracts.IQuotaStore"/> registration) to be present. The application must register an
    /// <see cref="IQuotaResourceResolver"/> after this call; otherwise enforcement fails closed.
    /// </summary>
    public static IServiceCollection AddPlatformQuotaAspNetCore(this IServiceCollection services, Action<QuotaEnforcementOptions>? configure = null)
    {
        ArgumentNullException.ThrowIfNull(services);

        services.AddOptions<QuotaEnforcementOptions>().Configure(o =>
        {
            configure?.Invoke(o);
            o.Validate();
        });
        services.TryAddSingleton<IClock, SystemClock>();
        services.TryAddSingleton<IQuotaSubjectResolver, HeaderQuotaSubjectResolver>();
        services.TryAddSingleton<IQuotaResourceResolver, UnconfiguredQuotaResourceResolver>();
        services.TryAddSingleton<QuotaProblemDetailsWriter>();
        services.TryAddSingleton<PlatformQuotaMiddleware>();

        return services;
    }

    /// <summary>
    /// Inserts the quota enforcement middleware into the pipeline. Place it after authentication and
    /// rate limiting so only authenticated, counted calls consume quota. The middleware no-ops when
    /// <see cref="QuotaEnforcementOptions.Enabled"/> is false.
    /// </summary>
    public static IApplicationBuilder UsePlatformQuota(this IApplicationBuilder app)
    {
        ArgumentNullException.ThrowIfNull(app);
        return app.UseMiddleware<PlatformQuotaMiddleware>();
    }
}
