using Platform.Core.Time;

namespace Platform.Eventing.Tests;

public class IntegrationEventEnvelopeDeserializerTests
{
    [Fact]
    public void Round_trip_preserves_payload_values()
    {
        var clock = new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        var serializer = new IntegrationEventEnvelopeSerializer(clock);
        var deserializer = new IntegrationEventEnvelopeDeserializer(clock);
        var orderId = Guid.NewGuid();
        var orderPlaced = new OrderPlaced(clock.UtcNow, orderId, amount: 19.99m, correlationId: "corr-1");

        var envelope = serializer.Serialize(orderPlaced);
        var deserialised = (OrderPlaced)deserializer.Deserialize(envelope);

        Assert.Equal(orderPlaced.OrderId, deserialised.OrderId);
        Assert.Equal(orderPlaced.Amount, deserialised.Amount);
        Assert.Equal(orderPlaced.CorrelationId, deserialised.CorrelationId);
    }

    [Fact]
    public void Round_trip_preserves_envelope_OccurredAt()
    {
        var clock = new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        var serializer = new IntegrationEventEnvelopeSerializer(clock);
        var deserializer = new IntegrationEventEnvelopeDeserializer(clock);
        var orderPlaced = new OrderPlaced(clock.UtcNow, Guid.NewGuid(), 10m);

        var envelope = serializer.Serialize(orderPlaced);

        Assert.Equal(envelope.OccurredAt, envelope.OccurredAt);
        Assert.Equal(orderPlaced.OccurredAt, envelope.OccurredAt);
    }

    [Fact]
    public void Deserialize_throws_for_unknown_payload_type()
    {
        var clock = new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        var deserializer = new IntegrationEventEnvelopeDeserializer(clock);
        var envelope = new IntegrationEventEnvelope(
            MessageId: "msg-1",
            PayloadType: "NonExistent.Type, NonExistent.Assembly",
            PayloadJson: "{}",
            OccurredAt: clock.UtcNow);

        Assert.Throws<InvalidOperationException>(() => deserializer.Deserialize(envelope));
    }

    [Fact]
    public void Deserialize_rejects_null_envelope()
    {
        var clock = new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        var deserializer = new IntegrationEventEnvelopeDeserializer(clock);

        Assert.Throws<ArgumentNullException>(() => deserializer.Deserialize(null!));
    }

    [Fact]
    public void Deserialize_rejects_null_clock()
    {
        Assert.Throws<ArgumentNullException>(
            () => new IntegrationEventEnvelopeDeserializer(null!));
    }
}
