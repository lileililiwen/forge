using RabbitMQ.Client;

namespace Platform.Eventing.RabbitMq;

/// <summary>Default <see cref="IRabbitMqChannel"/> over the RabbitMQ client.</summary>
public sealed class RabbitMqChannel : IRabbitMqChannel
{
    private readonly IConnection _connection;
    private readonly IChannel _channel;
    private readonly RabbitMqEventingOptions _options;

    /// <summary>Creates the channel over an open connection and channel pair.</summary>
    /// <param name="connection">The owning AMQP connection.</param>
    /// <param name="channel">The AMQP channel in publisher-confirm mode.</param>
    /// <param name="options">The validated adapter options.</param>
    public RabbitMqChannel(IConnection connection, IChannel channel, RabbitMqEventingOptions options)
    {
        _connection = connection ?? throw new ArgumentNullException(nameof(connection));
        _channel = channel ?? throw new ArgumentNullException(nameof(channel));
        _options = options ?? throw new ArgumentNullException(nameof(options));
    }

    /// <inheritdoc />
    public bool IsOpen => _channel.IsOpen && _connection.IsOpen;

    /// <inheritdoc />
    public async Task DeclareExchangeAsync(CancellationToken cancellationToken = default)
    {
        await _channel.ExchangeDeclareAsync(
            exchange: _options.Exchange,
            type: _options.ExchangeType,
            durable: _options.DurableExchange,
            autoDelete: false,
            cancellationToken: cancellationToken).ConfigureAwait(false);
    }

    /// <inheritdoc />
    public async Task PublishAsync(RabbitMqOutboundMessage message, CancellationToken cancellationToken = default)
    {
        var properties = new BasicProperties
        {
            MessageId = message.MessageId,
            ContentType = "application/json",
            DeliveryMode = DeliveryModes.Persistent,
            CorrelationId = message.CorrelationId,
            Timestamp = new AmqpTimestamp(message.OccurredAt.ToUnixTimeSeconds()),
            Headers = new Dictionary<string, object?>
            {
                ["payload-type"] = message.PayloadType,
                ["tenant-id"] = message.TenantId,
            },
        };

        await _channel.BasicPublishAsync(
            exchange: message.Exchange,
            routingKey: message.RoutingKey,
            mandatory: false,
            basicProperties: properties,
            body: message.Body,
            cancellationToken: cancellationToken).ConfigureAwait(false);
    }

    /// <inheritdoc />
    public async ValueTask DisposeAsync()
    {
        try
        {
            await _channel.CloseAsync().ConfigureAwait(false);
        }
        catch (Exception)
        {
        }

        try
        {
            await _connection.CloseAsync().ConfigureAwait(false);
        }
        catch (Exception)
        {
        }

        try
        {
            await _channel.DisposeAsync().ConfigureAwait(false);
        }
        catch (Exception)
        {
        }

        try
        {
            await _connection.DisposeAsync().ConfigureAwait(false);
        }
        catch (Exception)
        {
        }
    }
}
