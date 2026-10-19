using Microsoft.Extensions.Options;
using Platform.Eventing.Contracts;
using Platform.Eventing.RabbitMq;

namespace Platform.Eventing.RabbitMq.Tests;

internal sealed class FakeRabbitMqChannel : IRabbitMqChannel
{
    public Func<RabbitMqOutboundMessage, CancellationToken, Task>? OnPublish { get; set; }
    public Func<CancellationToken, Task>? OnDeclareExchange { get; set; }
    public bool IsOpen { get; set; } = true;
    public List<RabbitMqOutboundMessage> Published { get; } = new();
    public int DeclareExchangeCalls { get; private set; }
    public int DisposeCalls { get; private set; }

    public Task DeclareExchangeAsync(CancellationToken cancellationToken = default)
    {
        DeclareExchangeCalls++;
        return OnDeclareExchange is null ? Task.CompletedTask : OnDeclareExchange(cancellationToken);
    }

    public async Task PublishAsync(RabbitMqOutboundMessage message, CancellationToken cancellationToken = default)
    {
        Published.Add(message);
        if (OnPublish is not null)
        {
            await OnPublish(message, cancellationToken);
        }
    }

    public ValueTask DisposeAsync()
    {
        DisposeCalls++;
        return ValueTask.CompletedTask;
    }
}

internal sealed class FakeRabbitMqChannelFactory : IRabbitMqChannelFactory
{
    public Func<CancellationToken, Task<IRabbitMqChannel>>? OnCreate { get; set; }
    public List<FakeRabbitMqChannel> Created { get; } = new();
    public int CreateCalls { get; private set; }

    public async Task<IRabbitMqChannel> CreateAsync(CancellationToken cancellationToken = default)
    {
        CreateCalls++;
        if (OnCreate is not null)
        {
            return await OnCreate(cancellationToken);
        }

        var channel = new FakeRabbitMqChannel();
        Created.Add(channel);
        return channel;
    }

    public ValueTask DisposeAsync() => ValueTask.CompletedTask;
}

internal static class TestOptions
{
    public static RabbitMqEventingOptions Create(Action<RabbitMqEventingOptions>? configure = null)
    {
        var options = new RabbitMqEventingOptions
        {
            Exchange = "platform.events",
        };
        configure?.Invoke(options);
        return options;
    }

    public static IOptions<RabbitMqEventingOptions> Wrap(RabbitMqEventingOptions options) => Options.Create(options);

    public static DurableEventEnvelope Envelope(
        string payloadType = "order.created",
        string payloadJson = "{\"orderId\":\"1\"}",
        string? tenantId = null,
        string? correlationId = null) =>
        new(
            "msg-1",
            payloadType,
            payloadJson,
            new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero),
            tenantId,
            correlationId);
}
