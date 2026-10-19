namespace Platform.Jobs;

/// <summary>
/// Documented payload shape accepted by <see cref="IJobDispatcher"/>.
/// Consumers attach any engine-specific arguments through
/// <see cref="Arguments"/>; the platform never inspects the dictionary.
/// </summary>
/// <param name="Name">The job name. MUST be non-null and non-empty.</param>
/// <param name="Arguments">Optional engine-specific arguments. May be <c>null</c>.</param>
public sealed record JobPayload(
    string Name,
    IReadOnlyDictionary<string, object?>? Arguments = null)
{
    /// <summary>
    /// Creates a payload from the supplied name and optional argument
    /// pairs.
    /// </summary>
    /// <param name="name">The job name.</param>
    /// <param name="arguments">Optional key/value pairs to attach.</param>
    /// <returns>The constructed <see cref="JobPayload"/>.</returns>
    /// <exception cref="ArgumentException"><paramref name="name"/> is null, empty, or whitespace.</exception>
    public static JobPayload Create(
        string name,
        IReadOnlyDictionary<string, object?>? arguments = null)
    {
        if (string.IsNullOrWhiteSpace(name))
        {
            throw new ArgumentException("Job name must be a non-empty string.", nameof(name));
        }

        return new JobPayload(name, arguments);
    }
}
