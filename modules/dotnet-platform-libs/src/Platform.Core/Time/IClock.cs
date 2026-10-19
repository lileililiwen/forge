namespace Platform.Core.Time;

/// <summary>
/// Provides the current UTC time without coupling consumers to a static
/// <see cref="DateTimeOffset"/> call. Implementations must return a
/// <see cref="DateTimeOffset"/> whose <see cref="DateTimeOffset.Offset"/>
/// is <c>TimeSpan.Zero</c>.
/// </summary>
public interface IClock
{
    /// <summary>
    /// Returns the current UTC time.
    /// </summary>
    DateTimeOffset UtcNow { get; }
}
