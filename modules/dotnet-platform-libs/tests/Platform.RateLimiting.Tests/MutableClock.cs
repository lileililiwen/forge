using Platform.Core.Time;

namespace Platform.RateLimiting.Tests;

internal sealed class MutableClock : IClock
{
    private DateTimeOffset _value;
    public MutableClock() : this(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero)) { }
    public MutableClock(DateTimeOffset value) { _value = value; }
    public DateTimeOffset UtcNow => _value;
    public void Advance(TimeSpan delta) => _value += delta;
    public void Set(DateTimeOffset value) => _value = value;
}
