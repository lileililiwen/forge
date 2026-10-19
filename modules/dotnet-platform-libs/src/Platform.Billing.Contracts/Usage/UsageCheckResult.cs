using Platform.Billing.Contracts.Identifiers;

namespace Platform.Billing.Contracts.Usage;

/// <summary>
/// Outcome of a usage-meter query. The platform does not prescribe
/// the storage or counting algorithm; the meter implementation
/// returns these fields and the consumer decides what to do with
/// them.
/// </summary>
/// <param name="Feature">The feature whose usage was checked.</param>
/// <param name="Used">The current usage count inside the metered window.</param>
/// <param name="Limit">The configured limit for the window, or <c>null</c> when no limit applies.</param>
/// <param name="WindowStart">The inclusive start of the metered window, or <c>null</c> when the meter is windowless.</param>
/// <param name="WindowEnd">The exclusive end of the metered window, or <c>null</c> when the meter is windowless.</param>
public sealed record UsageCheckResult(
    FeatureKey Feature,
    long Used,
    long? Limit,
    DateTimeOffset? WindowStart,
    DateTimeOffset? WindowEnd)
{
    /// <summary>
    /// Gets a value indicating whether the usage is within the
    /// configured limit. When <see cref="Limit"/> is <c>null</c> the
    /// usage is always considered within limit.
    /// </summary>
    public bool IsWithinLimit => Limit is null || Used <= Limit.Value;
}
