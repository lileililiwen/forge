using System.Text;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;
using Platform.Eventing.Contracts;

namespace Platform.Eventing.RabbitMq;

/// <summary>
/// RabbitMQ implementation of the platform durable publisher seam. Publishes
/// envelopes through the application-registered topology with publisher
/// confirms and bounded waits; connection, confirmation, and broker failures
/// are surfaced as safe transient failures so the durable outbox stays the
/// retry owner. Unregistered payload types fail with a configuration error
/// before an ambiguous message is sent.
/// </summary>
public sealed partial class RabbitMqDurableEventPublisher : IDurableEventPublisher, IAsyncDisposable
{
    private const string TopologyUnconfiguredCode = "eventing.rabbitmq.topology_unconfigured";
    private const string UnregisteredTypeCode = "eventing.rabbitmq.unregistered_type";
    private const string InvalidBindingCode = "eventing.rabbitmq.invalid_binding";
    private const string ConnectFailedCode = "eventing.rabbitmq.connect_failed";
    private const string ConnectTimeoutCode = "eventing.rabbitmq.connect_timeout";
    private const string ConfirmTimeoutCode = "eventing.rabbitmq.confirm_timeout";
    private const string PublishFailedCode = "eventing.rabbitmq.publish_failed";

    private readonly IRabbitMqEventTopology _topology;
    private readonly IRabbitMqChannelFactory _channelFactory;
    private readonly RabbitMqEventingOptions _options;
    private readonly ILogger<RabbitMqDurableEventPublisher> _logger;
    private readonly SemaphoreSlim _channelLock = new(1, 1);
    private IRabbitMqChannel? _channel;
    private volatile RabbitMqEventingProviderStatus _status;
    private bool _disposed;

    /// <summary>Creates the publisher over an application-owned topology and channel factory.</summary>
    /// <param name="topology">The application-owned topology registration.</param>
    /// <param name="channelFactory">The transport seam.</param>
    /// <param name="options">The adapter options.</param>
    /// <param name="logger">The logger.</param>
    /// <exception cref="ArgumentNullException">An argument is <c>null</c> or the options value is missing.</exception>
    public RabbitMqDurableEventPublisher(
        IRabbitMqEventTopology topology,
        IRabbitMqChannelFactory channelFactory,
        IOptions<RabbitMqEventingOptions> options,
        ILogger<RabbitMqDurableEventPublisher> logger)
    {
        ArgumentNullException.ThrowIfNull(topology);
        ArgumentNullException.ThrowIfNull(channelFactory);
        ArgumentNullException.ThrowIfNull(options);
        ArgumentNullException.ThrowIfNull(logger);
        var value = options.Value;
        ArgumentNullException.ThrowIfNull(value);
        value.Validate();
        _topology = topology;
        _channelFactory = channelFactory;
        _options = value;
        _logger = logger;
        _status = new RabbitMqEventingProviderStatus(RabbitMqEventingProviderStatus.ProviderName, RabbitMqEventingProviderState.Healthy);
    }

    /// <summary>
    /// Gets the documented status snapshot derived from the most recent
    /// publish attempt.
    /// </summary>
    public RabbitMqEventingProviderStatus Status => _status;

    /// <inheritdoc />
    public async Task PublishAsync(DurableEventEnvelope envelope, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(envelope);

        var binding = _topology.Resolve(envelope.PayloadType);
        if (binding is null)
        {
            throw _topology is UnconfiguredRabbitMqEventTopology
                ? new RabbitMqPublishException(RabbitMqPublishFailure.Configuration(
                    TopologyUnconfiguredCode,
                    "No RabbitMQ event topology is registered."))
                : new RabbitMqPublishException(RabbitMqPublishFailure.Configuration(
                    UnregisteredTypeCode,
                    "The event payload type is not registered with the RabbitMQ topology."));
        }

        try
        {
            binding.Validate();
        }
        catch (ArgumentException)
        {
            throw new RabbitMqPublishException(RabbitMqPublishFailure.Configuration(
                InvalidBindingCode,
                "The registered RabbitMQ binding is invalid."));
        }

        var outbound = new RabbitMqOutboundMessage(
            Exchange: binding.Exchange ?? _options.Exchange,
            RoutingKey: binding.RoutingKey,
            Body: Encoding.UTF8.GetBytes(envelope.PayloadJson),
            MessageId: envelope.MessageId,
            PayloadType: envelope.PayloadType,
            OccurredAt: envelope.OccurredAt,
            CorrelationId: envelope.CorrelationId,
            TenantId: envelope.TenantId);

        var channel = await GetOpenChannelAsync(cancellationToken).ConfigureAwait(false);

        using var confirmCts = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        confirmCts.CancelAfter(_options.ConfirmTimeout);
        try
        {
            await channel.PublishAsync(outbound, confirmCts.Token).ConfigureAwait(false);
            _status = new RabbitMqEventingProviderStatus(RabbitMqEventingProviderStatus.ProviderName, RabbitMqEventingProviderState.Healthy);
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            throw;
        }
        catch (OperationCanceledException) when (confirmCts.IsCancellationRequested)
        {
            throw Transient(
                ConfirmTimeoutCode,
                "The RabbitMQ publisher confirmation timed out.");
        }
        catch (Exception exception)
        {
            LogPublishFailed(exception.GetType().Name);
            await InvalidateChannelAsync(channel).ConfigureAwait(false);
            throw Transient(
                PublishFailedCode,
                "The RabbitMQ broker did not accept the published event.");
        }
    }

