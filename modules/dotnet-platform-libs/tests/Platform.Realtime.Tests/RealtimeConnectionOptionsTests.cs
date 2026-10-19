using Platform.Realtime;

namespace Platform.Realtime.Tests;

public class RealtimeConnectionOptionsTests
{
    [Fact]
    public void Defaults_are_safe_and_within_bounds()
    {
        var options = new RealtimeConnectionOptions();

        Assert.Equal(1000, options.MaxConcurrentConnections);
        Assert.Equal(64 * 1024, options.MaxPayloadBytes);
        Assert.Equal(TimeSpan.FromMinutes(5), options.ConnectionIdleTimeout);
        Assert.Equal(TimeSpan.FromSeconds(30), options.HeartbeatInterval);
        Assert.False(options.AllowCrossTenantBroadcast);
        Assert.Empty(options.Validate());
    }

    [Theory]
    [InlineData(0)]
    [InlineData(-1)]
    [InlineData(100_001)]
    public void Validate_rejects_out_of_range_connection_limit(int value)
    {
        var options = new RealtimeConnectionOptions { MaxConcurrentConnections = value };
        Assert.NotEmpty(options.Validate());
    }

    [Fact]
    public void Validate_rejects_non_positive_intervals()
    {
        var options = new RealtimeConnectionOptions
        {
            ConnectionIdleTimeout = TimeSpan.Zero,
            HeartbeatInterval = TimeSpan.Zero,
        };
        Assert.NotEmpty(options.Validate());
    }
}
