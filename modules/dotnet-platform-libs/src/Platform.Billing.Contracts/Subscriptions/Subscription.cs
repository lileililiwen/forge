using Platform.Billing.Contracts.Identifiers;

namespace Platform.Billing.Contracts.Subscriptions;

/// <summary>
/// Normalized subscription snapshot. Adapters translate provider
/// subscriptions into this shape; the platform never inspects the
/// provider SDK types.
/// </summary>
/// <param name="Subject">The subject the subscription belongs to.</param>
/// <param name="Plan">The opaque plan identifier.</param>
/// <param name="Status">The normalized subscription state.</param>
/// <param name="Provider">The opaque provider name.</param>
/// <param name="ProviderSubscriptionId">The opaque provider subscription identifier.</param>
/// <param name="PeriodStart">The inclusive start of the current period in UTC.</param>
/// <param name="PeriodEnd">The exclusive end of the current period in UTC.</param>
public sealed record Subscription(
    SubjectKey Subject,
    PlanId Plan,
    SubscriptionStatus Status,
    ProviderName Provider,
    string ProviderSubscriptionId,
    DateTimeOffset PeriodStart,
    DateTimeOffset PeriodEnd)
{
    /// <summary>
    /// Gets a value indicating whether the subscription currently
    /// entitles the subject to access. <see cref="SubscriptionStatus.Free"/>
    /// and <see cref="SubscriptionStatus.Active"/> are the only
    /// treating-as-active states.
    /// </summary>
    public bool IsActive => Status is SubscriptionStatus.Free or SubscriptionStatus.Active;

    /// <summary>
    /// Returns <c>true</c> when <paramref name="now"/> falls inside
    /// the current period (inclusive of <see cref="PeriodStart"/>,
    /// exclusive of <see cref="PeriodEnd"/>).
    /// </summary>
    /// <param name="now">The instant to test, in UTC.</param>
    public bool IsWithinPeriod(DateTimeOffset now) =>
        now >= PeriodStart && now < PeriodEnd;
}
