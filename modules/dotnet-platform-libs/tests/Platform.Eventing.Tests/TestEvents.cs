using System.Text.Json.Serialization;

namespace Platform.Eventing.Tests;

public sealed record OrderPlaced : IntegrationEvent
{
    [JsonConstructor]
    public OrderPlaced(
        DateTimeOffset occurredAt,
        Guid orderId,
        decimal amount,
        string? correlationId = null)
        : base(occurredAt, correlationId)
    {
        OrderId = orderId;
        Amount = amount;
    }

    public Guid OrderId { get; init; }
    public decimal Amount { get; init; }
}

public sealed class RecordingOrderPlacedHandler : IIntegrationEventHandler<OrderPlaced>
{
    public List<(OrderPlaced Event, IntegrationEventEnvelope Envelope)> Received { get; } = new();

    public string ConsumerName => "recording-order-placed";

    public Task HandleAsync(OrderPlaced payload, IntegrationEventEnvelope envelope, CancellationToken cancellationToken = default)
    {
        Received.Add((payload, envelope));
        return Task.CompletedTask;
    }
}
