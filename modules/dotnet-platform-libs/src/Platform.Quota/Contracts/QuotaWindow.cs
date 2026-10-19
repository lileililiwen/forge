namespace Platform.Quota.Contracts;

/// <summary>Explicit UTC quota accounting window.</summary>
public readonly record struct QuotaWindow(DateTimeOffset StartsAt, DateTimeOffset EndsAt)
{
    /// <summary>Validates UTC values and positive duration.</summary>
    public void Validate()
    {
        if (StartsAt.Offset != TimeSpan.Zero || EndsAt.Offset != TimeSpan.Zero) throw new ArgumentException("Quota windows must use UTC timestamps.");
        if (EndsAt <= StartsAt) throw new ArgumentOutOfRangeException(nameof(EndsAt));
        if (EndsAt - StartsAt > TimeSpan.FromDays(366)) throw new ArgumentOutOfRangeException(nameof(EndsAt));
    }
}
