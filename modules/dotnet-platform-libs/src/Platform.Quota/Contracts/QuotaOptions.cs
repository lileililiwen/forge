namespace Platform.Quota.Contracts;

/// <summary>Quota validation and reservation settings.</summary>
public sealed class QuotaOptions
{
    /// <summary>Maximum accepted amount in one operation.</summary>
    public long MaximumAmount { get; init; } = 1_000_000_000;
    /// <summary>Maximum reservation lifetime.</summary>
    public TimeSpan MaximumReservationLifetime { get; init; } = TimeSpan.FromHours(24);
    /// <summary>Maximum stable operation key length.</summary>
    public int MaximumOperationKeyLength { get; init; } = 256;
    /// <summary>Validates options.</summary>
    public void Validate()
    {
        if (MaximumAmount <= 0) throw new ArgumentOutOfRangeException(nameof(MaximumAmount));
        if (MaximumReservationLifetime <= TimeSpan.Zero || MaximumReservationLifetime > TimeSpan.FromDays(30)) throw new ArgumentOutOfRangeException(nameof(MaximumReservationLifetime));
        if (MaximumOperationKeyLength <= 0 || MaximumOperationKeyLength > 1024) throw new ArgumentOutOfRangeException(nameof(MaximumOperationKeyLength));
    }
}
