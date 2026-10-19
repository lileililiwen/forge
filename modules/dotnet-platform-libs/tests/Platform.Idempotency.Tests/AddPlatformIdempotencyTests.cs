using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Options;
using Platform.Core.Time;
using Platform.Idempotency.DependencyInjection;

namespace Platform.Idempotency.Tests;

public class AddPlatformIdempotencyTests
{
    [Fact]
    public void Default_registration_binds_options_and_registers_in_memory_store()
    {
        var services = new ServiceCollection();

        services.AddPlatformIdempotency();

        using var provider = services.BuildServiceProvider();
        var options = provider.GetRequiredService<IOptions<IdempotencyOptions>>().Value;
        var store = provider.GetRequiredService<IIdempotencyStore>();

        Assert.True(options.Enabled);
        Assert.Equal("memory", options.Storage);
        Assert.IsType<InMemoryIdempotencyStore>(store);
    }

    [Fact]
    public void Configure_delegate_overrides_defaults()
    {
        var services = new ServiceCollection();

        services.AddPlatformIdempotency(options =>
        {
            options.RetentionSeconds = 120;
            options.MaxKeyLength = 64;
        });

        using var provider = services.BuildServiceProvider();
        var options = provider.GetRequiredService<IOptions<IdempotencyOptions>>().Value;

        Assert.Equal(120, options.RetentionSeconds);
        Assert.Equal(64, options.MaxKeyLength);
    }

    [Fact]
    public async Task Disabled_registration_uses_no_op_store()
    {
        var services = new ServiceCollection();

        services.AddPlatformIdempotency(options => options.Enabled = false);

        using var provider = services.BuildServiceProvider();
        var store = provider.GetRequiredService<IIdempotencyStore>();
        var record = NewRecord("key-1");

        await store.SaveAsync(record);
        var fetched = await store.TryGetAsync("key-1");

        Assert.Null(fetched);
    }

    [Fact]
    public void Registration_preserves_existing_clock()
    {
        var services = new ServiceCollection();
        var preExisting = new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        services.AddSingleton<IClock>(preExisting);

        services.AddPlatformIdempotency();

        using var provider = services.BuildServiceProvider();
        Assert.Same(preExisting, provider.GetRequiredService<IClock>());
    }

    [Fact]
    public void Null_services_throws()
    {
        Assert.Throws<ArgumentNullException>(() => ((IServiceCollection)null!).AddPlatformIdempotency());
        Assert.Throws<ArgumentNullException>(
            () => ((IServiceCollection)null!).AddPlatformIdempotency(_ => { }));
        Assert.Throws<ArgumentNullException>(
            () => new ServiceCollection().AddPlatformIdempotency(configure: null!));
    }

    [Fact]
    public void AddPlatformIdempotency_returns_same_service_collection()
    {
        var services = new ServiceCollection();

        var result = services.AddPlatformIdempotency();

        Assert.Same(services, result);
    }

    private static IdempotencyRecord NewRecord(string key) => new(
        Key: key,
        Fingerprint: "fp-1",
        StatusCode: 200,
        ContentType: "application/json",
        ResponseBody: "{}",
        CreatedAt: new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
}
