using Microsoft.Extensions.Options;
using Platform.Core.Tenancy;
using Platform.Persistence.Multitenancy;

namespace Platform.Persistence.Multitenancy.Tests;

public sealed class TenantScopeFactoryTests
{
    [Fact]
    public void Unresolved_scope_is_the_default_before_any_installation()
    {
        var (factory, store, _) = CreateFactory();
        _ = factory;

        Assert.Equal(TenantResolutionStatus.Unresolved, store.Current.Status);
        Assert.False(store.Current.AllowsTenantAccess);
    }

    [Fact]
    public void Begin_tenant_installs_resolved_scope_and_restores_on_dispose()
    {
        var (factory, store, _) = CreateFactory();
        var tenant = new StaticTenant("tenant-a");

        using (factory.BeginTenant(tenant))
        {
            Assert.Equal(TenantResolutionStatus.Resolved, store.Current.Status);
            Assert.Same(tenant, store.Current.Tenant);
        }

        Assert.Equal(TenantResolutionStatus.Unresolved, store.Current.Status);
    }

    [Fact]
    public void Begin_global_operation_installs_global_scope_and_restores_on_dispose()
    {
        var (factory, store, _) = CreateFactory();

        using (factory.BeginGlobalOperation("background_reindex"))
        {
            Assert.Equal(TenantResolutionStatus.GlobalOperation, store.Current.Status);
            Assert.Equal("background_reindex", store.Current.Reason);
            Assert.True(store.Current.AllowsTenantAccess);
        }

        Assert.Equal(TenantResolutionStatus.Unresolved, store.Current.Status);
    }

    [Fact]
    public void Nested_installs_restore_to_the_previous_scope()
    {
        var (factory, store, _) = CreateFactory();
        var first = new StaticTenant("tenant-a");
        var second = new StaticTenant("tenant-b");

        using (factory.BeginTenant(first))
        {
            using (factory.BeginTenant(second))
            {
                Assert.Same(second, store.Current.Tenant);
            }
            Assert.Same(first, store.Current.Tenant);
        }
    }

    [Fact]
    public void Begin_tenant_rejects_empty_or_whitespace_identifier()
    {
        var (factory, _, _) = CreateFactory();
        Assert.Throws<ArgumentException>(() => factory.BeginTenant(new StaticTenant("")));
        Assert.Throws<ArgumentException>(() => factory.BeginTenant(new StaticTenant(" ")));
    }

    [Fact]
    public void Begin_tenant_rejects_identifier_longer_than_max()
    {
        var options = new MultitenancyOptions { MaxTenantIdLength = 4 };
        var (factory, _, _) = CreateFactory(options);

        Assert.Throws<ArgumentException>(() => factory.BeginTenant(new StaticTenant(new string('a', 5))));
    }

    [Fact]
    public void Begin_global_operation_requires_reason()
    {
        var (factory, _, _) = CreateFactory();
        Assert.Throws<ArgumentException>(() => factory.BeginGlobalOperation(""));
    }

    [Fact]
    public void Begin_tenant_throws_for_null_tenant()
    {
        var (factory, _, _) = CreateFactory();
        Assert.Throws<ArgumentNullException>(() => factory.BeginTenant(null!));
    }

    [Fact]
    public async Task Restoring_isolates_concurrent_async_flows()
    {
        var (factory, store, _) = CreateFactory();
        var tenantA = new StaticTenant("tenant-a");
        var tenantB = new StaticTenant("tenant-b");
        var observed = new System.Collections.Concurrent.ConcurrentBag<string>();

        var taskA = Task.Run(async () =>
        {
            using (factory.BeginTenant(tenantA))
            {
                await Task.Delay(50);
                observed.Add(store.Current.Tenant?.Id ?? "<null>");
            }
        });
        var taskB = Task.Run(async () =>
        {
            using (factory.BeginTenant(tenantB))
            {
                await Task.Delay(20);
                observed.Add(store.Current.Tenant?.Id ?? "<null>");
            }
        });

        await Task.WhenAll(taskA, taskB);

        Assert.Equal(2, observed.Count);
        Assert.Contains("tenant-a", observed);
        Assert.Contains("tenant-b", observed);
    }

    private static (TenantScopeFactory Factory, AmbientTenantScopeStore Store, MultitenancyOptions Options) CreateFactory(MultitenancyOptions? options = null)
    {
        options ??= new MultitenancyOptions();
        var store = new AmbientTenantScopeStore();
        return (new TenantScopeFactory(store, options), store, options);
    }

    private sealed record StaticTenant(string Id) : ITenantInfo
    {
        public string? Name => Id;
    }
}
