using Microsoft.AspNetCore.Builder;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Options;
using Platform.Auditing.AspNetCore.Capture;
using Platform.Auditing.AspNetCore.Common;
using Platform.Auditing.AspNetCore.Middleware;
using Platform.Auditing.Contracts.DependencyInjection;

namespace Platform.Auditing.AspNetCore.DependencyInjection;

/// <summary>Registration helpers for the ASP.NET Core auditing adapter.</summary>
public static class ServiceCollectionExtensions
{
    /// <summary>Registers the audit middleware, subject resolver, and the contracts pipeline with default options.</summary>
    /// <param name="services">The service collection.</param>
    /// <returns>The same <paramref name="services"/> for chaining.</returns>
    public static IServiceCollection AddPlatformAuditingAspNetCore(this IServiceCollection services)
        => services.AddPlatformAuditingAspNetCore(_ => { });

    /// <summary>Registers the audit middleware and configures its HTTP capture options.</summary>
    /// <param name="services">The service collection.</param>
    /// <param name="configure">The HTTP audit options configuration delegate.</param>
    /// <returns>The same <paramref name="services"/> for chaining.</returns>
    public static IServiceCollection AddPlatformAuditingAspNetCore(this IServiceCollection services, Action<AuditAspNetCoreOptions> configure)
    {
        ArgumentNullException.ThrowIfNull(services);
        ArgumentNullException.ThrowIfNull(configure);
        services.AddPlatformAuditing();
        services.AddOptions<AuditAspNetCoreOptions>()
            .Configure(configure)
            .Validate(o => o.Validate().Count == 0, "AuditAspNetCoreOptions failed platform validation.");
        services.TryAddSingleton<IAuditSubjectResolver, HeaderAuditSubjectResolver>();
        services.TryAddSingleton<AuditMiddleware>();
        return services;
    }
}

/// <summary>Pipeline helpers for the ASP.NET Core auditing adapter.</summary>
public static class ApplicationBuilderExtensions
{
    /// <summary>Adds the audit capture middleware to the request pipeline.</summary>
    /// <param name="app">The application builder.</param>
    /// <returns>The same <paramref name="app"/> for chaining.</returns>
    public static IApplicationBuilder UsePlatformAuditing(this IApplicationBuilder app)
    {
        ArgumentNullException.ThrowIfNull(app);
        return app.UseMiddleware<AuditMiddleware>();
    }
}
