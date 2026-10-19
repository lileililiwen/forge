using Microsoft.Data.Sqlite;
using Microsoft.EntityFrameworkCore;
using Platform.Core.Time;
using Platform.Eventing.Contracts;
using Platform.Eventing.EfCore;

namespace Platform.Eventing.EfCore.Tests;

public sealed class EfCoreDurableEventStoreTests
{
    private static readonly DateTimeOffset Now = new(2026, 9, 8, 0, 0, 0, TimeSpan.Zero);

    [Fact]
    public async Task Model_mapping_creates_application_configured_tables()
    {
        await using var fixture = await SqliteFixture.CreateAsync();
        await using var db = fixture.CreateContext();

        var outbox = db.Model.FindEntityType(typeof(PlatformOutboxMessage))!;
        var inbox = db.Model.FindEntityType(typeof(PlatformInboxMessage))!;

        Assert.Equal("app_outbox", outbox.GetTableName());
        Assert.Equal("app_inbox", inbox.GetTableName());
    }

    [Fact]
    public async Task Outbox_store_round_trips_metadata_and_claim_state()
    {
        await using var fixture = await SqliteFixture.CreateAsync();
        await using var db = fixture.CreateContext();
        var store = new EfCoreOutboxStore(db, new FixedClock(Now));
        var message = OutboxMessage.Create(new DurableEventEnvelope("message-1", "Orders.OrderPlaced", "{}", Now, "tenant-1", "corr-1"), Now);

        await store.AddAsync(message);
        var claimed = Assert.Single(await store.ClaimAsync(Now, "worker-1", TimeSpan.FromMinutes(5), 10));

        Assert.Equal("tenant-1", claimed.Envelope.TenantId);
        Assert.Equal("corr-1", claimed.Envelope.CorrelationId);
        Assert.Equal(DurableMessageState.Leased, claimed.State);
    }

    [Fact]
    public async Task Inbox_store_suppresses_completed_duplicate()
    {
        await using var fixture = await SqliteFixture.CreateAsync();
        await using var db = fixture.CreateContext();
        var store = new EfCoreInboxStore(db, new FixedClock(Now));
        var message = InboxMessage.Create(new DurableEventEnvelope("message-1", "Orders.OrderPlaced", "{}", Now), Now);

        Assert.Equal(InboxClaimDecision.Claimed, (await store.TryClaimAsync(message, Now, "worker-1", TimeSpan.FromMinutes(5))).Decision);
        await store.MarkSucceededAsync(message.MessageId, "worker-1");

        Assert.Equal(InboxClaimDecision.Duplicate, (await store.TryClaimAsync(message, Now, "worker-2", TimeSpan.FromMinutes(5))).Decision);
    }

    [Fact]
    public async Task Outbox_claim_is_visible_across_independent_context_instances()
    {
        await using var fixture = await SqliteFixture.CreateAsync();
        await using var writer = fixture.CreateContext();
        await using var reader = fixture.CreateContext();
        var message = OutboxMessage.Create(new DurableEventEnvelope("message-1", "Orders.OrderPlaced", "{}", Now), Now);

        await new EfCoreOutboxStore(writer, new FixedClock(Now)).AddAsync(message);

        var claimed = await new EfCoreOutboxStore(reader, new FixedClock(Now))
            .ClaimAsync(Now, "worker-2", TimeSpan.FromMinutes(5), 10);

        Assert.Equal("message-1", Assert.Single(claimed).MessageId);
    }

    private sealed class SqliteFixture : IAsyncDisposable
    {
        private readonly SqliteConnection _connection;
        private SqliteFixture(SqliteConnection connection) => _connection = connection;

        public static async Task<SqliteFixture> CreateAsync()
        {
            var connection = new SqliteConnection("Data Source=:memory:");
            await connection.OpenAsync();
            await using var db = new TestDbContext(connection);
            await db.Database.EnsureCreatedAsync();
            return new SqliteFixture(connection);
        }

        public TestDbContext CreateContext() => new(_connection);
        public ValueTask DisposeAsync() => _connection.DisposeAsync();
    }

    private sealed class TestDbContext(SqliteConnection connection) : DbContext
    {
        protected override void OnConfiguring(DbContextOptionsBuilder options) => options.UseSqlite(connection);
        protected override void OnModelCreating(ModelBuilder modelBuilder) => modelBuilder.ConfigurePlatformEventing(new PlatformEventingModelOptions
        {
            OutboxTableName = "app_outbox",
            InboxTableName = "app_inbox",
        });
    }
}
