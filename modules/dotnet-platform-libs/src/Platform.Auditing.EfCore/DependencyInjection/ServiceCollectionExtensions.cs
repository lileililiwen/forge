using Microsoft.EntityFrameworkCore;
using Microsoft.EntityFrameworkCore.Diagnostics;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Platform.Auditing.EfCore.Interceptors;
using Platform.Auditing.Contracts.DependencyInjection;

namespace Platform.Auditing.EfCore.DependencyInjection;

/// <summary>Registration helpers for the EF Core auditing adapter.</summary>
public static class ServiceCollectionExtensions
{
    /// <summary>Registers the auditing save-changes interceptor and ensures the contracts pipeline is present.</summary>
    /// <param name="services">The service collection.</param>
    /// <returns>The same <paramref name="services"/> for chaining.</returns>
    public static IServiceCollection AddPlatformAuditingEfCore(this IServiceCollection services)
    {
        ArgumentNullException.ThrowIfNull(services);
        services.AddPlatformAuditing();
        services.TryAddSingleton<ISaveChangesInterceptor, AuditingSaveChangesInterceptor>();
        return services;
    }
}
