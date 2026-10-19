using Microsoft.Data.Sqlite;
using Microsoft.EntityFrameworkCore;
using Microsoft.EntityFrameworkCore.Diagnostics;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging;
using Platform.Auditing.Contracts;
using Platform.Auditing.Contracts.Common;
using Platform.Auditing.Contracts.DependencyInjection;
using Platform.Auditing.EfCore.DependencyInjection;
using Platform.Core.Time;

namespace Platform.Auditing.Tests.EfCore;

public class AuditingSaveChangesInterceptorTests : IDisposable
{
    private readonly SqliteConnection _connection;

    public AuditingSaveChangesInterceptorTests()
    {
        _connection = new SqliteConnection("DataSource=:memory:");
        _connection.Open();
    }

    public void Dispose() => _connection.Dispose();

    private sealed class Fixture : IDisposable
    {
        private readonly ServiceProvider _provider;
        private readonly IServiceScope _scope;

        public TestDbContext Context { get; }

        public CapturingAuditSink Sink { get; }

        public Fixture(SqliteConnection connection, Action<AuditOptions>? configure = null, IAuditSink? sink = null)
        {
            var services = new ServiceCollection();
            services.AddLogging();
            services.AddPlatformAuditing(o =>
            {
                o.PublishMode = AuditPublishMode.Synchronous;
                configure?.Invoke(o);
            });
            services.AddPlatformAuditingEfCore();
            services.AddSingleton<IClock>(new FixedClock(AuditTestDoubles.FixedUtc()));
            Sink = new CapturingAuditSink();
            services.AddSingleton<IAuditSink>(sink ?? Sink);
            services.AddDbContext<TestDbContext>((sp, o) => o
                .UseSqlite(connection)
                .AddInterceptors(sp.GetRequiredService<ISaveChangesInterceptor>()), ServiceLifetime.Scoped);
            _provider = services.BuildServiceProvider();
            _scope = _provider.CreateScope();
            Context = _scope.ServiceProvider.GetRequiredService<TestDbContext>();
            Context.Database.EnsureCreated();
        }

        public void Dispose()
        {
            _scope.Dispose();
            _provider.Dispose();
        }
    }

    private Fixture Build(Action<AuditOptions>? configure = null, IAuditSink? sink = null)
        => new Fixture(_connection, configure, sink);

    [Fact]
    public void Added_audited_entity_emits_created_event_with_masked_password()
    {
        using var fixture = Build();
        fixture.Context.Accounts.Add(new AuditedAccount { Name = "alice", Password = "s3cret" });
        fixture.Context.SaveChanges();

        var recorded = Assert.Single(fixture.Sink.Recorded);
        Assert.Equal("entity.created", recorded.Action);
        Assert.Equal("entity", recorded.Category);
        Assert.Equal("AuditedAccount", recorded.Metadata["entity.type"]);
        Assert.Equal("created", recorded.Metadata["entity.operation"]);
        Assert.Equal(AuditTestDoubles.FixedUtc(), recorded.OccurredAt);
        Assert.Equal(DefaultAuditMasker.Redacted, recorded.Metadata["entity.change.Password"]);
        Assert.Equal("set:alice", recorded.Metadata["entity.change.Name"]);
    }

    [Fact]
    public void Modified_audited_entity_emits_updated_event_with_diffs()
    {
        using var fixture = Build();
        var account = new AuditedAccount { Name = "bob", Password = "hunter2" };
        fixture.Context.Accounts.Add(account);
        fixture.Context.SaveChanges();

        account.Name = "bobby";
        fixture.Context.SaveChanges();

        var recorded = Assert.Single(fixture.Sink.Recorded.Skip(1));
        Assert.Equal("entity.updated", recorded.Action);
        Assert.Equal("bob->bobby", recorded.Metadata["entity.change.Name"]);
        Assert.False(recorded.Metadata.ContainsKey("entity.change.Password"));
    }

    [Fact]
    public void Deleted_audited_entity_emits_deleted_event()
    {
        using var fixture = Build();
        var account = new AuditedAccount { Name = "carol", Password = "pw" };
        fixture.Context.Accounts.Add(account);
        fixture.Context.SaveChanges();

        fixture.Context.Accounts.Remove(account);
        fixture.Context.SaveChanges();

        var recorded = Assert.Single(fixture.Sink.Recorded.Skip(1));
        Assert.Equal("entity.deleted", recorded.Action);
        Assert.Equal("removed:carol", recorded.Metadata["entity.change.Name"]);
    }

    [Fact]
    public void Non_audited_entity_is_not_captured()
    {
        using var fixture = Build();
        fixture.Context.Things.Add(new IgnoredThing { Label = "noise" });
        fixture.Context.SaveChanges();

        Assert.Empty(fixture.Sink.Recorded);
    }

    [Fact]
    public void Navigation_reference_is_not_treated_as_scalar_change()
    {
        using var fixture = Build();
        var account = new AuditedAccount { Name = "dave", Password = "pw", Owner = new AccountOwner { Title = "admin" } };
        fixture.Context.Accounts.Add(account);
        fixture.Context.SaveChanges();

        var recorded = Assert.Single(fixture.Sink.Recorded);
        Assert.False(recorded.Metadata.ContainsKey("entity.change.Owner"));
        Assert.Contains("entity.change.OwnerId", recorded.Metadata.Keys);
    }

    [Fact]
    public void Entity_capture_disabled_by_option_emits_nothing()
    {
        using var fixture = Build(o => o.EnableEntityCapture = false);
        fixture.Context.Accounts.Add(new AuditedAccount { Name = "erin", Password = "pw" });
        fixture.Context.SaveChanges();

        Assert.DoesNotContain(fixture.Sink.Recorded, e => e.Category == "entity");
    }

    [Fact]
    public void Failing_recorder_does_not_block_save()
    {
        using var fixture = Build(o => o.FailurePolicy = AuditFailurePolicy.FailClosed, sink: new ThrowingAuditSink());
        var account = new AuditedAccount { Name = "frank", Password = "pw" };
        fixture.Context.Accounts.Add(account);
        fixture.Context.SaveChanges();

        fixture.Context.Entry(account).State = EntityState.Detached;
        Assert.NotNull(fixture.Context.Accounts.SingleOrDefault(a => a.Name == "frank"));
    }

    private sealed class TestDbContext : DbContext
    {
        public TestDbContext(DbContextOptions<TestDbContext> options) : base(options) { }

        public DbSet<AuditedAccount> Accounts => Set<AuditedAccount>();

        public DbSet<IgnoredThing> Things => Set<IgnoredThing>();
    }

    private sealed class AuditedAccount : IAuditedEntity
    {
        public int Id { get; set; }

        public string Name { get; set; } = string.Empty;

        public string Password { get; set; } = string.Empty;

        public AccountOwner? Owner { get; set; }
    }

    private sealed class AccountOwner
    {
        public int Id { get; set; }

        public string Title { get; set; } = string.Empty;
    }

    private sealed class IgnoredThing
    {
        public int Id { get; set; }

        public string Label { get; set; } = string.Empty;
    }
}
