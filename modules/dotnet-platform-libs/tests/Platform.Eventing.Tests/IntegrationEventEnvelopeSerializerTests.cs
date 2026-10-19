using Platform.Core.Time;

namespace Platform.Eventing.Tests;

public class IntegrationEventEnvelopeSerializerTests
{
    [Fact]
    public void Serialize_writes_payload_type_assembly_qualified_name()
    {
        var clock = new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        var serializer = new IntegrationEventEnvelopeSerializer(clock);
        var occurredAt = clock.UtcNow;
        var orderPlaced = new OrderPlaced(occurredAt, orderId: Guid.NewGuid(), amount: 9.99m);

        var envelope = serializer.Serialize(orderPlaced);

        Assert.Equal(typeof(OrderPlaced).AssemblyQualifiedName, envelope.PayloadType);
    }

    [Fact]
    public void Serialize_uses_event_OccurredAt_when_set()
    {
        var clock = new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        var serializer = new IntegrationEventEnvelopeSerializer(clock);
        var occurredAt = new DateTimeOffset(2025, 5, 5, 5, 5, 5, TimeSpan.Zero);
        var orderPlaced = new OrderPlaced(occurredAt, orderId: Guid.NewGuid(), amount: 9.99m);

        var envelope = serializer.Serialize(orderPlaced);

        Assert.Equal(occurredAt, envelope.OccurredAt);
    }

    [Fact]
    public void Serialize_uses_event_CorrelationId_when_set()
    {
        var clock = new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        var serializer = new IntegrationEventEnvelopeSerializer(clock);
        var orderPlaced = new OrderPlaced(
            clock.UtcNow,
            orderId: Guid.NewGuid(),
            amount: 9.99m,
            correlationId: "corr-1");

        var envelope = serializer.Serialize(orderPlaced);

        Assert.Equal("corr-1", envelope.CorrelationId);
    }

    [Fact]
    public void Serialize_rejects_null_event()
    {
        var clock = new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        var serializer = new IntegrationEventEnvelopeSerializer(clock);

        Assert.Throws<ArgumentNullException>(() => serializer.Serialize(null!));
    }

    [Fact]
    public void Serialize_rejects_null_clock()
    {
        Assert.Throws<ArgumentNullException>(
            () => new IntegrationEventEnvelopeSerializer(null!));
    }
}
