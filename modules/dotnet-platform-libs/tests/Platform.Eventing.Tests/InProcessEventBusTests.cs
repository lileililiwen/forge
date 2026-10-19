using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging.Abstractions;
using Platform.Core.Time;
using Platform.Eventing.DependencyInjection;

namespace Platform.Eventing.Tests;

public class InProcessEventBusTests
{
    [Fact]
    public async Task Publish_dispatches_to_typed_handler()
    {
        var services = BuildServices();
        var handler = new RecordingOrderPlacedHandler();
        services.AddSingleton<IIntegrationEventHandler<OrderPlaced>>(handler);

        await using var provider = services.BuildServiceProvider();
        var bus = provider.GetRequiredService<IEventBus>();
        var serializer = provider.GetRequiredService<IIntegrationEventEnvelopeSerializer>();
        var clock = provider.GetRequiredService<IClock>();
        var orderPlaced = new OrderPlaced(clock.UtcNow, Guid.NewGuid(), 12.34m);
        var envelope = serializer.Serialize(orderPlaced);

        await bus.PublishAsync(envelope);

        await WaitForHandlerAsync(handler, expected: 1);
        var (received, receivedEnvelope) = Assert.Single(handler.Received);
        Assert.Equal(orderPlaced.OrderId, received.OrderId);
        Assert.Equal(envelope.PayloadType, receivedEnvelope.PayloadType);
    }

    [Fact]
    public async Task Consumer_failure_does_not_crash_the_bus()
    {
        var services = BuildServices();
        services.AddSingleton<IIntegrationEventHandler<OrderPlaced>>(new ThrowingHandler());

        await using var provider = services.BuildServiceProvider();
        var bus = provider.GetRequiredService<IEventBus>();
        var serializer = provider.GetRequiredService<IIntegrationEventEnvelopeSerializer>();
        var clock = provider.GetRequiredService<IClock>();
        var envelope = serializer.Serialize(new OrderPlaced(clock.UtcNow, Guid.NewGuid(), 1m));

        await bus.PublishAsync(envelope);

        // Give the bus time to attempt the failed dispatch without throwing.
        await Task.Delay(50);

        // Re-publish to confirm the bus is still alive.
        var secondEnvelope = serializer.Serialize(new OrderPlaced(clock.UtcNow, Guid.NewGuid(), 2m));
        await bus.PublishAsync(secondEnvelope);
    }

    [Fact]
    public async Task Publish_rejects_null_envelope()
    {
        var services = BuildServices();
        await using var provider = services.BuildServiceProvider();
        var bus = provider.GetRequiredService<IEventBus>();

        await Assert.ThrowsAsync<ArgumentNullException>(
            async () => await bus.PublishAsync(null!));
    }

    [Fact]
    public async Task DisposeAsync_is_idempotent()
    {
        var services = BuildServices();
        await using var provider = services.BuildServiceProvider();
        var bus = (InProcessEventBus)provider.GetRequiredService<IEventBus>();

        await bus.DisposeAsync();
        await bus.DisposeAsync();
    }

    [Fact]
    public void Ctor_rejects_non_positive_bounded_capacity()
    {
        var clock = new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        var deserializer = new IntegrationEventEnvelopeDeserializer(clock);
        var services = new ServiceCollection().BuildServiceProvider();

        Assert.Throws<ArgumentOutOfRangeException>(
            () => new InProcessEventBus(
                clock,
                deserializer,
                services,
                NullLogger<InProcessEventBus>.Instance,
                boundedCapacity: 0));
    }

    private static ServiceCollection BuildServices()
    {
        var services = new ServiceCollection();
        services.AddLogging();
        services.AddPlatformEventing();
        services.AddPlatformEventingInProcess();
        return services;
    }

    private static async Task WaitForHandlerAsync(
        RecordingOrderPlacedHandler handler,
        int expected,
        int timeoutMs = 2000)
    {
        var deadline = Environment.TickCount + timeoutMs;
        while (Environment.TickCount < deadline)
        {
            if (handler.Received.Count >= expected)
            {
                return;
            }
            await Task.Delay(10);
        }
    }

    private sealed class ThrowingHandler : IIntegrationEventHandler<OrderPlaced>
    {
        public string ConsumerName => "throwing";

        public Task HandleAsync(OrderPlaced payload, IntegrationEventEnvelope envelope, CancellationToken cancellationToken = default)
        {
            throw new InvalidOperationException("boom");
        }
    }
}
