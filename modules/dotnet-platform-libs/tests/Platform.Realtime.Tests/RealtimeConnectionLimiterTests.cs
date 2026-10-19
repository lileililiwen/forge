using Microsoft.Extensions.Options;
using Platform.Realtime;
using Platform.Realtime.AspNetCore.Sse;

namespace Platform.Realtime.Tests;

public class RealtimeConnectionLimiterTests
{
    private static RealtimeConnectionLimiter Create(int max)
    {
        var options = Options.Create(new RealtimeConnectionOptions { MaxConcurrentConnections = max });
        return new RealtimeConnectionLimiter(options);
    }

    [Fact]
    public void Acquires_up_to_max_then_rejects()
    {
        var limiter = Create(2);

        Assert.True(limiter.TryAcquire());
        Assert.True(limiter.TryAcquire());
        Assert.Equal(2, limiter.ActiveCount);
        Assert.False(limiter.TryAcquire());
        Assert.Equal(2, limiter.ActiveCount);
    }

    [Fact]
    public void Release_restores_capacity()
    {
        var limiter = Create(1);
        Assert.True(limiter.TryAcquire());
        Assert.False(limiter.TryAcquire());

        limiter.Release();

        Assert.True(limiter.TryAcquire());
        Assert.Equal(1, limiter.ActiveCount);
    }

    [Fact]
    public void Release_never_goes_negative()
    {
        var limiter = Create(1);
        limiter.Release();
        Assert.Equal(0, limiter.ActiveCount);
    }
}
