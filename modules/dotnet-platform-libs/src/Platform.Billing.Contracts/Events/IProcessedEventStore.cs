using Platform.Billing.Contracts.Events;

namespace Platform.Billing.Contracts.Events;

/// <summary>
/// Records and queries processed provider events. The platform does
/// not assume a specific storage technology; the implementation
/// owns its own persistence schema.
/// </summary>
public interface IProcessedEventStore
{
    /// <summary>
    /// Records the supplied <paramref name="processedEvent"/> and
    /// returns whether the event was a first delivery. A duplicate
    /// delivery returns <see cref="ProcessedEventDecision.Duplicate"/>
    /// and does not overwrite the original record.
    /// </summary>
    /// <param name="processedEvent">The processed event to record.</param>
    /// <param name="cancellationToken">A token to cancel the operation.</param>
    /// <returns>The idempotency decision.</returns>
    Task<ProcessedEventDecision> MarkProcessedAsync(
        ProcessedEvent processedEvent,
        CancellationToken cancellationToken = default);
}
