namespace Platform.Eventing;

/// <summary>
/// Contract for publishing <see cref="IntegrationEventEnvelope"/>
/// instances to the bus. Implementations live in the consumer; the
/// platform ships an in-process default
/// (<see cref="InProcessEventBus"/>).
/// </summary>
public interface IEventBus
{
    /// <summary>
    /// Publishes the supplied <paramref name="envelope"/>. Publishers
    /// MUST honour the <paramref name="cancellationToken"/>; the bus
    /// MAY back-pressure the publisher when its bounded channel is
    /// full.
    /// </summary>
    /// <param name="envelope">The envelope to publish.</param>
    /// <param name="cancellationToken">The cancellation token.</param>
    /// <exception cref="ArgumentNullException"><paramref name="envelope"/> is <c>null</c>.</exception>
    Task PublishAsync(IntegrationEventEnvelope envelope, CancellationToken cancellationToken = default);
}
