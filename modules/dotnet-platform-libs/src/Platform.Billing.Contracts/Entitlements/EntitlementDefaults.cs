using Platform.Billing.Contracts.Identifiers;
using Platform.Billing.Contracts.Subscriptions;

namespace Platform.Billing.Contracts.Entitlements;

/// <summary>
/// Factory helpers that produce the safe defaults feature-gating code
/// can rely on. Adapters use these when no information is available;
/// feature checks treat the resulting snapshot as non-active.
/// </summary>
public static class EntitlementDefaults
{
    private static readonly IReadOnlySet<FeatureKey> NoFeatures = new HashSet<FeatureKey>();

    /// <summary>
    /// Returns an inactive entitlement for an anonymous subject. The
    /// snapshot has no subscription, no features, and no limits.
    /// </summary>
    /// <param name="capturedAt">The UTC time at which the snapshot was produced.</param>
    public static Entitlement Anonymous(DateTimeOffset capturedAt) =>
        new(SubjectKey.Anonymous, Tenant: null, Subscription: null, NoFeatures, Limits: null, capturedAt);

    /// <summary>
    /// Returns an inactive entitlement for a known subject that has
    /// no current subscription (for example, a user who has not yet
    /// subscribed or whose subscription is not yet active).
    /// </summary>
    /// <param name="subject">The subject with no subscription.</param>
    /// <param name="capturedAt">The UTC time at which the snapshot was produced.</param>
    public static Entitlement Inactive(SubjectKey subject, DateTimeOffset capturedAt) =>
        new(subject, Tenant: null, Subscription: null, NoFeatures, Limits: null, capturedAt);

    /// <summary>
    /// Returns an inactive entitlement for a known subject on a
    /// subscription that the adapter could not normalize. The
    /// subscription is preserved so the adapter can surface a
    /// diagnosable mapping result, but the snapshot is not active.
    /// </summary>
    /// <param name="subject">The subject the snapshot describes.</param>
    /// <param name="subscription">The raw subscription carrying an unrecognized status.</param>
    /// <param name="capturedAt">The UTC time at which the snapshot was produced.</param>
    public static Entitlement Unknown(SubjectKey subject, Subscription subscription, DateTimeOffset capturedAt) =>
        new(subject, Tenant: null, subscription, NoFeatures, Limits: null, capturedAt);
}
