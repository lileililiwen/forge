namespace Platform.Billing.Contracts.Features;

/// <summary>
/// Stable, machine-readable reason a feature check produced. Web and
/// non-web consumers translate the reason into their own UX without
/// re-inspecting the snapshot.
/// </summary>
public enum FeatureCheckReason
{
    /// <summary>
    /// The feature is allowed.
    /// </summary>
    Allowed = 0,

    /// <summary>
    /// The caller is not authenticated. Equivalent to the snapshot
    /// being produced for <c>SubjectKey.Anonymous</c>.
    /// </summary>
    NotAuthenticated = 1,

    /// <summary>
    /// The subject has no active subscription.
    /// </summary>
    NotSubscribed = 2,

    /// <summary>
    /// The subject's plan does not include the feature and a
    /// <c>RequiredPlan</c> is communicated to the consumer.
    /// </summary>
    PlanMismatch = 3,

    /// <summary>
    /// The subject's current usage exceeds the configured limit for
    /// the feature. <c>Usage</c> information is communicated to the
    /// consumer.
    /// </summary>
    LimitExceeded = 4,

    /// <summary>The subscription period has expired.</summary>
    Expired = 5,

    /// <summary>
    /// The entitlement state could not be determined. Consumers
    /// should treat the result as deny and may surface a
    /// <c>RequiredPlan</c> or diagnostic metadata.
    /// </summary>
    Unknown = 99,
}
