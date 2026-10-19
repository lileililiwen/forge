namespace Platform.Eventing;

/// <summary>
/// Contract for deserialising an <see cref="IntegrationEventEnvelope"/>
/// into a typed <see cref="IIntegrationEvent"/>. The bus resolves the
/// payload type from <see cref="IntegrationEventEnvelope.PayloadType"/>
/// and dispatches the deserialised instance to the matching consumer.
/// </summary>
public interface IIntegrationEventEnvelopeDeserializer
{
    /// <summary>
    /// Deserialises the supplied <paramref name="envelope"/> into the
    /// typed event identified by
    /// <see cref="IntegrationEventEnvelope.PayloadType"/>.
    /// </summary>
    /// <param name="envelope">The envelope to deserialise.</param>
    /// <param name="cancellationToken">The cancellation token.</param>
    /// <returns>The deserialised typed event.</returns>
    /// <exception cref="ArgumentNullException"><paramref name="envelope"/> is <c>null</c>.</exception>
    /// <exception cref="InvalidOperationException">The payload type cannot be resolved or deserialised.</exception>
    IIntegrationEvent Deserialize(IntegrationEventEnvelope envelope, CancellationToken cancellationToken = default);
}
