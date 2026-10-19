using System.Threading.Channels;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging;
using Platform.Core.Time;

namespace Platform.Eventing;

/// <summary>
/// Default in-process implementation of <see cref="IEventBus"/>. The
/// bus serialises events into an unbounded pair of
/// <see cref="Channel{T}"/> readers (one for typed handlers, one
/// resolver) and dispatches each envelope to every registered
/// <see cref="IIntegrationEventHandler{TEvent}"/> whose payload type
/// matches <see cref="IntegrationEventEnvelope.PayloadType"/>. A
/// consumer failure is logged and swallowed so the bus never crashes.
/// </summary>
public sealed class InProcessEventBus : IEventBus, IAsyncDisposable
{
    private readonly IClock _clock;
    private readonly IIntegrationEventEnvelopeDeserializer _deserializer;
    private readonly IServiceProvider _services;
    private readonly ILogger<InProcessEventBus> _logger;
    private readonly Channel<IntegrationEventEnvelope> _channel;
    private readonly Task _reader;
    private readonly CancellationTokenSource _shutdown = new();
    private bool _disposed;

    /// <summary>
    /// Initializes a new <see cref="InProcessEventBus"/> with the
    /// supplied dependencies and bounded capacity.
    /// </summary>
    /// <param name="clock">The platform clock.</param>
    /// <param name="deserializer">The envelope deserializer.</param>
    /// <param name="services">The service provider used to resolve handlers.</param>
    /// <param name="logger">The logger.</param>
    /// <param name="boundedCapacity">The bounded capacity. MUST be positive.</param>
    /// <exception cref="ArgumentNullException">A required dependency is <c>null</c>.</exception>
    /// <exception cref="ArgumentOutOfRangeException"><paramref name="boundedCapacity"/> is non-positive.</exception>
    public InProcessEventBus(
        IClock clock,
        IIntegrationEventEnvelopeDeserializer deserializer,
        IServiceProvider services,
        ILogger<InProcessEventBus> logger,
        int boundedCapacity)
    {
        ArgumentNullException.ThrowIfNull(clock);
        ArgumentNullException.ThrowIfNull(deserializer);
        ArgumentNullException.ThrowIfNull(services);
        ArgumentNullException.ThrowIfNull(logger);
        if (boundedCapacity <= 0)
        {
            throw new ArgumentOutOfRangeException(nameof(boundedCapacity), boundedCapacity, "Bounded capacity must be positive.");
        }

        _clock = clock;
        _deserializer = deserializer;
        _services = services;
        _logger = logger;
        _channel = Channel.CreateBounded<IntegrationEventEnvelope>(new BoundedChannelOptions(boundedCapacity)
        {
            FullMode = BoundedChannelFullMode.Wait,
            SingleReader = true,
            SingleWriter = false,
        });
        _reader = Task.Run(ReadAsync);
    }

    /// <inheritdoc />
    public async Task PublishAsync(IntegrationEventEnvelope envelope, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(envelope);

        if (!await _channel.Writer.WaitToWriteAsync(cancellationToken).ConfigureAwait(false))
        {
            return;
        }
        if (_channel.Writer.TryWrite(envelope))
        {
            return;
        }
        await _channel.Writer.WriteAsync(envelope, cancellationToken).ConfigureAwait(false);
    }

    /// <inheritdoc />
    public async ValueTask DisposeAsync()
    {
        _channel.Writer.TryComplete();
        if (!_shutdown.IsCancellationRequested)
        {
            try
            {
                _shutdown.Cancel();
            }
            catch (ObjectDisposedException)
            {
                // Already disposed.
            }
        }
        try
        {
            await _reader.ConfigureAwait(false);
        }
        catch (OperationCanceledException)
        {
            // Expected on shutdown.
        }
        if (!_disposed)
        {
            _disposed = true;
            _shutdown.Dispose();
        }
    }

    private async Task ReadAsync()
    {
        var reader = _channel.Reader;
        while (await reader.WaitToReadAsync(_shutdown.Token).ConfigureAwait(false))
        {
            while (reader.TryRead(out var envelope))
            {
                try
                {
                    await DispatchAsync(envelope, _shutdown.Token).ConfigureAwait(false);
                }
                catch (OperationCanceledException) when (_shutdown.IsCancellationRequested)
                {
                    return;
                }
#pragma warning disable CA1848 // Logger source-generator delegates are not available in the production contract; the helper extension is used here for clarity.
                catch (Exception ex)
                {
                    _logger.LogError(
                        ex,
                        "In-process bus failed to dispatch envelope {MessageId} for payload type {PayloadType}.",
                        envelope.MessageId,
                        envelope.PayloadType);
                }
#pragma warning restore CA1848
            }
        }
    }

    private async Task DispatchAsync(IntegrationEventEnvelope envelope, CancellationToken cancellationToken)
    {
        var payload = _deserializer.Deserialize(envelope, cancellationToken);
        var handlerType = typeof(IIntegrationEventHandler<>).MakeGenericType(payload.GetType());
        var handlers = _services.GetServices(handlerType);
        foreach (var handler in handlers)
        {
            var handleMethod = handlerType.GetMethod(nameof(IIntegrationEventHandler<IIntegrationEvent>.HandleAsync))
                ?? throw new InvalidOperationException(
                    $"Handler type {handlerType.FullName} does not expose HandleAsync.");
            var task = (Task)handleMethod.Invoke(handler, new object?[] { payload, envelope, cancellationToken })!;
            await task.ConfigureAwait(false);
        }
    }
}
