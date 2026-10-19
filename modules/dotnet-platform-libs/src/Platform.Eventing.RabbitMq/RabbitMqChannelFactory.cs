using RabbitMQ.Client;

namespace Platform.Eventing.RabbitMq;

/// <summary>Default <see cref="IRabbitMqChannelFactory"/> over the RabbitMQ client.</summary>
public sealed class RabbitMqChannelFactory : IRabbitMqChannelFactory
{
    private readonly RabbitMqEventingOptions _options;

    /// <summary>Creates the factory from validated options.</summary>
    /// <param name="options">The adapter options.</param>
    /// <exception cref="ArgumentNullException"><paramref name="options"/> is <c>null</c>.</exception>
    /// <exception cref="ArgumentException">The options are invalid.</exception>
    public RabbitMqChannelFactory(RabbitMqEventingOptions options)
    {
        ArgumentNullException.ThrowIfNull(options);
        options.Validate();
        _options = options;
    }

    /// <inheritdoc />
    public async Task<IRabbitMqChannel> CreateAsync(CancellationToken cancellationToken = default)
    {
        var factory = new ConnectionFactory
        {
            HostName = _options.HostName,
            Port = _options.Port,
            VirtualHost = _options.VirtualHost,
            UserName = _options.UserName,
            Password = _options.Password,
        };

        if (_options.UseSsl)
        {
            factory.Ssl = new SslOption { Enabled = true };
        }

        using var connectCts = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        connectCts.CancelAfter(_options.ConnectTimeout);
        var connection = await factory.CreateConnectionAsync(connectCts.Token).ConfigureAwait(false);
        try
        {
            var channel = await connection.CreateChannelAsync(
                new CreateChannelOptions(
                    publisherConfirmationsEnabled: true,
                    publisherConfirmationTrackingEnabled: true),
                connectCts.Token).ConfigureAwait(false);
            return new RabbitMqChannel(connection, channel, _options);
        }
        catch (Exception)
        {
            try
            {
                await connection.DisposeAsync().ConfigureAwait(false);
            }
            catch (Exception)
            {
            }

            throw;
        }
    }

    /// <inheritdoc />
    public ValueTask DisposeAsync() => ValueTask.CompletedTask;
}
