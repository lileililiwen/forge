using Platform.Billing.Contracts.Identifiers;
using Platform.Billing.Contracts.Subscriptions;

namespace Platform.Billing.Contracts.Entitlements;

/// <summary>
/// Immutable entitlement snapshot consumed by feature-gating code.
/// The snapshot is the single point of truth: web and non-web
/// consumers use the same fields and never re-query the provider.
/// </summary>
/// <param name="Subject">The subject the snapshot describes.</param>
/// <param name="Tenant">An optional tenant scope; <c>null</c> when the subject is unscoped.</param>
/// <param name="Subscription">The current normalized subscription, or <c>null</c> when anonymous.</param>
/// <param name="ActiveFeatures">The set of features granted by the snapshot.</param>
/// <param name="Limits">The numeric limits granted per feature, or <c>null</c> when no limits are defined.</param>
/// <param name="CapturedAt">The UTC time at which the snapshot was produced.</param>
public sealed record Entitlement(
    SubjectKey Subject,
    SubjectKey? Tenant,
    Subscription? Subscription,
    IReadOnlySet<FeatureKey> ActiveFeatures,
    IReadOnlyDictionary<FeatureKey, long>? Limits,
    DateTimeOffset CapturedAt)
{
    /// <summary>
    /// Gets a value indicating whether the snapshot grants the
    /// supplied <paramref name="feature"/>.
    /// </summary>
    /// <param name="feature">The feature key to check.</param>
    public bool Grants(FeatureKey feature) => ActiveFeatures.Contains(feature);

    /// <summary>
    /// Returns the limit configured for the supplied
    /// <paramref name="feature"/>, or <c>null</c> when no limit is
    /// configured.
    /// </summary>
    /// <param name="feature">The feature key to look up.</param>
    public long? LimitFor(FeatureKey feature) =>
        Limits is not null && Limits.TryGetValue(feature, out var value) ? value : null;
}
