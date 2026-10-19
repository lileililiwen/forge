namespace Platform.Storage.Contracts;

/// <summary>Portable storage limits and presign bounds.</summary>
public sealed class StorageOptions
{
    /// <summary>Maximum accepted upload size.</summary>
    public long MaximumObjectBytes { get; init; } = 100 * 1024 * 1024;
    /// <summary>Maximum presigned lifetime.</summary>
    public TimeSpan MaximumPresignLifetime { get; init; } = TimeSpan.FromHours(1);
    /// <summary>Maximum provider operation wait.</summary>
    public TimeSpan OperationTimeout { get; init; } = TimeSpan.FromSeconds(30);
    /// <summary>Validates limits.</summary>
    public void Validate()
    {
        if (MaximumObjectBytes <= 0) throw new ArgumentOutOfRangeException(nameof(MaximumObjectBytes));
        if (MaximumPresignLifetime <= TimeSpan.Zero || MaximumPresignLifetime > TimeSpan.FromDays(7)) throw new ArgumentOutOfRangeException(nameof(MaximumPresignLifetime));
        if (OperationTimeout <= TimeSpan.Zero || OperationTimeout > TimeSpan.FromMinutes(5)) throw new ArgumentOutOfRangeException(nameof(OperationTimeout));
    }
}
