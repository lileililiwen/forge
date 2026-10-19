using Platform.Billing.Contracts.Identifiers;

namespace Platform.Billing.Contracts.Features;

/// <summary>
/// Structured decision returned by a feature check. The decision
/// describes whether the feature is allowed, the stable reason, and
/// optional context (required plan, current usage, limit).
/// </summary>
/// <param name="Feature">The feature that was checked.</param>
/// <param name="Reason">The stable decision reason.</param>
/// <param name="RequiredPlan">The plan required to unlock the feature, when known.</param>
/// <param name="CurrentUsage">The current usage recorded for the feature, when known.</param>
/// <param name="Limit">The configured limit for the feature, when known.</param>
public sealed record FeatureCheckResult(
    FeatureKey Feature,
    FeatureCheckReason Reason,
    PlanId? RequiredPlan = null,
    long? CurrentUsage = null,
    long? Limit = null)
{
    /// <summary>
    /// Gets a value indicating whether the feature is allowed.
    /// Equivalent to <c>Reason == FeatureCheckReason.Allowed</c>.
    /// </summary>
    public bool IsAllowed => Reason == FeatureCheckReason.Allowed;
}
