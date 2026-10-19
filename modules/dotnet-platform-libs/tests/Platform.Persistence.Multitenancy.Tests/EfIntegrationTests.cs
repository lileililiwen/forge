using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.DependencyInjection;
using Platform.Core.Tenancy;
using Platform.Persistence.EfCore.Tenancy;
using Platform.Persistence.Multitenancy;
using Platform.Persistence.Multitenancy.Connections;
using Platform.Persistence.Multitenancy.ModelCustomization;

namespace Platform.Persistence.Multitenancy.Tests;

public sealed class EfIntegrationTests
{
    [Fact]
    public void Model_builder_applies_filter_to_each_tenant_scoped_entity_type()
    {
        var (scope, _, factory) = CreateAmbient();
        var databaseName = Guid.NewGuid().ToString();

        using (factory.BeginTenant(new StaticTenant("tenant-a")))
        {
            using var db = new TenantDbContext(databaseName, scope);
            var entity = db.Model.FindEntityType(typeof(TenantEntity));
            Assert.NotNull(entity!.GetQueryFilter());
        }
    }

    [Fact]
    public void Model_builder_does_not_filter_global_entities()
    {
        var (scope, _, factory) = CreateAmbient();
        var databaseName = Guid.NewGuid().ToString();

        using (factory.BeginTenant(new StaticTenant("tenant-a")))
        {
            using var db = new TenantDbContext(databaseName, scope);
            var entity = db.Model.FindEntityType(typeof(GlobalEntity));
            Assert.Null(entity!.GetQueryFilter());
        }
    }

    [Fact]
    public void Model_builder_uses_explicit_override_when_global_scope_is_ignored()
    {
        var (scope, _, _) = CreateAmbient();
        var databaseName = Guid.NewGuid().ToString();

        using var db = new OverrideTenantDbContext(databaseName, scope, "tenant-x");
        var entity = db.Model.FindEntityType(typeof(TenantEntity));
        Assert.NotNull(entity!.GetQueryFilter());
    }

    [Fact]
    public void Model_builder_skips_filter_application_when_behavior_is_skip()
    {
        var (scope, _, _) = CreateAmbient();
        var databaseName = Guid.NewGuid().ToString();

        using var db = new SkipTenantDbContext(databaseName, scope);
        var entity = db.Model.FindEntityType(typeof(TenantEntity));
        Assert.Null(entity!.GetQueryFilter());
    }

    [Fact]
    public async Task Scoped_connection_provider_returns_descriptor_for_tenant()
    {
        var (_, store, factory) = CreateAmbient();
        var resolver = new StaticConnectionResolver(
            ("tenant-a", "Data Source=tenant-a.db"),
            ("tenant-b", "Data Source=tenant-b.db"));
        using var provider = new ScopedTenantConnectionProvider(store, resolver, new MultitenancyOptions());

        using (factory.BeginTenant(new StaticTenant("tenant-a")))
        {
            var descriptor = await provider.GetConnectionAsync();
            Assert.Equal("tenant-a", descriptor.Name);
            Assert.Equal("Data Source=tenant-a.db", descriptor.ConnectionString);
        }

        using (factory.BeginTenant(new StaticTenant("tenant-b")))
        {
            var descriptor = await provider.GetConnectionAsync();
            Assert.Equal("tenant-b", descriptor.Name);
        }
    }

    [Fact]
    public async Task Scoped_connection_provider_returns_shared_descriptor_for_global_scope()
    {
        var (_, store, factory) = CreateAmbient();
        var resolver = new StaticConnectionResolver(
            (null, "Data Source=shared.db"));
        using var provider = new ScopedTenantConnectionProvider(store, resolver, new MultitenancyOptions());

        using (factory.BeginGlobalOperation("provision"))
        {
            var descriptor = await provider.GetConnectionAsync();
            Assert.Equal("shared", descriptor.Name);
        }
    }

