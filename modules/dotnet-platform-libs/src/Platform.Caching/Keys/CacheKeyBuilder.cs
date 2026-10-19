namespace Platform.Caching.Keys;

/// <summary>Builds explicit application- and tenant-scoped cache keys.</summary>
public sealed class CacheKeyBuilder
{
    private readonly string _application;

    /// <summary>Creates a builder for an application namespace.</summary>
    public CacheKeyBuilder(string application)
    {
        ValidateSegment(application, nameof(application));
        _application = application;
    }

    /// <summary>Builds an application-scoped key.</summary>
    public CacheKey ForApplication(string logicalKey) => Build($"{_application}:a:{logicalKey}", logicalKey);

    /// <summary>Builds a tenant-scoped key.</summary>
    public CacheKey ForTenant(string tenantId, string logicalKey)
    {
        ValidateSegment(tenantId, nameof(tenantId));
        return Build($"{_application}:t:{tenantId}:{logicalKey}", logicalKey);
    }

    private static CacheKey Build(string value, string logicalKey)
    {
        if (string.IsNullOrWhiteSpace(logicalKey) || logicalKey.Any(char.IsWhiteSpace)) throw new ArgumentException("Logical keys must be non-empty and whitespace-free.", nameof(logicalKey));
        return new CacheKey(value);
    }

    private static void ValidateSegment(string value, string name)
    {
        if (string.IsNullOrWhiteSpace(value) || value.Any(char.IsWhiteSpace) || value.Contains(':')) throw new ArgumentException("Key segments must be non-empty, whitespace-free, and must not contain ':'.", name);
    }
}
