using Platform.Core.Time;

namespace Platform.Testing.Time;

/// <summary>
/// Deterministic, test-controllable <see cref="IClock"/>
/// implementation. Tests advance the returned time with
/// <see cref="Advance"/> or replace it with <see cref="Set"/>; the
/// initial value is supplied at construction. The clock never reads
/// the system clock.
/// </summary>
public sealed class ControllableClock : IClock
{
    private DateTimeOffset _value;
    private readonly object _gate = new();

    /// <summary>
    /// Initializes a new instance of the
    /// <see cref="ControllableClock"/> class with the supplied
    /// initial UTC time.
    /// </summary>
    /// <param name="initial">The initial UTC time.</param>
    public ControllableClock(DateTimeOffset initial)
    {
        if (initial.Offset != TimeSpan.Zero)
        {
            throw new ArgumentException("Initial clock value must be UTC.", nameof(initial));
        }
        _value = initial;
    }

    /// <summary>
    /// Initializes a new instance of the
    /// <see cref="ControllableClock"/> class anchored at the Unix
    /// epoch.
    /// </summary>
    public ControllableClock()
        : this(DateTimeOffset.UnixEpoch)
    {
    }

    /// <inheritdoc />
    public DateTimeOffset UtcNow
    {
        get
        {
            lock (_gate)
            {
                return _value;
            }
        }
    }

    /// <summary>
    /// Replaces the current time with the supplied value.
    /// </summary>
    /// <param name="value">The new UTC time.</param>
    /// <exception cref="ArgumentException"><paramref name="value"/> is not UTC.</exception>
    public void Set(DateTimeOffset value)
    {
        if (value.Offset != TimeSpan.Zero)
        {
            throw new ArgumentException("Clock value must be UTC.", nameof(value));
        }
        lock (_gate)
        {
            _value = value;
        }
    }

    /// <summary>
    /// Advances the current time by the supplied <paramref name="amount"/>.
    /// </summary>
    /// <param name="amount">The amount of time to add; may be negative.</param>
    public void Advance(TimeSpan amount)
    {
        lock (_gate)
        {
            _value = _value.Add(amount);
        }
    }
}