    /// <inheritdoc />
    public async ValueTask DisposeAsync()
    {
        if (_disposed)
        {
            return;
        }

        _disposed = true;
        await _channelLock.WaitAsync(CancellationToken.None).ConfigureAwait(false);
        IRabbitMqChannel? channel;
        try
        {
            channel = _channel;
            _channel = null;
        }
        finally
        {
            _channelLock.Release();
        }

        if (channel is not null)
        {
            await DisposeChannelQuietlyAsync(channel).ConfigureAwait(false);
        }

        _channelLock.Dispose();
    }

    private async Task<IRabbitMqChannel> GetOpenChannelAsync(CancellationToken cancellationToken)
    {
        var existing = _channel;
        if (existing is { IsOpen: true })
        {
            return existing;
        }

        await _channelLock.WaitAsync(cancellationToken).ConfigureAwait(false);
        try
        {
            if (_channel is { IsOpen: true })
            {
                return _channel;
            }

            if (_channel is not null)
            {
                var stale = _channel;
                _channel = null;
                await DisposeChannelQuietlyAsync(stale).ConfigureAwait(false);
            }

            IRabbitMqChannel? created = null;
            using var connectCts = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
            connectCts.CancelAfter(_options.ConnectTimeout);
            try
            {
                created = await _channelFactory.CreateAsync(connectCts.Token).ConfigureAwait(false);
                if (_options.DeclareExchange)
                {
                    await created.DeclareExchangeAsync(connectCts.Token).ConfigureAwait(false);
                }
            }
            catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
            {
                await DisposeChannelQuietlyAsync(created).ConfigureAwait(false);
                throw;
            }
            catch (OperationCanceledException)
            {
                await DisposeChannelQuietlyAsync(created).ConfigureAwait(false);
                throw Transient(
                    ConnectTimeoutCode,
                    "Connecting to the RabbitMQ broker timed out.");
            }
            catch (Exception exception)
            {
                await DisposeChannelQuietlyAsync(created).ConfigureAwait(false);
                LogConnectFailed(exception.GetType().Name);
                throw Transient(
                    ConnectFailedCode,
                    "The RabbitMQ broker could not be reached.");
            }

            _channel = created;
            return created;
        }
        finally
        {
            _channelLock.Release();
        }
    }

    private RabbitMqPublishException Transient(string code, string message)
    {
        _status = new RabbitMqEventingProviderStatus(RabbitMqEventingProviderStatus.ProviderName, RabbitMqEventingProviderState.Unavailable, code);
        return new RabbitMqPublishException(RabbitMqPublishFailure.Transient(code, message));
    }

    private async Task InvalidateChannelAsync(IRabbitMqChannel channel)
    {
        await _channelLock.WaitAsync(CancellationToken.None).ConfigureAwait(false);
        try
        {
            if (!ReferenceEquals(_channel, channel))
            {
                return;
            }

            _channel = null;
        }
        finally
        {
            _channelLock.Release();
        }

        await DisposeChannelQuietlyAsync(channel).ConfigureAwait(false);
    }

    private static async Task DisposeChannelQuietlyAsync(IRabbitMqChannel? channel)
    {
        if (channel is null)
        {
            return;
        }

        try
        {
            await channel.DisposeAsync().ConfigureAwait(false);
        }
        catch (Exception)
        {
        }
    }

    [LoggerMessage(Level = LogLevel.Warning, Message = "The RabbitMQ publish failed; the transport error type was {ExceptionTypeName}.")]
    private partial void LogPublishFailed(string exceptionTypeName);

    [LoggerMessage(Level = LogLevel.Warning, Message = "The RabbitMQ connection could not be established; the transport error type was {ExceptionTypeName}.")]
    private partial void LogConnectFailed(string exceptionTypeName);
}
