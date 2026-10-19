using Microsoft.EntityFrameworkCore;
using Platform.Core.Time;
using Platform.Webhooks.Contracts.Common;
using Platform.Webhooks.Contracts.Inbound;
using Platform.Webhooks.Contracts.Outbound;
using Platform.Webhooks.EfCore.Inbound;
using Platform.Webhooks.EfCore.Outbound;

namespace Platform.Webhooks.Tests.EfCore;

public sealed class EfCoreWebhookStoresTests
{
    [Fact]
    public async Task Inbox_store_records_completed_message()
    {
        await using var context = await TestContext.CreateAsync();
        var store = new EfCoreWebhookInboxStore(context.DbContext, context.Clock);
        var provider = new WebhookProviderId("test");
        var message = WebhookInboxMessage.Create(provider, "evt_1", context.Clock.UtcNow);

        var claim = await store.TryClaimAsync(message, context.Clock, "worker-1", TimeSpan.FromMinutes(1));
        Assert.Equal(WebhookInboxClaimStatus.Claimed, claim.Status);
        await store.MarkCompletedAsync(claim.Message!.ReplayKey, "worker-1");

        var stored = await store.GetAsync(claim.Message.ReplayKey);
        Assert.NotNull(stored);
        Assert.Equal(WebhookInboxState.Completed, stored!.State);
    }

    [Fact]
    public async Task Inbox_store_duplicate_is_detected()
    {
        await using var context = await TestContext.CreateAsync();
        var store = new EfCoreWebhookInboxStore(context.DbContext, context.Clock);
        var message = WebhookInboxMessage.Create(new WebhookProviderId("test"), "evt_dup", context.Clock.UtcNow);
        var first = await store.TryClaimAsync(message, context.Clock, "worker-1", TimeSpan.FromMinutes(1));
        await store.MarkCompletedAsync(first.Message!.ReplayKey, "worker-1");

        var retry = await store.TryClaimAsync(message, context.Clock, "worker-2", TimeSpan.FromMinutes(1));
        Assert.Equal(WebhookInboxClaimStatus.Duplicate, retry.Status);
    }

    [Fact]
    public async Task Delivery_store_persists_attempts()
    {
        await using var context = await TestContext.CreateAsync();
        var store = new EfCoreWebhookDeliveryStore(context.DbContext);
        var delivery = WebhookDelivery.Create(new WebhookDeliveryId("d_1"), new WebhookSubscriptionId("sub_1"), "order.created", "evt_1", context.Clock.UtcNow);

        await store.CreateAsync(delivery);
        var updated = delivery.WithPersistedState(WebhookDeliveryStatus.Succeeded, 1, null, null, 200);
        await store.UpdateAsync(updated);

        var stored = await store.GetAsync(new WebhookDeliveryId("d_1"));
        Assert.Equal(WebhookDeliveryStatus.Succeeded, stored!.Status);
        Assert.Equal(200, stored.LastResponseStatus);
    }

    private sealed class TestContext : IAsyncDisposable
    {
        public TestContext(DbContext context, IClock clock) { DbContext = context; Clock = clock; }
        public DbContext DbContext { get; }
        public IClock Clock { get; }
        public static async Task<TestContext> CreateAsync()
        {
            var connection = new Microsoft.Data.Sqlite.SqliteConnection("Filename=:memory:");
            await connection.OpenAsync();
            var options = new DbContextOptionsBuilder<TestDbContext>().UseSqlite(connection).Options;
            var context = new TestDbContext(options);
            await context.Database.EnsureCreatedAsync();
            return new TestContext(context, new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero)));
        }
        public async ValueTask DisposeAsync()
        {
            await DbContext.DisposeAsync();
        }
    }

    private sealed class TestDbContext : DbContext
    {
        public TestDbContext(DbContextOptions<TestDbContext> options) : base(options) { }
        public DbSet<EfWebhookInboxEntry> Inbox => Set<EfWebhookInboxEntry>();
        public DbSet<EfWebhookDeliveryEntry> Deliveries => Set<EfWebhookDeliveryEntry>();
        protected override void OnModelCreating(ModelBuilder modelBuilder)
        {
            modelBuilder.ApplyConfiguration(new WebhookInboxEntityConfiguration());
            modelBuilder.ApplyConfiguration(new WebhookDeliveryEntityConfiguration());
        }
    }
}
