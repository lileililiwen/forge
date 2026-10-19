namespace Platform.Eventing.RabbitMq;

/// <summary>A normalized AMQP publish request built from a durable envelope.</summary>
/// <param name="Exchange">The target exchange.</param>
/// <param name="RoutingKey">The resolved routing key.</param>
/// <param name="Body">The UTF-8 encoded payload.</param>
/// <param name="MessageId">The stable durable message identifier.</param>
/// <param name="PayloadType">The durable payload type.</param>
/// <param name="OccurredAt">The event occurrence time.</param>
/// <param name="CorrelationId">The optional correlation identifier.</param>
/// <param name="TenantId">The optional tenant identifier.</param>
public sealed record RabbitMqOutboundMessage(
    string Exchange,
    string RoutingKey,
    ReadOnlyMemory<byte> Body,
    string MessageId,
    string PayloadType,
    DateTimeOffset OccurredAt,
    string? CorrelationId = null,
    string? TenantId = null);
