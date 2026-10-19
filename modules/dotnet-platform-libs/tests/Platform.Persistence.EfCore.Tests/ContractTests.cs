using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Options;
using Platform.Core.Audit;
using Platform.Core.Context;
using Platform.Core.Time;
using Platform.Persistence.EfCore.Audit;
using Platform.Persistence.EfCore.Deletion;
using Platform.Persistence.EfCore.DependencyInjection;
using Platform.Persistence.EfCore.Interceptors;
using Platform.Persistence.EfCore.Migrations;
using Platform.Persistence.EfCore.Paging;
using Platform.Persistence.EfCore.Specifications;
using Platform.Persistence.EfCore.Tenancy;

namespace Platform.Persistence.EfCore.Tests;

public sealed class ContractTests
{
    [Fact]
    public void Registration_defaults_to_explicitly_disabled_conventions()
    {
        var services = new ServiceCollection();

        services.AddPlatformPersistenceEfCore();

        using var provider = services.BuildServiceProvider();
        var options = provider.GetRequiredService<IOptions<PersistenceOptions>>().Value;
        Assert.False(options.EnableAuditInterception);
        Assert.False(options.EnableSoftDeleteInterception);
        Assert.False(options.EnableTenantQueryFilters);
    }

    [Fact]
    public void Page_query_applies_ordering_and_bounded_skip_take()
    {
        var result = new[] { 5, 2, 4, 1, 3 }.AsQueryable()
            .Page(new PageRequest(2, 2, PageSort.Descending));

        Assert.Equal(new[] { 3, 2 }, result.Items);
        Assert.Equal(5, result.TotalCount);
        Assert.Equal(2, result.PageNumber);
        Assert.Equal(2, result.PageSize);
    }

    [Fact]
    public void Specification_composes_product_predicate_without_replacing_it()
    {
        var specification = new PredicateSpecification<TestEntity>(entity => entity.Value >= 2);

        var result = new[]
        {
            new TestEntity(1), new TestEntity(2), new TestEntity(3),
        }.AsQueryable().Where(specification.Criteria).ToArray();

        Assert.Equal(new[] { 2, 3 }, result.Select(entity => entity.Value));
    }

    [Fact]
    public async Task Save_changes_interceptor_audits_and_soft_deletes_without_physical_delete()
    {
        var clock = new FixedClock(new DateTimeOffset(2026, 1, 1, 12, 0, 0, TimeSpan.Zero));
        var actor = new StaticActorAccessor("user-1");
        var options = new DbContextOptionsBuilder<TestDbContext>()
            .UseInMemoryDatabase(Guid.NewGuid().ToString())
            .AddInterceptors(new PlatformSaveChangesInterceptor(clock, actor))
            .Options;

        await using var db = new TestDbContext(options);
        var entity = new TestEntity(4);
        db.Entities.Add(entity);
        await db.SaveChangesAsync();
        db.Entities.Remove(entity);
        await db.SaveChangesAsync();

        Assert.Equal(clock.UtcNow, entity.CreatedAt);
        Assert.Equal("user-1", entity.CreatedBy);
        Assert.True(entity.IsDeleted);
        Assert.Equal("user-1", entity.DeletedBy);
        Assert.Single(await db.Entities.IgnoreQueryFilters().ToListAsync());
    }

    [Fact]
    public async Task Migration_status_is_read_only_and_ready_when_no_migrations_are_pending()
    {
        var options = new DbContextOptionsBuilder<TestDbContext>()
            .UseInMemoryDatabase(Guid.NewGuid().ToString())
            .Options;

        await using var db = new TestDbContext(options);
        await db.Database.EnsureCreatedAsync();
        var status = await new EfCoreMigrationStatusReader(db).GetStatusAsync();

        Assert.True(status.IsAvailable);
        Assert.True(status.IsReady);
        Assert.Equal(0, status.PendingMigrationCount);
    }

    [Fact]
    public async Task Readiness_check_reports_unhealthy_without_applying_migrations()
    {
        var check = new EfCoreReadinessCheck(new FixedMigrationStatusReader(
            new MigrationStatus(true, false, 2)));

        var result = await check.CheckHealthAsync(new Microsoft.Extensions.Diagnostics.HealthChecks.HealthCheckContext());

        Assert.Equal(Microsoft.Extensions.Diagnostics.HealthChecks.HealthStatus.Unhealthy, result.Status);
    }

