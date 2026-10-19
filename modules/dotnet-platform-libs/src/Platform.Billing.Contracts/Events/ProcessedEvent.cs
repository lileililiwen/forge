using Platform.Billing.Contracts.Identifiers;

namespace Platform.Billing.Contracts.Events;

/// <summary>
/// Stored record of a processed provider event. The
/// <see cref="IProcessedEventStore"/> uses this to enforce
/// idempotency for redelivered events.
/// </summary>
/// <param name="EventId">The opaque provider event identifier.</param>
/// <param name="Provider">The opaque provider name.</param>
/// <param name="ProcessedAt">The UTC time the adapter recorded the processing.</param>
/// <param name="Result">The application-defined processing result, such as <c>applied</c> or <c>ignored</c>.</param>
public sealed record ProcessedEvent(
    ProviderEventId EventId,
    ProviderName Provider,
    DateTimeOffset ProcessedAt,
    string Result);
