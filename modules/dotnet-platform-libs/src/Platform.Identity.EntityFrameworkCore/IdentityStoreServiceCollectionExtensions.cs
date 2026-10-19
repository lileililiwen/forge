using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Platform.Identity.Contracts;

namespace Platform.Identity.EntityFrameworkCore;

/// <summary>Registration helpers for application-owned identity persistence.</summary>
public static class IdentityStoreServiceCollectionExtensions
{
    /// <summary>Registers an application-owned identity store used by the EF persistence adapter.</summary>
    public static IServiceCollection AddPlatformIdentityStore<TStore>(this IServiceCollection services)
        where TStore : class, IIdentityStore
    {
        ArgumentNullException.ThrowIfNull(services);
        services.TryAddScoped<IIdentityStore, TStore>();
        return services;
    }
}