    [Fact]
    public async Task Scoped_connection_provider_fails_closed_when_scope_unresolved()
    {
        var (_, store, _) = CreateAmbient();
        var resolver = new StaticConnectionResolver(("tenant-a", "Data Source=tenant-a.db"));
        using var provider = new ScopedTenantConnectionProvider(store, resolver, new MultitenancyOptions());

        await Assert.ThrowsAsync<TenantScopeNotResolvedException>(() => provider.GetConnectionAsync().AsTask());
    }

    [Fact]
    public async Task Scoped_connection_provider_returns_shared_when_fail_closed_disabled()
    {
        var (_, store, _) = CreateAmbient();
        var resolver = new StaticConnectionResolver((null, "Data Source=shared.db"));
        var options = new MultitenancyOptions { FailClosedOnMissingScope = false };
        using var provider = new ScopedTenantConnectionProvider(store, resolver, options);

        var descriptor = await provider.GetConnectionAsync();
        Assert.Equal("shared", descriptor.Name);
    }

    [Fact]
    public async Task Scoped_connection_provider_caches_within_a_single_scope()
    {
        var (_, store, factory) = CreateAmbient();
        var resolver = new CountingConnectionResolver(("tenant-a", "Data Source=tenant-a.db"));
        using var provider = new ScopedTenantConnectionProvider(store, resolver, new MultitenancyOptions());

        using (factory.BeginTenant(new StaticTenant("tenant-a")))
        {
            var first = await provider.GetConnectionAsync();
            var second = await provider.GetConnectionAsync();
            Assert.Same(first, second);
            Assert.Equal(1, resolver.CallCount);
        }
    }

    private static (IAmbientTenantScope Scope, AmbientTenantScopeStore Store, TenantScopeFactory Factory) CreateAmbient()
    {
        var store = new AmbientTenantScopeStore();
        var factory = new TenantScopeFactory(store, new MultitenancyOptions());
        return (store.Current, store, factory);
    }

    private sealed record StaticTenant(string Id) : ITenantInfo
    {
        public string? Name => Id;
    }

    private sealed class StaticConnectionResolver : ITenantConnectionResolver
    {
        private readonly Dictionary<string, string> _byTenant = new(StringComparer.Ordinal);
        private string? _shared;

        public StaticConnectionResolver(params (string? TenantId, string ConnectionString)[] entries)
        {
            foreach (var entry in entries)
            {
                if (entry.TenantId is null) _shared = entry.ConnectionString;
                else _byTenant[entry.TenantId] = entry.ConnectionString;
            }
        }

        public ValueTask<TenantConnectionDescriptor> ResolveConnectionAsync(ITenantInfo? tenant, CancellationToken cancellationToken = default)
        {
            if (tenant is null)
            {
                if (_shared is null)
                    throw new InvalidOperationException("No shared connection configured.");
                return ValueTask.FromResult(new TenantConnectionDescriptor("shared", _shared, "Microsoft.EntityFrameworkCore.Sqlite"));
            }
            if (!_byTenant.TryGetValue(tenant.Id, out var connectionString))
                throw new InvalidOperationException($"No connection configured for tenant {tenant.Id}.");
            return ValueTask.FromResult(new TenantConnectionDescriptor(tenant.Id, connectionString, "Microsoft.EntityFrameworkCore.Sqlite"));
        }
    }

    private sealed class CountingConnectionResolver : ITenantConnectionResolver
    {
        private readonly string _connectionString;
        public int CallCount { get; private set; }

        public CountingConnectionResolver(params (string? TenantId, string ConnectionString)[] entries)
        {
            _connectionString = entries[0].ConnectionString;
        }

        public ValueTask<TenantConnectionDescriptor> ResolveConnectionAsync(ITenantInfo? tenant, CancellationToken cancellationToken = default)
        {
            CallCount++;
            return ValueTask.FromResult(new TenantConnectionDescriptor(
                tenant?.Id ?? "shared",
                _connectionString,
                "Microsoft.EntityFrameworkCore.Sqlite"));
        }
    }

