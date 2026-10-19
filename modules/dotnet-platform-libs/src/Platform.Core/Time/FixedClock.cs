namespace Platform.Core.Time;

/// <summary>
/// Deterministic <see cref="IClock"/> implementation for tests. The
/// returned time is supplied at construction and can be advanced through
/// the constructor or replaced with a delegate.
/// </summary>
public sealed class FixedClock : IClock
{
    private readonly Func<DateTimeOffset> _provider;

    /// <summary>
    /// Initializes a new instance that always returns the supplied
    /// <paramref name="value"/>.
    /// </summary>
    /// <param name="value">The fixed UTC time returned by this clock.</param>
    public FixedClock(DateTimeOffset value)
        : this(() => value)
    {
    }

    /// <summary>
    /// Initializes a new instance that delegates to the supplied
    /// <paramref name="provider"/>. Use this overload when a test must
    /// observe advancing time.
    /// </summary>
    /// <param name="provider">The delegate invoked for each <see cref="UtcNow"/> access.</param>
    /// <exception cref="ArgumentNullException"><paramref name="provider"/> is <c>null</c>.</exception>
    public FixedClock(Func<DateTimeOffset> provider)
    {
        ArgumentNullException.ThrowIfNull(provider);
        _provider = provider;
    }

    /// <inheritdoc />
    public DateTimeOffset UtcNow => _provider();
}
