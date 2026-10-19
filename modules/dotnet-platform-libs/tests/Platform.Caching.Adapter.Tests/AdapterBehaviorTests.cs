using Microsoft.Extensions.DependencyInjection;
using Platform.Caching.Contracts;
using Platform.Caching.Hybrid;
using Platform.Caching.Hybrid.DependencyInjection;
using Platform.Caching.Keys;
using Platform.Caching.Redis;
using Platform.Caching.Redis.DependencyInjection;
using StackExchange.Redis;

namespace Platform.Caching.Adapter.Tests;

public sealed class AdapterBehaviorTests
{
    [Fact]
    public async Task Hybrid_adapter_returns_hits_and_invalidates_tags()
    {
        using var provider = new ServiceCollection()
            .AddPlatformCachingHybrid()
            .BuildServiceProvider();
        var store = provider.GetRequiredService<ICacheStore>();
        var key = new CacheKey("catalog:t:tenant-a:products");
        var calls = 0;

        var first = await store.GetOrCreateAsync(key, _ => Task.FromResult(++calls), new CacheEntryOptions { Tags = ["products"] });
        var second = await store.GetOrCreateAsync(key, _ => Task.FromResult(++calls));

        Assert.Equal(CacheReadStatus.Hit, first.Status);
        Assert.Equal(1, first.Value);
        Assert.Equal(1, second.Value);
        Assert.Equal(1, calls);

        Assert.Equal(CacheOperationStatus.Succeeded, (await store.RemoveByTagAsync("products")).Status);
        await store.GetOrCreateAsync(key, _ => Task.FromResult(++calls));
        Assert.Equal(2, calls);
    }

    [Fact]
    public async Task Redis_adapter_reports_unavailable_backend_without_exposing_connection_details()
    {
        using var multiplexer = await ConnectionMultiplexer.ConnectAsync("localhost:6399,connectTimeout=100,abortConnect=false");
        var store = new RedisCacheStore(multiplexer, new ByteSerializer(), new RedisCacheOptions { OperationTimeout = TimeSpan.FromMilliseconds(150) });

        var result = await store.GetAsync<string>(new CacheKey("catalog:a:missing"));

        Assert.Equal(CacheReadStatus.Unavailable, result.Status);
        Assert.NotNull(result.Failure);
        Assert.Equal("cache.provider_unavailable", result.Failure!.Code);
        Assert.DoesNotContain("6399", result.Failure.Message);
        Assert.Equal(CacheProviderState.Unavailable, store.Status.State);
    }

    [Fact]
    public void Redis_registration_exposes_the_same_store_and_status_provider()
    {
        using var multiplexer = ConnectionMultiplexer.Connect("localhost:6399,connectTimeout=100,abortConnect=false");
        var services = new ServiceCollection();
        services.AddPlatformCachingRedis(multiplexer, new ByteSerializer());
        using var provider = services.BuildServiceProvider();

        Assert.Same(provider.GetRequiredService<RedisCacheStore>(), provider.GetRequiredService<ICacheStore>());
        Assert.Same(provider.GetRequiredService<RedisCacheStore>(), provider.GetRequiredService<ICacheProviderStatus>());
    }

    private sealed class ByteSerializer : ICacheValueSerializer
    {
        public byte[] Serialize<T>(T value) => System.Text.Encoding.UTF8.GetBytes(value?.ToString() ?? string.Empty);
        public T? Deserialize<T>(byte[] payload) => default;
    }
}
