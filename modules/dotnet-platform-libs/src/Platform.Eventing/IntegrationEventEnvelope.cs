namespace Platform.Eventing;

/// <summary>
/// Transport-agnostic wire format for an integration event. The
/// envelope carries the <see cref="PayloadJson"/> string, the
/// <see cref="PayloadType"/> assembly-qualified name, and the
/// correlation metadata. Adapters convert the envelope to and from
/// their transport-specific shapes without inspecting the payload.
/// </summary>
/// <param name="MessageId">The transport-assigned message identifier. MUST be non-empty.</param>
/// <param name="PayloadType">The assembly-qualified type name of the payload.</param>
/// <param name="PayloadJson">The serialised JSON payload.</param>
/// <param name="OccurredAt">The UTC time the event occurred.</param>
/// <param name="CorrelationId">The optional correlation identifier.</param>
public sealed record IntegrationEventEnvelope(
    string MessageId,
    string PayloadType,
    string PayloadJson,
    DateTimeOffset OccurredAt,
    string? CorrelationId = null);
