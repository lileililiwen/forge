using Platform.Testing.Time;

namespace Platform.Testing.Tests.Time;

public class ControllableClockTests
{
    [Fact]
    public void UtcNow_returns_initial_value()
    {
        var initial = new DateTimeOffset(2026, 1, 1, 12, 0, 0, TimeSpan.Zero);
        var clock = new ControllableClock(initial);

        Assert.Equal(initial, clock.UtcNow);
    }

    [Fact]
    public void UtcNow_starts_at_unix_epoch_when_no_value_supplied()
    {
        var clock = new ControllableClock();

        Assert.Equal(DateTimeOffset.UnixEpoch, clock.UtcNow);
    }

    [Fact]
    public void Advance_moves_clock_forward()
    {
        var clock = new ControllableClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));

        clock.Advance(TimeSpan.FromHours(2));

        Assert.Equal(new DateTimeOffset(2026, 1, 1, 2, 0, 0, TimeSpan.Zero), clock.UtcNow);
    }

    [Fact]
    public void Advance_accepts_negative_durations()
    {
        var clock = new ControllableClock(new DateTimeOffset(2026, 1, 2, 0, 0, 0, TimeSpan.Zero));

        clock.Advance(TimeSpan.FromDays(-1));

        Assert.Equal(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero), clock.UtcNow);
    }

    [Fact]
    public void Set_replaces_current_value()
    {
        var clock = new ControllableClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        var next = new DateTimeOffset(2026, 6, 1, 0, 0, 0, TimeSpan.Zero);

        clock.Set(next);

        Assert.Equal(next, clock.UtcNow);
    }

    [Fact]
    public void Constructor_rejects_non_utc_initial_value()
    {
        var nonUtc = new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.FromHours(2));

        Assert.Throws<ArgumentException>(() => new ControllableClock(nonUtc));
    }

    [Fact]
    public void Set_rejects_non_utc_value()
    {
        var clock = new ControllableClock();

        Assert.Throws<ArgumentException>(() => clock.Set(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.FromHours(2))));
    }
}
