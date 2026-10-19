using Microsoft.Extensions.DependencyInjection;
using Platform.ConsumerConformance.Fixtures;
using Platform.Idempotency;
using Platform.Idempotency.DependencyInjection;
using Platform.RateLimiting;
using Platform.RateLimiting.DependencyInjection;

namespace Platform.ConsumerConformance.Tests;

public sealed class CancellationTests
{
    [Fact]
    public async Task RateLimiter_respects_a_cancelled_token()
    {
        var services = ConsumerServiceCollectionFactory.CreateServices();
        services.AddPlatformRateLimiting();
        await using var provider = ConsumerServiceCollectionFactory.BuildProvider(services);
        var limiter = provider.GetRequiredService<IRateLimiter>();
        using var cts = new CancellationTokenSource();
        cts.Cancel();
        await Assert.ThrowsAsync<OperationCanceledException>(() => limiter.CheckAsync(new RateLimitKey("feed", "subject-a"), cts.Token));
    }

    [Fact]
    public async Task Consumer_replacement_idempotency_store_can_observe_cancellation()
    {
        var services = ConsumerServiceCollectionFactory.CreateServices();
        var observed = new TaskCompletionSource<bool>(TaskCreationOptions.RunContinuationsAsynchronously);
        services.AddSingleton<IIdempotencyStore>(new CancellationObservingIdempotencyStore(observed));
        await using var provider = ConsumerServiceCollectionFactory.BuildProvider(services);
        var store = provider.GetRequiredService<IIdempotencyStore>();
        using var cts = new CancellationTokenSource();
        cts.Cancel();
        try
        {
            await store.TryGetAsync("key-1", cts.Token);
        }
        catch (OperationCanceledException)
        {
            // expected; the test asserts that the registered store observed the cancellation.
        }
        Assert.True(await observed.Task);
    }

    private sealed class CancellationObservingIdempotencyStore : IIdempotencyStore
    {
        private readonly TaskCompletionSource<bool> _observed;

        public CancellationObservingIdempotencyStore(TaskCompletionSource<bool> observed) => _observed = observed;

        public Task<IdempotencyRecord?> TryGetAsync(string key, CancellationToken cancellationToken = default)
        {
            _observed.TrySetResult(cancellationToken.IsCancellationRequested);
            cancellationToken.ThrowIfCancellationRequested();
            return Task.FromResult<IdempotencyRecord?>(null);
        }

        public Task SaveAsync(IdempotencyRecord record, CancellationToken cancellationToken = default) => Task.CompletedTask;

        public Task<int> EvictExpiredAsync(CancellationToken cancellationToken = default) => Task.FromResult(0);
    }
}
