using Microsoft.Extensions.Caching.Hybrid;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Platform.Caching.Contracts;

namespace Platform.Caching.Hybrid.DependencyInjection;

/// <summary>Registration helpers for the optional HybridCache adapter.</summary>
public static class ServiceCollectionExtensions
{
    /// <summary>Registers HybridCache and exposes it through <see cref="ICacheStore"/>.</summary>
    public static IServiceCollection AddPlatformCachingHybrid(this IServiceCollection services, Action<HybridCacheOptions>? configure = null)
    {
        ArgumentNullException.ThrowIfNull(services);
        if (configure is null) services.AddHybridCache(); else services.AddHybridCache(configure);
        services.TryAddSingleton<ICacheStore, HybridCacheStore>();
        return services;
    }
}
