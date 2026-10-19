namespace Platform.Caching.Redis;

/// <summary>Bounded Redis adapter settings.</summary>
public sealed class RedisCacheOptions
{
    /// <summary>Redis key namespace.</summary>
    public string KeyPrefix { get; init; } = "platform:cache:";
    /// <summary>Maximum time awaited for one provider operation.</summary>
    public TimeSpan OperationTimeout { get; init; } = TimeSpan.FromSeconds(2);
    /// <summary>Logical Redis database number.</summary>
    public int Database { get; init; }

    /// <summary>Validates settings.</summary>
    public void Validate()
    {
        if (string.IsNullOrWhiteSpace(KeyPrefix) || KeyPrefix.Any(char.IsWhiteSpace)) throw new ArgumentException("Key prefixes must be non-empty and whitespace-free.", nameof(KeyPrefix));
        if (OperationTimeout <= TimeSpan.Zero || OperationTimeout > TimeSpan.FromMinutes(1)) throw new ArgumentOutOfRangeException(nameof(OperationTimeout));
        if (Database is < 0 or > MaxDatabase) throw new ArgumentOutOfRangeException(nameof(Database));
    }

    private const int MaxDatabase = 15;
}
