namespace Platform.Core.Time;

/// <summary>
/// Production <see cref="IClock"/> implementation that delegates to
/// <see cref="DateTimeOffset.UtcNow"/>. Always returns a UTC value.
/// </summary>
public sealed class SystemClock : IClock
{
    /// <inheritdoc />
    public DateTimeOffset UtcNow => DateTimeOffset.UtcNow;
}
