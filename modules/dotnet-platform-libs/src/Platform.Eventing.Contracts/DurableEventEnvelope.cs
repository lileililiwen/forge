namespace Platform.Eventing.Contracts;

/// <summary>Serialized event data that can be persisted and dispatched later.</summary>
public sealed record DurableEventEnvelope
{
    /// <summary>Creates a durable event envelope.</summary>
    public DurableEventEnvelope(
        string messageId,
        string payloadType,
        string payloadJson,
        DateTimeOffset occurredAt,
        string? tenantId = null,
        string? correlationId = null)
    {
        MessageId = Require(messageId, nameof(messageId));
        PayloadType = Require(payloadType, nameof(payloadType));
        PayloadJson = Require(payloadJson, nameof(payloadJson));
        if (tenantId is not null && HasControlCharacters(tenantId))
            throw new ArgumentException("Tenant id must not contain control characters.", nameof(tenantId));
        if (correlationId is not null && HasControlCharacters(correlationId))
            throw new ArgumentException("Correlation id must not contain control characters.", nameof(correlationId));
        OccurredAt = occurredAt;
        TenantId = tenantId;
        CorrelationId = correlationId;
    }

    /// <summary>Gets the stable message identifier.</summary>
    public string MessageId { get; }

    /// <summary>Gets the serialized payload type.</summary>
    public string PayloadType { get; }

    /// <summary>Gets the serialized payload.</summary>
    public string PayloadJson { get; }

    /// <summary>Gets the event occurrence time.</summary>
    public DateTimeOffset OccurredAt { get; }

    /// <summary>Gets the optional tenant identifier.</summary>
    public string? TenantId { get; }

    /// <summary>Gets the optional correlation identifier.</summary>
    public string? CorrelationId { get; }

    private static string Require(string value, string parameterName)
    {
        if (string.IsNullOrWhiteSpace(value) || HasControlCharacters(value))
            throw new ArgumentException("Value is required and must not contain control characters.", parameterName);
        return value;
    }

    private static bool HasControlCharacters(string value) => value.Any(char.IsControl);
}
