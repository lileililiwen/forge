using Platform.Billing.Contracts.Identifiers;

namespace Platform.Billing.Contracts.Events;

/// <summary>
/// Provider-neutral description of a webhook event. Adapters
/// translate provider payloads into this shape before invoking the
/// idempotency store or applying state changes.
/// </summary>
/// <param name="Id">The opaque provider event identifier.</param>
/// <param name="Provider">The opaque provider name.</param>
/// <param name="Type">The application-defined event type, such as <c>subscription.updated</c>.</param>
/// <param name="OccurredAt">The UTC time the provider recorded for the event.</param>
/// <param name="Payload">The opaque provider-specific payload. The platform does not inspect the contents.</param>
/// <param name="Subject">The normalized subject when the provider event identifies one.</param>
public sealed record ProviderEvent(
    ProviderEventId Id,
    ProviderName Provider,
    string Type,
    DateTimeOffset OccurredAt,
    string Payload,
    SubjectKey? Subject = null);
