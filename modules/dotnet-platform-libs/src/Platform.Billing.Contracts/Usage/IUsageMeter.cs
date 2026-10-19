using Platform.Billing.Contracts.Identifiers;

namespace Platform.Billing.Contracts.Usage;

/// <summary>
/// Replaces a concrete storage or counting implementation. Adapters
/// may back this with a database, cache, or in-process counter; the
/// platform does not assume any specific technology.
/// </summary>
public interface IUsageMeter
{
    /// <summary>
    /// Returns the current usage of the supplied
    /// <paramref name="feature"/> for the supplied
    /// <paramref name="subject"/>.
    /// </summary>
    /// <param name="subject">The subject the meter is queried for.</param>
    /// <param name="feature">The feature to query.</param>
    /// <param name="cancellationToken">A token to cancel the operation.</param>
    /// <returns>The current usage count and optional window.</returns>
    Task<UsageCheckResult> CheckAsync(
        SubjectKey subject,
        FeatureKey feature,
        CancellationToken cancellationToken = default);

    /// <summary>
    /// Records one or more units of usage for the supplied
    /// <paramref name="feature"/>.
    /// </summary>
    /// <param name="subject">The subject the usage is recorded for.</param>
    /// <param name="feature">The feature being consumed.</param>
    /// <param name="units">The number of units to record; must be non-negative.</param>
    /// <param name="cancellationToken">A token to cancel the operation.</param>
    /// <returns>The post-record usage count and window.</returns>
    Task<UsageCheckResult> RecordAsync(
        SubjectKey subject,
        FeatureKey feature,
        long units,
        CancellationToken cancellationToken = default);
}
