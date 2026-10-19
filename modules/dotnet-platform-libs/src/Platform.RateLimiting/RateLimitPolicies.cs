namespace Platform.RateLimiting;

/// <summary>
/// Documented default policy catalog. Consumers can override the
/// catalog by registering their own <see cref="RateLimitPolicies"/>
/// before <c>AddPlatformRateLimiting</c> is called.
/// </summary>
public sealed class RateLimitPolicies
{
    /// <summary>
    /// Initializes a new <see cref="RateLimitPolicies"/> with the
    /// supplied entries. Entries with null or whitespace names are
    /// ignored; duplicate names keep the first occurrence.
    /// </summary>
    /// <param name="entries">The catalog entries.</param>
    /// <exception cref="ArgumentNullException"><paramref name="entries"/> is <c>null</c>.</exception>
    public RateLimitPolicies(IEnumerable<RateLimitPolicyOptions> entries)
    {
        ArgumentNullException.ThrowIfNull(entries);
        var seen = new HashSet<string>(StringComparer.Ordinal);
        var list = new List<RateLimitPolicyOptions>();
        foreach (var entry in entries)
        {
            if (string.IsNullOrWhiteSpace(entry?.Name) || entry.Limit <= 0 || entry.WindowSeconds <= 0)
            {
                continue;
            }
            if (!seen.Add(entry.Name))
            {
                continue;
            }
            list.Add(entry);
        }
        Entries = list;
    }

    /// <summary>Gets the catalog entries.</summary>
    public IReadOnlyList<RateLimitPolicyOptions> Entries { get; }

    /// <summary>
    /// Returns the entry for the supplied <paramref name="name"/>,
    /// or <c>null</c> when the policy is not registered.
    /// </summary>
    public RateLimitPolicyOptions? Find(string name)
    {
        if (string.IsNullOrWhiteSpace(name))
        {
            return null;
        }
        foreach (var entry in Entries)
        {
            if (string.Equals(entry.Name, name, StringComparison.Ordinal))
            {
                return entry;
            }
        }
        return null;
    }

    /// <summary>
    /// Returns the documented default catalog
    /// (<c>feed</c>, <c>search</c>, <c>uploads</c>,
    /// <c>downloads</c>, <c>account-recovery</c>).
    /// </summary>
    public static RateLimitPolicies Default() => new(new[]
    {
        new RateLimitPolicyOptions { Name = "feed", Limit = 60, WindowSeconds = 60 },
        new RateLimitPolicyOptions { Name = "search", Limit = 30, WindowSeconds = 60 },
        new RateLimitPolicyOptions { Name = "uploads", Limit = 5, WindowSeconds = 60 },
        new RateLimitPolicyOptions { Name = "downloads", Limit = 10, WindowSeconds = 60 },
        new RateLimitPolicyOptions { Name = "account-recovery", Limit = 3, WindowSeconds = 3600 },
    });
}
