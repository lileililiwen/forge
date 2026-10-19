using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Platform.Storage.Contracts;

namespace Platform.Storage.DependencyInjection;

/// <summary>Registration helpers for storage contracts.</summary>
public static class ServiceCollectionExtensions
{
    /// <summary>Registers an application-provided object storage implementation.</summary>
    public static IServiceCollection AddPlatformStorage<TStorage>(this IServiceCollection services)
        where TStorage : class, IObjectStorage
    {
        ArgumentNullException.ThrowIfNull(services);
        services.TryAddScoped<IObjectStorage, TStorage>();
        return services;
    }
}
