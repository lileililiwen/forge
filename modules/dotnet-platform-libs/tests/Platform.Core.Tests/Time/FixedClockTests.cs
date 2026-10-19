using Platform.Core.Time;

namespace Platform.Core.Tests.Time;

public class FixedClockTests
{
    [Fact]
    public void UtcNow_returns_supplied_value()
    {
        var supplied = new DateTimeOffset(2026, 1, 2, 3, 4, 5, TimeSpan.Zero);
        IClock clock = new FixedClock(supplied);

        Assert.Equal(supplied, clock.UtcNow);
        Assert.Equal(supplied, clock.UtcNow);
    }

    [Fact]
    public void UtcNow_delegates_to_supplied_provider()
    {
        var value = new DateTimeOffset(2030, 6, 1, 0, 0, 0, TimeSpan.Zero);
        var calls = 0;
        IClock clock = new FixedClock(() =>
        {
            calls++;
            return value;
        });

        _ = clock.UtcNow;
        _ = clock.UtcNow;
        _ = clock.UtcNow;

        Assert.Equal(3, calls);
    }

    [Fact]
    public void Provider_advances_time_between_calls()
    {
        var step = TimeSpan.FromMinutes(1);
        var start = new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero);
        var counter = 0;
        IClock clock = new FixedClock(() => start.AddMinutes(counter++ * step.TotalMinutes));

        var first = clock.UtcNow;
        var second = clock.UtcNow;
        var third = clock.UtcNow;

        Assert.Equal(start, first);
        Assert.Equal(start.Add(step), second);
        Assert.Equal(start.Add(step + step), third);
    }

    [Fact]
    public void Constructor_rejects_null_provider()
    {
        Assert.Throws<ArgumentNullException>(() => new FixedClock(provider: null!));
    }
}
