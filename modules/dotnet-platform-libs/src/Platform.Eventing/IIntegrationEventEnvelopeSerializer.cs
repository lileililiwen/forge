namespace Platform.Eventing;

#pragma warning disable CA1716 // Intentional: "event" matches the documented parameter name in this style of API.

/// <summary>
/// Contract for serialising an <see cref="IIntegrationEvent"/> into an
/// <see cref="IntegrationEventEnvelope"/>. Implementations live in
/// the consumer; the platform ships a default
/// <see cref="IntegrationEventEnvelopeSerializer"/> that consumers
/// can replace.
/// </summary>
public interface IIntegrationEventEnvelopeSerializer
{
    /// <summary>
    /// Serialises the supplied <paramref name="payload"/> into an
    /// <see cref="IntegrationEventEnvelope"/>. The implementation MUST
    /// set <see cref="IntegrationEventEnvelope.OccurredAt"/> from the
    /// platform clock when the event does not already carry one.
    /// </summary>
    /// <param name="payload">The event to serialise.</param>
    /// <param name="cancellationToken">The cancellation token.</param>
    /// <returns>The serialised envelope.</returns>
    /// <exception cref="ArgumentNullException"><paramref name="payload"/> is <c>null</c>.</exception>
    IntegrationEventEnvelope Serialize(IIntegrationEvent payload, CancellationToken cancellationToken = default);
}

#pragma warning restore CA1716