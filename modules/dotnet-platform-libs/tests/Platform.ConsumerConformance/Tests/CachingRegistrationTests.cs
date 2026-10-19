using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Options;
using Platform.Caching.Contracts;
using Platform.Caching.DependencyInjection;
using Platform.Caching.Keys;
using Platform.ConsumerConformance.Fixtures;

namespace Platform.ConsumerConformance.Tests;

public sealed class CachingRegistrationTests
{
    [Fact]
    public void AddPlatformCaching_registers_in_memory_store()
    {
        var services = ConsumerServiceCollectionFactory.CreateServices();
        services.AddPlatformCaching("test-app");

        using var provider = ConsumerServiceCollectionFactory.BuildProvider(services);
        var store = provider.GetRequiredService<ICacheStore>();
        Assert.NotNull(store);
    }

    [Fact]
    public void CacheKeyBuilder_builds_application_scoped_keys()
    {
        var key = new CacheKeyBuilder("test-app").ForApplication("user:42");
        Assert.Equal("test-app:a:user:42", key.Value);
    }

    [Fact]
    public void CacheKeyBuilder_builds_tenant_scoped_keys()
    {
        var key = new CacheKeyBuilder("test-app").ForTenant("tenant-1", "user:42");
        Assert.Equal("test-app:t:tenant-1:user:42", key.Value);
    }
}
