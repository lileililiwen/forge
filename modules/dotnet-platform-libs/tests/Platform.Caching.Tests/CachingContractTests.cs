using Platform.Caching.Contracts;
using Platform.Caching.InMemory;
using Platform.Caching.DependencyInjection;
using Microsoft.Extensions.DependencyInjection;
using Platform.Caching.Keys;
using Platform.Caching.Telemetry;
using Platform.Core.Time;

namespace Platform.Caching.Tests;

public sealed class CachingContractTests
{
    [Fact]
    public void Key_builder_isolates_tenants_and_rejects_invalid_values()
    {
        var builder = new CacheKeyBuilder("catalog");

        var first = builder.ForTenant("tenant-a", "products:active");
        var second = builder.ForTenant("tenant-b", "products:active");

        Assert.NotEqual(first, second);
        Assert.Equal("catalog:t:tenant-a:products:active", first.Value);
        Assert.Throws<ArgumentException>(() => builder.ForTenant("tenant a", "products"));
        Assert.Throws<ArgumentException>(() => builder.ForApplication(" "));
    }

    [Fact]
    public void Options_validate_absolute_expiration_tags_and_limits()
    {
        var options = new CacheEntryOptions
        {
            AbsoluteExpirationRelativeToNow = TimeSpan.FromMinutes(2),
            Tags = ["products", "tenant:tenant-a"]
        };

        options.Validate();
        Assert.Throws<ArgumentOutOfRangeException>(() => new CacheEntryOptions { AbsoluteExpirationRelativeToNow = TimeSpan.Zero }.Validate());
        Assert.Throws<ArgumentException>(() => new CacheEntryOptions { Tags = ["raw key with spaces"] }.Validate());
    }

    [Fact]
    public void Telemetry_names_do_not_include_keys_or_payloads()
    {
        Assert.Equal("platform.cache", CacheTelemetry.ActivitySourceName);
        Assert.Equal("cache.get_or_create", CacheTelemetry.GetOrCreateOperation);
        Assert.DoesNotContain("key", CacheTelemetry.GetOrCreateOperation, StringComparison.OrdinalIgnoreCase);
        Assert.DoesNotContain("value", CacheTelemetry.GetOrCreateOperation, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public async Task In_memory_store_supports_hit_miss_expiry_and_tag_invalidation()
    {
        var clock = new MutableClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        var store = new InMemoryCacheStore(clock);
        var key = new CacheKey("catalog:t:tenant-a:products");
        var options = new CacheEntryOptions { AbsoluteExpirationRelativeToNow = TimeSpan.FromMinutes(1), Tags = ["products"] };

        Assert.Equal(CacheReadStatus.Miss, (await store.GetAsync<string>(key)).Status);
        Assert.Equal(CacheOperationStatus.Succeeded, (await store.SetAsync(key, "value", options)).Status);
        Assert.Equal("value", (await store.GetAsync<string>(key)).Value);

        await store.RemoveByTagAsync("products");
        Assert.Equal(CacheReadStatus.Miss, (await store.GetAsync<string>(key)).Status);

        await store.SetAsync(key, "value", options);
        clock.Advance(TimeSpan.FromMinutes(2));
        Assert.Equal(CacheReadStatus.Miss, (await store.GetAsync<string>(key)).Status);
    }

    [Fact]
    public void Base_registration_is_local_and_overrideable()
    {
        var services = new ServiceCollection().AddPlatformCaching("catalog");
        using var provider = services.BuildServiceProvider();

        Assert.IsType<InMemoryCacheStore>(provider.GetRequiredService<ICacheStore>());
        Assert.Equal("catalog:a:key", provider.GetRequiredService<CacheKeyBuilder>().ForApplication("key").Value);
    }

    private sealed class MutableClock(DateTimeOffset initial) : IClock
    {
        public DateTimeOffset UtcNow { get; private set; } = initial;
        public void Advance(TimeSpan duration) => UtcNow = UtcNow.Add(duration);
    }
}
