namespace Platform.Caching.Contracts;

/// <summary>Portable cache entry expiration and tag settings.</summary>
public sealed class CacheEntryOptions
{
    /// <summary>Default absolute lifetime for an entry.</summary>
    public TimeSpan AbsoluteExpirationRelativeToNow { get; init; } = TimeSpan.FromHours(1);

    /// <summary>Portable tags used for grouped invalidation.</summary>
    public IReadOnlyCollection<string> Tags { get; init; } = Array.Empty<string>();

    /// <summary>Validates portable limits and tag syntax.</summary>
    public void Validate()
    {
        if (AbsoluteExpirationRelativeToNow <= TimeSpan.Zero) throw new ArgumentOutOfRangeException(nameof(AbsoluteExpirationRelativeToNow));
        if (AbsoluteExpirationRelativeToNow > TimeSpan.FromDays(365)) throw new ArgumentOutOfRangeException(nameof(AbsoluteExpirationRelativeToNow));
        if (Tags is null) throw new ArgumentNullException(nameof(Tags));
        if (Tags.Count > 16) throw new ArgumentOutOfRangeException(nameof(Tags));
        foreach (var tag in Tags)
        {
            if (string.IsNullOrWhiteSpace(tag) || tag.Any(char.IsWhiteSpace) || tag.Length > 128)
                throw new ArgumentException("Tags must be non-empty, whitespace-free, and at most 128 characters.", nameof(Tags));
        }
    }
}
