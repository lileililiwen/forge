namespace Platform.Eventing.Tests;

public class IntegrationEventEnvelopeTests
{
    [Fact]
    public void Ctor_stores_documented_fields()
    {
        var envelope = new IntegrationEventEnvelope(
            MessageId: "msg-1",
            PayloadType: "Type, Assembly",
            PayloadJson: "{}",
            OccurredAt: new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero),
            CorrelationId: "corr-1");

        Assert.Equal("msg-1", envelope.MessageId);
        Assert.Equal("Type, Assembly", envelope.PayloadType);
        Assert.Equal("{}", envelope.PayloadJson);
        Assert.Equal(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero), envelope.OccurredAt);
        Assert.Equal("corr-1", envelope.CorrelationId);
    }

    [Fact]
    public void Ctor_allows_null_correlation_id()
    {
        var envelope = new IntegrationEventEnvelope(
            MessageId: "msg-1",
            PayloadType: "Type, Assembly",
            PayloadJson: "{}",
            OccurredAt: new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));

        Assert.Null(envelope.CorrelationId);
    }
}
