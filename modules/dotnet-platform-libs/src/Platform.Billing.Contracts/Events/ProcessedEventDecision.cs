namespace Platform.Billing.Contracts.Events;

/// <summary>
/// Decision returned by <see cref="IProcessedEventStore.MarkProcessedAsync"/>.
/// Adapters use the decision to either apply state changes (first
/// delivery) or skip them (redelivery).
/// </summary>
public enum ProcessedEventDecision
{
    /// <summary>
    /// The event was not previously recorded and the adapter may
    /// apply its state changes.
    /// </summary>
    FirstDelivery = 0,

    /// <summary>
    /// The event was already recorded. The adapter should acknowledge
    /// the delivery without reapplying state changes.
    /// </summary>
    Duplicate = 1,
}
