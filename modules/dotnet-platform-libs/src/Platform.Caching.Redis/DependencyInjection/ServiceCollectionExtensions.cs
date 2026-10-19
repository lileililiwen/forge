using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Platform.Caching.Contracts;
using StackExchange.Redis;

namespace Platform.Caching.Redis.DependencyInjection;

/// <summary>Registration helpers for the optional Redis cache adapter.</summary>
public static class ServiceCollectionExtensions
{
    /// <summary>Registers a Redis cache store over a shared multiplexer and serializer.</summary>
    public static IServiceCollection AddPlatformCachingRedis(this IServiceCollection services, IConnectionMultiplexer multiplexer, ICacheValueSerializer serializer, Action<RedisCacheOptions>? configure = null)
    {
        ArgumentNullException.ThrowIfNull(services);
        ArgumentNullException.ThrowIfNull(multiplexer);
        ArgumentNullException.ThrowIfNull(serializer);
        var options = new RedisCacheOptions();
        configure?.Invoke(options);
        services.TryAddSingleton<ICacheProviderStatus>(sp => sp.GetRequiredService<RedisCacheStore>());
        services.AddSingleton(sp => new RedisCacheStore(multiplexer, serializer, options));
        services.AddSingleton<ICacheStore>(sp => sp.GetRequiredService<RedisCacheStore>());
        return services;
    }
}
