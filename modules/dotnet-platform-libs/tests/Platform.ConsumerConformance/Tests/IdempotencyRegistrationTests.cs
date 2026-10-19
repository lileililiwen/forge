using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Options;
using Platform.ConsumerConformance.Fixtures;
using Platform.Core.Time;
using Platform.Idempotency;
using Platform.Idempotency.DependencyInjection;

namespace Platform.ConsumerConformance.Tests;

public sealed class IdempotencyRegistrationTests
{
    [Fact]
    public void AddPlatformIdempotency_registers_in_memory_store_and_options()
    {
        var services = ConsumerServiceCollectionFactory.CreateServices();
        services.AddPlatformIdempotency();

        using var provider = ConsumerServiceCollectionFactory.BuildProvider(services);
        var options = provider.GetRequiredService<IOptions<IdempotencyOptions>>().Value;
        Assert.Equal("Idempotency", IdempotencyOptions.SectionName);
        Assert.Equal("idempotency.hit", IdempotencyOptions.HitMetric);
        var store = provider.GetRequiredService<IIdempotencyStore>();
        Assert.IsType<InMemoryIdempotencyStore>(store);
    }

    [Fact]
    public void AddPlatformIdempotency_with_disabled_flag_registers_no_op_store()
    {
        var services = ConsumerServiceCollectionFactory.CreateServices();
        services.AddPlatformIdempotency(options => options.Enabled = false);

        using var provider = ConsumerServiceCollectionFactory.BuildProvider(services);
        var store = provider.GetRequiredService<IIdempotencyStore>();
        var record = new IdempotencyRecord("key-1", "fingerprint-1", 200, "application/json", null, new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        Assert.Null(store.TryGetAsync("key-1", CancellationToken.None).GetAwaiter().GetResult());
    }

    [Fact]
    public void AddPlatformIdempotency_honours_consumer_clock()
    {
        var services = ConsumerServiceCollectionFactory.CreateServices();
        services.AddSingleton<IClock>(new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero)));
        services.AddPlatformIdempotency();

        using var provider = ConsumerServiceCollectionFactory.BuildProvider(services);
        var clock = provider.GetRequiredService<IClock>();
        Assert.IsType<FixedClock>(clock);
    }
}
