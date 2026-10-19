using Platform.Core.Time;

namespace Platform.Core.Tests.Time;

public class SystemClockTests
{
    [Fact]
    public void UtcNow_returns_UTC_offset()
    {
        IClock clock = new SystemClock();

        var value = clock.UtcNow;

        Assert.Equal(TimeSpan.Zero, value.Offset);
    }

    [Fact]
    public void UtcNow_is_close_to_system_clock()
    {
        IClock clock = new SystemClock();
        var before = DateTimeOffset.UtcNow;

        var actual = clock.UtcNow;

        var after = DateTimeOffset.UtcNow;
        Assert.InRange(actual, before.AddSeconds(-1), after.AddSeconds(1));
    }
}
