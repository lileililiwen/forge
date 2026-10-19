using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Platform.Caching.Contracts;
using Platform.Caching.InMemory;
using Platform.Caching.Keys;
using Platform.Core.Time;

namespace Platform.Caching.DependencyInjection;

/// <summary>Registration helpers for the provider-neutral local cache.</summary>
public static class ServiceCollectionExtensions
{
    /// <summary>Registers the in-memory cache and application-scoped key builder.</summary>
    public static IServiceCollection AddPlatformCaching(this IServiceCollection services, string application)
    {
        ArgumentNullException.ThrowIfNull(services);
        ArgumentException.ThrowIfNullOrWhiteSpace(application);
        services.TryAddSingleton<IClock, SystemClock>();
        services.AddSingleton(new CacheKeyBuilder(application));
        services.TryAddSingleton<ICacheStore, InMemoryCacheStore>();
        return services;
    }
}
