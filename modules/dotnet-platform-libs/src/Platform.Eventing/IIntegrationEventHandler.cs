namespace Platform.Eventing;

#pragma warning disable CA1711 // Intentional: "EventHandler" matches the documented type name in this style of API.

/// <summary>
/// Contract for a consumer of typed integration events. Consumers
/// expose a stable <see cref="ConsumerName"/> for audit metrics and
/// implement <see cref="HandleAsync"/> to react to dispatched events.
/// </summary>
/// <typeparam name="TEvent">The event type this consumer handles.</typeparam>
public interface IIntegrationEventHandler<TEvent>
    where TEvent : IIntegrationEvent
{
    /// <summary>
    /// Gets a stable name used in audit metrics and structured logs.
    /// </summary>
    string ConsumerName { get; }

    /// <summary>
    /// Handles the supplied <paramref name="payload"/>. The original
    /// <see cref="IntegrationEventEnvelope"/> is provided so consumers
    /// can read correlation metadata without re-serialising.
    /// </summary>
    /// <param name="payload">The deserialised event.</param>
    /// <param name="envelope">The envelope that carried the event.</param>
    /// <param name="cancellationToken">The cancellation token.</param>
    Task HandleAsync(TEvent payload, IntegrationEventEnvelope envelope, CancellationToken cancellationToken = default);
}

#pragma warning restore CA1711
