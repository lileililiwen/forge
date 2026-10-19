namespace Platform.Eventing.RabbitMq;

/// <summary>
/// Transport seam over an AMQP channel in publisher-confirm mode. Applications
/// and tests can replace the default implementation; the platform publisher
/// only depends on this contract.
/// </summary>
public interface IRabbitMqChannel : IAsyncDisposable
{
    /// <summary>Gets whether the underlying channel and its connection are open.</summary>
    bool IsOpen { get; }

    /// <summary>Declares the configured exchange when the application opted in. Never declares queues.</summary>
    Task DeclareExchangeAsync(CancellationToken cancellationToken = default);

    /// <summary>Publishes one message and completes when the broker confirms it.</summary>
    Task PublishAsync(RabbitMqOutboundMessage message, CancellationToken cancellationToken = default);
}

/// <summary>
/// Transport seam that creates confirmed channels. The default implementation
/// opens one AMQP connection per channel; applications can replace it to share
/// connections or to supply an in-process test transport.
/// </summary>
public interface IRabbitMqChannelFactory : IAsyncDisposable
{
    /// <summary>Creates an open channel in publisher-confirm mode.</summary>
    Task<IRabbitMqChannel> CreateAsync(CancellationToken cancellationToken = default);
}
