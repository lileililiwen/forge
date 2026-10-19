namespace Platform.Eventing.Contracts;

/// <summary>Publishes a durable envelope through an application-selected transport.</summary>
public interface IDurableEventPublisher
{
    /// <summary>Publishes an envelope and completes when the transport accepts it.</summary>
    Task PublishAsync(DurableEventEnvelope envelope, CancellationToken cancellationToken = default);
}
