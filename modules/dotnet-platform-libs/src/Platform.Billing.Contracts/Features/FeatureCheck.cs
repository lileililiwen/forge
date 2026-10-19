using Platform.Billing.Contracts.Entitlements;
using Platform.Billing.Contracts.Identifiers;

namespace Platform.Billing.Contracts.Features;

/// <summary>
/// Helpers that translate <see cref="Entitlement"/> snapshots into
/// <see cref="FeatureCheckResult"/> decisions. Adapters may use these
/// directly or layer their own logic on top.
/// </summary>
public static class FeatureCheck
{
    /// <summary>
    /// Evaluates the supplied <paramref name="entitlement"/> against
    /// the supplied <paramref name="feature"/>.
    /// </summary>
    /// <param name="entitlement">The current entitlement snapshot.</param>
    /// <param name="feature">The feature to check.</param>
    /// <returns>The structured decision.</returns>
    public static FeatureCheckResult Evaluate(Entitlement entitlement, FeatureKey feature)
    {
        ArgumentNullException.ThrowIfNull(entitlement);
        return Evaluate(entitlement, feature, entitlement.CapturedAt);
    }

    /// <summary>Evaluates a snapshot at an explicit instant.</summary>
    public static FeatureCheckResult Evaluate(Entitlement entitlement, FeatureKey feature, DateTimeOffset now)
    {
        ArgumentNullException.ThrowIfNull(entitlement);

        if (entitlement.Subject.IsAnonymous)
        {
            return Denied(feature, FeatureCheckReason.NotAuthenticated);
        }

        if (entitlement.Subscription is null || !entitlement.Subscription.IsActive)
        {
            return Denied(feature, FeatureCheckReason.NotSubscribed);
        }

        if (!entitlement.Subscription.IsWithinPeriod(now))
        {
            return Denied(feature, FeatureCheckReason.Expired);
        }

        return entitlement.Grants(feature)
            ? Allowed(feature)
            : Denied(feature, FeatureCheckReason.PlanMismatch);
    }

    /// <summary>
    /// Builds an allow decision for the supplied
    /// <paramref name="feature"/>.
    /// </summary>
    /// <param name="feature">The feature that was allowed.</param>
    public static FeatureCheckResult Allowed(FeatureKey feature) =>
        new(feature, FeatureCheckReason.Allowed);

    /// <summary>
    /// Builds a deny decision for the supplied <paramref name="feature"/>
    /// with the supplied <paramref name="reason"/>.
    /// </summary>
    /// <param name="feature">The feature that was denied.</param>
    /// <param name="reason">The denial reason.</param>
    /// <param name="requiredPlan">The plan required to unlock the feature, when known.</param>
    public static FeatureCheckResult Denied(
        FeatureKey feature,
        FeatureCheckReason reason,
        PlanId? requiredPlan = null) =>
        new(feature, reason, RequiredPlan: requiredPlan);
}
