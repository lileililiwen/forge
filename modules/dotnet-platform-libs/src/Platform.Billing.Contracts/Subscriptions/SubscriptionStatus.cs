namespace Platform.Billing.Contracts.Subscriptions;

/// <summary>
/// Normalized subscription state used by the platform. The consuming
/// adapter is responsible for mapping provider-specific states to
/// these values; the platform never inspects the provider SDK
/// directly.
/// </summary>
public enum SubscriptionStatus
{
    /// <summary>
    /// The subject has access through a free plan or a no-cost
    /// default. The platform treats this state as active for
    /// entitlement checks.
    /// </summary>
    Free = 0,

    /// <summary>
    /// The subscription is current and within its active period.
    /// </summary>
    Active = 1,

    /// <summary>
    /// The subscription has been temporarily suspended by the
    /// application or the provider. Treated as non-active.
    /// </summary>
    Suspended = 2,

    /// <summary>
    /// The subscription is past the renewal grace period. Treated as
    /// non-active.
    /// </summary>
    PastDue = 3,

    /// <summary>
    /// The subscription has been canceled and is no longer entitled
    /// to access. Treated as non-active.
    /// </summary>
    Canceled = 4,

    /// <summary>
    /// The provider state is unrecognized or unparseable. The
    /// platform treats this as non-active; the adapter should surface
    /// a diagnosable mapping result.
    /// </summary>
    Unknown = 99,
}