    [Fact]
    public async Task Tenant_filter_is_applied_only_to_the_explicit_entity_type()
    {
        var scope = new StaticTenantScope("tenant-a");
        var options = new DbContextOptionsBuilder<TenantDbContext>()
            .UseInMemoryDatabase(Guid.NewGuid().ToString())
            .Options;

        await using (var db = new TenantDbContext(options, scope))
        {
            db.Entities.AddRange(new TenantEntity { TenantId = "tenant-a" }, new TenantEntity { TenantId = "tenant-b" });
            await db.SaveChangesAsync();
        }

        await using var scopedDb = new TenantDbContext(options, scope);
        Assert.Single(await scopedDb.Entities.ToListAsync());
        Assert.Equal("tenant-a", (await scopedDb.Entities.SingleAsync()).TenantId);
    }

    [Fact]
    public async Task Relational_sqlite_status_is_read_without_running_migrations()
    {
        await using var connection = new Microsoft.Data.Sqlite.SqliteConnection("Data Source=:memory:");
        await connection.OpenAsync();
        var options = new DbContextOptionsBuilder<TestDbContext>().UseSqlite(connection).Options;

        await using var db = new TestDbContext(options);
        await db.Database.EnsureCreatedAsync();
        var status = await new EfCoreMigrationStatusReader(db).GetStatusAsync();

        Assert.True(status.IsAvailable);
        Assert.True(status.IsReady);
        Assert.Equal(0, status.PendingMigrationCount);
    }

    [Fact]
    public async Task Independent_contexts_can_save_concurrently_to_the_same_store()
    {
        var databaseName = Guid.NewGuid().ToString();
        var tasks = Enumerable.Range(1, 8).Select(async id =>
        {
            var options = new DbContextOptionsBuilder<TestDbContext>()
                .UseInMemoryDatabase(databaseName)
                .Options;
            await using var db = new TestDbContext(options);
            db.Entities.Add(new TestEntity(id));
            await db.SaveChangesAsync();
        });

        await Task.WhenAll(tasks);

        var readOptions = new DbContextOptionsBuilder<TestDbContext>().UseInMemoryDatabase(databaseName).Options;
        await using var reader = new TestDbContext(readOptions);
        Assert.Equal(8, await reader.Entities.CountAsync());
    }

    private sealed record TestEntity(int Value) : IAuditableEntity, ISoftDeletable
    {
        public int Id { get; init; }
        public DateTimeOffset CreatedAt { get; set; }
        public string? CreatedBy { get; set; }
        public DateTimeOffset? UpdatedAt { get; set; }
        public string? UpdatedBy { get; set; }
        public bool IsDeleted { get; set; }
        public DateTimeOffset? DeletedAt { get; set; }
        public string? DeletedBy { get; set; }
    }

    private sealed class TestDbContext(DbContextOptions<TestDbContext> options) : DbContext(options)
    {
        public DbSet<TestEntity> Entities => Set<TestEntity>();
    }

    private sealed class FixedMigrationStatusReader(MigrationStatus status) : IMigrationStatusReader
    {
        public Task<MigrationStatus> GetStatusAsync(CancellationToken cancellationToken = default) => Task.FromResult(status);
    }

    private sealed class StaticTenantScope(string? tenantId) : ITenantScope
    {
        public string? TenantId { get; } = tenantId;
    }

    private sealed class TenantEntity : ITenantScoped
    {
        public int Id { get; set; }
        public string? TenantId { get; set; }
    }

    private sealed class TenantDbContext(DbContextOptions<TenantDbContext> options, ITenantScope scope) : DbContext(options)
    {
        public DbSet<TenantEntity> Entities => Set<TenantEntity>();

        protected override void OnModelCreating(ModelBuilder modelBuilder) =>
            modelBuilder.ApplyTenantFilter<TenantEntity>(scope);
    }

    private sealed class StaticActorAccessor(string? subjectId) : IActorAccessor
    {
        public string? SubjectId { get; } = subjectId;
    }
}
