using Microsoft.Extensions.Options;
using Platform.Core.Time;

namespace Platform.Idempotency.Tests;

public class InMemoryIdempotencyStoreTests
{
    [Fact]
    public async Task TryGetAsync_returns_null_for_unknown_key()
    {
        var store = NewStore();

        var record = await store.TryGetAsync("unknown");

        Assert.Null(record);
    }

    [Fact]
    public async Task SaveAsync_then_TryGetAsync_round_trips()
    {
        var store = NewStore();
        var record = NewRecord("key-1", fingerprint: "fp-1");

        await store.SaveAsync(record);
        var fetched = await store.TryGetAsync("key-1");

        Assert.NotNull(fetched);
        Assert.Equal(record.Key, fetched!.Key);
        Assert.Equal(record.Fingerprint, fetched.Fingerprint);
        Assert.Equal(record.StatusCode, fetched.StatusCode);
        Assert.Equal(record.ResponseBody, fetched.ResponseBody);
    }

    [Fact]
    public async Task TryGetAsync_rejects_null_or_empty_key()
    {
        var store = NewStore();

        await Assert.ThrowsAsync<ArgumentException>(async () => await store.TryGetAsync(""));
        await Assert.ThrowsAsync<ArgumentException>(async () => await store.TryGetAsync("   "));
    }

    [Fact]
    public async Task SaveAsync_rejects_null_or_empty_key()
    {
        var store = NewStore();

        await Assert.ThrowsAsync<ArgumentException>(async () =>
            await store.SaveAsync(NewRecord("", fingerprint: "fp-1")));
        await Assert.ThrowsAsync<ArgumentException>(async () =>
            await store.SaveAsync(NewRecord("   ", fingerprint: "fp-1")));
    }

    [Fact]
    public async Task SaveAsync_rejects_keys_longer_than_MaxKeyLength()
    {
        var clock = new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        var options = Options(new IdempotencyOptions { MaxKeyLength = 16 });
        var store = new InMemoryIdempotencyStore(clock, options);

        var oversized = new string('a', 17);

        await Assert.ThrowsAsync<ArgumentException>(async () =>
            await store.SaveAsync(NewRecord(oversized, fingerprint: "fp-1")));
    }

    [Fact]
    public async Task SaveAsync_rejects_null_record()
    {
        var store = NewStore();

        await Assert.ThrowsAsync<ArgumentNullException>(async () => await store.SaveAsync(null!));
    }

    [Fact]
    public async Task EvictExpiredAsync_removes_records_older_than_retention_window()
    {
        var clock = new MutableClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        var options = Options(new IdempotencyOptions { RetentionSeconds = 86400 });
        var store = new InMemoryIdempotencyStore(clock, options);

        var t0 = clock.UtcNow;
        await store.SaveAsync(NewRecord("stale", fingerprint: "fp", createdAt: t0));

        clock.Advance(TimeSpan.FromSeconds(86400 / 2));
        var halfway = clock.UtcNow;
        await store.SaveAsync(NewRecord("fresh", fingerprint: "fp", createdAt: halfway));

        clock.Advance(TimeSpan.FromSeconds(86400 / 2 + 1));

        var removed = await store.EvictExpiredAsync();

        Assert.Equal(1, removed);
        Assert.Null(await store.TryGetAsync("stale"));
        Assert.NotNull(await store.TryGetAsync("fresh"));
    }

    [Fact]
    public async Task EvictExpiredAsync_returns_zero_when_nothing_is_expired()
    {
        var store = NewStore();
        await store.SaveAsync(NewRecord("fresh", fingerprint: "fp"));

        var removed = await store.EvictExpiredAsync();

        Assert.Equal(0, removed);
    }

    [Fact]
    public async Task TryGetAsync_treats_expired_record_as_miss()
    {
        var clock = new MutableClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        var options = Options(new IdempotencyOptions { RetentionSeconds = 60 });
        var store = new InMemoryIdempotencyStore(clock, options);

        await store.SaveAsync(NewRecord("stale", fingerprint: "fp", createdAt: clock.UtcNow));
        clock.Advance(TimeSpan.FromSeconds(61));

        var record = await store.TryGetAsync("stale");

        Assert.Null(record);
    }

    [Fact]
    public void Ctor_rejects_null_dependencies()
    {
        var options = Options(new IdempotencyOptions());
        var clock = new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));

        Assert.Throws<ArgumentNullException>(() => new InMemoryIdempotencyStore(null!, options));
        Assert.Throws<ArgumentNullException>(() => new InMemoryIdempotencyStore(clock, null!));
    }

    private static InMemoryIdempotencyStore NewStore()
    {
        var clock = new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        return new InMemoryIdempotencyStore(clock, Options(new IdempotencyOptions()));
    }

    private static IOptions<IdempotencyOptions> Options(IdempotencyOptions options) =>
        Microsoft.Extensions.Options.Options.Create(options);

    private static IdempotencyRecord NewRecord(
        string key,
        string fingerprint,
        DateTimeOffset? createdAt = null)
    {
        return new IdempotencyRecord(
            Key: key,
            Fingerprint: fingerprint,
            StatusCode: 200,
            ContentType: "application/json",
            ResponseBody: "{}",
            CreatedAt: createdAt ?? new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
    }
}