    private sealed class TenantEntity : ITenantScoped
    {
        public int Id { get; set; }
        public string? TenantId { get; set; }
        public string? Name { get; set; }
    }

    private sealed class GlobalEntity : ITenantScoped, IGlobalTenantEntity
    {
        public int Id { get; set; }
        public string? TenantId { get; set; }
        public string? Note { get; set; }
    }

    private sealed class TenantDbContext : DbContext
    {
        private readonly string _databaseName;
        private readonly IAmbientTenantScope _scope;
        private readonly GlobalFilterBehavior _behavior;
        private readonly string? _overrideTenantId;

        public TenantDbContext(string databaseName, IAmbientTenantScope scope)
            : this(databaseName, scope, GlobalFilterBehavior.Apply, null) { }

        public TenantDbContext(string databaseName, IAmbientTenantScope scope, GlobalFilterBehavior behavior, string? overrideTenantId = null)
        {
            _databaseName = databaseName;
            _scope = scope;
            _behavior = behavior;
            _overrideTenantId = overrideTenantId;
        }

        public DbSet<TenantEntity> Entities => Set<TenantEntity>();
        public DbSet<GlobalEntity> GlobalEntities => Set<GlobalEntity>();

        protected override void OnConfiguring(DbContextOptionsBuilder optionsBuilder)
        {
            optionsBuilder.UseInMemoryDatabase(_databaseName);
        }

        protected override void OnModelCreating(ModelBuilder modelBuilder)
        {
            modelBuilder.ApplyDefaultTenantFilters(_scope, _behavior, _overrideTenantId);
            modelBuilder.Entity<TenantEntity>().HasKey(e => e.Id);
            modelBuilder.Entity<GlobalEntity>().HasKey(e => e.Id);
        }
    }

    private sealed class SkipTenantDbContext : DbContext
    {
        private readonly string _databaseName;
        private readonly IAmbientTenantScope _scope;

        public SkipTenantDbContext(string databaseName, IAmbientTenantScope scope)
        {
            _databaseName = databaseName;
            _scope = scope;
        }

        public DbSet<TenantEntity> Entities => Set<TenantEntity>();
        public DbSet<GlobalEntity> GlobalEntities => Set<GlobalEntity>();

        protected override void OnConfiguring(DbContextOptionsBuilder optionsBuilder)
        {
            optionsBuilder.UseInMemoryDatabase(_databaseName);
        }

        protected override void OnModelCreating(ModelBuilder modelBuilder)
        {
            modelBuilder.ApplyDefaultTenantFilters(_scope, GlobalFilterBehavior.Skip);
            modelBuilder.Entity<TenantEntity>().HasKey(e => e.Id);
            modelBuilder.Entity<GlobalEntity>().HasKey(e => e.Id);
        }
    }

    private sealed class OverrideTenantDbContext : DbContext
    {
        private readonly string _databaseName;
        private readonly IAmbientTenantScope _scope;
        private readonly string _overrideTenantId;

        public OverrideTenantDbContext(string databaseName, IAmbientTenantScope scope, string overrideTenantId)
        {
            _databaseName = databaseName;
            _scope = scope;
            _overrideTenantId = overrideTenantId;
        }

        public DbSet<TenantEntity> Entities => Set<TenantEntity>();

        protected override void OnConfiguring(DbContextOptionsBuilder optionsBuilder)
        {
            optionsBuilder.UseInMemoryDatabase(_databaseName);
        }

        protected override void OnModelCreating(ModelBuilder modelBuilder)
        {
            modelBuilder.ApplyDefaultTenantFilters(_scope, GlobalFilterBehavior.IgnoreGlobalScope, _overrideTenantId);
            modelBuilder.Entity<TenantEntity>().HasKey(e => e.Id);
        }
    }
}
